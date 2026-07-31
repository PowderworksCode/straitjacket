use std::collections::HashSet;

use entl_codebase::{LanguageProfile, language_profile_for_extension};
use infact_analysis::{AnalysisSelection, FactBatch};
use std::path::Path;

use crate::config::Settings;
use crate::finding::Finding;
use crate::rule::{Candidate, FileRule, RepositoryRule, RuleDescriptor, SourceFile};
use crate::rules;
use crate::suppression;

#[derive(Debug, Default)]
pub struct ScanResult {
    pub findings: Vec<Finding>,
    pub candidates: usize,
    pub suppressed: usize,
}

#[derive(Debug, Default)]
pub struct PendingScan {
    pub candidates: Vec<Candidate>,
    enabled: HashSet<crate::rules::RuleKey>,
    applicable: HashSet<crate::rules::RuleKey>,
}

pub struct PendingFileScan<'a> {
    pub text: &'a str,
    pub path: &'a str,
    pub language: &'a LanguageProfile,
    pub pending: &'a PendingScan,
}

struct RegisteredRule {
    rule: Box<dyn FileRule>,
    enabled: bool,
}

struct RegisteredRepositoryRule {
    rule: Box<dyn RepositoryRule>,
    enabled: bool,
}

pub struct Scanner {
    rules: Vec<RegisteredRule>,
    repository_rules: Vec<RegisteredRepositoryRule>,
    include_json: bool,
    fail_on_unused_markers: bool,
}

impl Scanner {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        settings.validate()?;
        let builtins = rules::builtins(settings)?;
        let repository_builtins = rules::repository_builtins(settings)?;
        let only: HashSet<_> = rules::resolve(&settings.only)?.into_iter().collect();
        let skip: HashSet<_> = rules::resolve(&settings.skip)?.into_iter().collect();
        let rules = builtins
            .into_iter()
            .map(|rule| {
                let descriptor = rule.descriptor();
                let mut enabled = if only.is_empty() {
                    descriptor.default_enabled
                        || (descriptor.id == rules::NO_COMMENTS && settings.no_comments)
                } else {
                    only.contains(&descriptor.id)
                };
                enabled &= !skip.contains(&descriptor.id);
                RegisteredRule { rule, enabled }
            })
            .collect();
        let repository_rules = repository_builtins
            .into_iter()
            .map(|rule| {
                let descriptor = rule.descriptor();
                let mut enabled = if only.is_empty() {
                    descriptor.default_enabled
                } else {
                    only.contains(&descriptor.id)
                };
                enabled &= !skip.contains(&descriptor.id);
                RegisteredRepositoryRule { rule, enabled }
            })
            .collect();

        Ok(Self {
            rules,
            repository_rules,
            include_json: settings.include_json,
            fail_on_unused_markers: settings.fail_on_unused_markers
                && !skip.contains(&rules::UNUSED_MARKER),
        })
    }

    pub fn descriptors(&self) -> Vec<RuleDescriptor> {
        self.rules
            .iter()
            .map(|registered| registered.rule.descriptor())
            .chain(
                self.repository_rules
                    .iter()
                    .map(|registered| registered.rule.descriptor()),
            )
            .chain(std::iter::once(rules::unused_marker_descriptor()))
            .collect()
    }

    pub fn enabled_descriptors(&self) -> Vec<RuleDescriptor> {
        self.rules
            .iter()
            .filter(|registered| registered.enabled)
            .map(|registered| registered.rule.descriptor())
            .chain(
                self.repository_rules
                    .iter()
                    .filter(|registered| registered.enabled)
                    .map(|registered| registered.rule.descriptor()),
            )
            .chain(
                self.fail_on_unused_markers
                    .then(rules::unused_marker_descriptor),
            )
            .collect()
    }

    pub fn handles_extension(&self, extension: &str) -> bool {
        let Some(profile) = language_profile_for_extension(extension) else {
            return false;
        };
        self.handles_language(profile)
    }

    pub fn handles_language(&self, language: &LanguageProfile) -> bool {
        if language.id == "json" && !self.include_json {
            return false;
        }
        self.has_enabled_repository_rules()
            || self
                .rules
                .iter()
                .any(|registered| registered.enabled && registered.rule.applies_to(language))
    }

    pub fn has_enabled_repository_rules(&self) -> bool {
        self.repository_rules
            .iter()
            .any(|registered| registered.enabled)
    }

    pub fn analysis_selection(&self) -> AnalysisSelection {
        let mut selection = AnalysisSelection::default();
        for registered in &self.repository_rules {
            if registered.enabled {
                registered.rule.select_analysis(&mut selection);
            }
        }
        selection
    }

    pub fn repository_candidates(&self, facts: &FactBatch, display_root: &Path) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for registered in &self.repository_rules {
            if registered.enabled {
                registered.rule.check(facts, display_root, &mut candidates);
            }
        }
        candidates
    }

    pub fn scan(&self, text: &str, path: &str, extension: &str) -> ScanResult {
        let Some(profile) = language_profile_for_extension(extension) else {
            return ScanResult::default();
        };
        self.scan_language(text, path, profile)
    }

    pub fn scan_language(&self, text: &str, path: &str, language: &LanguageProfile) -> ScanResult {
        let pending = self.collect_language(text, path, language);
        self.finish_language(text, path, language, pending)
    }

    pub fn collect_language(
        &self,
        text: &str,
        path: &str,
        language: &LanguageProfile,
    ) -> PendingScan {
        if !self.handles_language(language) {
            return PendingScan::default();
        }
        let file = SourceFile {
            path,
            language,
            text,
        };
        let mut candidates = Vec::new();
        let mut enabled = HashSet::new();
        let mut applicable = HashSet::new();
        for registered in &self.rules {
            if !registered.enabled {
                continue;
            }
            let descriptor = registered.rule.descriptor();
            enabled.insert(descriptor.id);
            if registered.rule.applies_to(language) {
                applicable.insert(descriptor.id);
                registered.rule.check(file, &mut candidates);
            }
        }
        for registered in &self.repository_rules {
            if registered.enabled {
                let descriptor = registered.rule.descriptor();
                enabled.insert(descriptor.id);
                applicable.insert(descriptor.id);
            }
        }
        PendingScan {
            candidates,
            enabled,
            applicable,
        }
    }

    pub fn finish_language(
        &self,
        text: &str,
        path: &str,
        language: &LanguageProfile,
        pending: PendingScan,
    ) -> ScanResult {
        self.finish_repository(
            vec![PendingFileScan {
                text,
                path,
                language,
                pending: &pending,
            }],
            Vec::new(),
        )
    }

    pub fn finish_repository(
        &self,
        files: Vec<PendingFileScan<'_>>,
        repository_candidates: Vec<Candidate>,
    ) -> ScanResult {
        let mut candidates = repository_candidates;
        let candidate_count = files
            .iter()
            .map(|file| file.pending.candidates.len())
            .sum::<usize>()
            + candidates.len();
        let mut suppression_files = Vec::with_capacity(files.len());
        for file in &files {
            candidates.extend(file.pending.candidates.iter().cloned());
            suppression_files.push(suppression::SuppressionFile {
                text: file.text,
                path: file.path,
                language: file.language,
                enabled_rules: &file.pending.enabled,
                applicable_rules: &file.pending.applicable,
            });
        }
        let applied = suppression::apply_repository(
            suppression_files,
            candidates,
            self.fail_on_unused_markers,
        );
        ScanResult {
            findings: applied.findings,
            candidates: candidate_count,
            suppressed: applied.suppressed,
        }
    }
}
