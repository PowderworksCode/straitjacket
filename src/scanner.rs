use std::collections::HashSet;

use crate::config::Settings;
use crate::finding::Finding;
use crate::language::{LanguageProfile, language_profile_for_extension};
use crate::rule::{Candidate, FileRule, RuleDescriptor, SourceFile};
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

/// Whether one rule survived `only`/`skip` and is going to run.
fn rules_enabled(rules: &[RegisteredRule], key: crate::rules::RuleKey) -> bool {
    rules
        .iter()
        .any(|registered| registered.enabled && registered.rule.descriptor().id == key)
}

pub struct Scanner {
    rules: Vec<RegisteredRule>,
    include_json: bool,
    fail_on_unused_markers: bool,
}

impl Scanner {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        rules::resolve_test_rules(&settings.test_rules)?;
        let builtins = rules::builtins(settings)?;
        let only: HashSet<_> = rules::resolve(&settings.only)?.into_iter().collect();
        let skip: HashSet<_> = rules::resolve(&settings.skip)?.into_iter().collect();
        let rules: Vec<RegisteredRule> = builtins
            .into_iter()
            .map(|rule| {
                let descriptor = rule.descriptor();
                let mut enabled = if only.is_empty() {
                    descriptor.default_enabled
                        || (descriptor.id == rules::NO_COMMENTS && settings.no_comments)
                        || (descriptor.id == rules::STRAY_CONST && settings.stray_const)
                        || (descriptor.id == rules::TEST_QUALITY && settings.test_quality)
                } else {
                    only.contains(&descriptor.id)
                };
                enabled &= !skip.contains(&descriptor.id);
                RegisteredRule { rule, enabled }
            })
            .collect();

        rules::check_const_files(rules_enabled(&rules, rules::STRAY_CONST), settings)?;

        Ok(Self {
            rules,
            include_json: settings.include_json,
            fail_on_unused_markers: settings.fail_on_unused_markers
                && !skip.contains(&rules::UNUSED_MARKER),
        })
    }

    pub fn descriptors(&self) -> Vec<RuleDescriptor> {
        self.rules
            .iter()
            .map(|registered| registered.rule.descriptor())
            .chain(std::iter::once(rules::unused_marker_descriptor()))
            .collect()
    }

    pub fn enabled_descriptors(&self) -> Vec<RuleDescriptor> {
        self.rules
            .iter()
            .filter(|registered| registered.enabled)
            .map(|registered| registered.rule.descriptor())
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
        self.rules
            .iter()
            .any(|registered| registered.enabled && registered.rule.applies_to(language))
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
        self.finish_repository(vec![PendingFileScan {
            text,
            path,
            language,
            pending: &pending,
        }])
    }

    pub fn finish_repository(&self, files: Vec<PendingFileScan<'_>>) -> ScanResult {
        let mut candidates = Vec::new();
        let candidate_count = files
            .iter()
            .map(|file| file.pending.candidates.len())
            .sum::<usize>();
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
