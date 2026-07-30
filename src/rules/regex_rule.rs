use std::path::{Path, PathBuf};

use entl_codebase::{LanguageFacet, LanguageProfile};
use regex::{Captures, Regex};

use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};

pub type Judge = fn(&Captures<'_>) -> Option<String>;

pub struct RegexRule {
    descriptor: RuleDescriptor,
    facet: &'static LanguageFacet,
    regex: Regex,
    judge: Judge,
    help: &'static str,
    exclude: Vec<PathBuf>,
}

impl RegexRule {
    pub fn new(
        key: RuleKey,
        summary: &'static str,
        facet: &'static LanguageFacet,
        pattern: &str,
        judge: Judge,
        help: &'static str,
    ) -> Self {
        Self {
            descriptor: RuleDescriptor {
                id: key,
                summary,
                default_enabled: true,
            },
            facet,
            regex: Regex::new(pattern).expect("built-in rule patterns must compile"),
            judge,
            help,
            exclude: Vec::new(),
        }
    }

    pub fn excluding(mut self, paths: Vec<PathBuf>) -> Self {
        self.exclude = paths;
        self
    }
}

impl FileRule for RegexRule {
    fn descriptor(&self) -> RuleDescriptor {
        self.descriptor
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        language.has_facet(self.facet)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        let path = Path::new(file.path);
        if self.exclude.iter().any(|excluded| {
            path == excluded
                || path.starts_with(excluded)
                || excluded.is_relative() && path.is_absolute() && path.ends_with(excluded)
        }) {
            return;
        }
        for (line_index, line) in file.text.lines().enumerate() {
            for captures in self.regex.captures_iter(line) {
                let whole = captures.get(0).expect("capture zero is always present");
                let Some(matched) = (self.judge)(&captures) else {
                    continue;
                };
                let mut finding = Finding::new(
                    self.descriptor.id,
                    Severity::Error,
                    Location::point(file.path, line_index + 1, whole.start() + 1),
                    matched,
                    self.descriptor.summary,
                );
                finding.help = Some(self.help.into());
                candidates.push(Candidate::line(finding));
            }
        }
    }
}

pub fn whole(captures: &Captures<'_>) -> Option<String> {
    Some(captures[0].to_string())
}
