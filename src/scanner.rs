use std::collections::HashSet;

use entl_codebase::{LanguageProfile, language_profile_for_extension};

use crate::config::Settings;
use crate::finding::Finding;
use crate::rule::{FileRule, RuleDescriptor, SourceFile};
use crate::rules;
use crate::suppression;

#[derive(Debug, Default)]
pub struct ScanResult {
    pub findings: Vec<Finding>,
    pub candidates: usize,
    pub suppressed: usize,
}

struct RegisteredRule {
    rule: Box<dyn FileRule>,
    enabled: bool,
}

pub struct Scanner {
    rules: Vec<RegisteredRule>,
    include_json: bool,
    fail_on_unused_markers: bool,
}

impl Scanner {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        let builtins = rules::builtins(settings)?;
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
        if !self.handles_language(language) {
            return ScanResult::default();
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
        let candidate_count = candidates.len();
        let applied = suppression::apply(
            text,
            path,
            language,
            candidates,
            &enabled,
            &applicable,
            self.fail_on_unused_markers,
        );
        ScanResult {
            findings: applied.findings,
            candidates: candidate_count,
            suppressed: applied.suppressed,
        }
    }
}
