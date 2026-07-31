use std::path::{Path, PathBuf};

use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("file-size");
pub struct FileSizeRule {
    max_lines: usize,
    exclude: Vec<PathBuf>,
}

impl FileSizeRule {
    pub fn new(max_lines: usize, exclude: Vec<PathBuf>) -> Self {
        Self { max_lines, exclude }
    }
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(FileSizeRule::new(
        settings.max_lines,
        settings.file_size_exclude.clone(),
    ))
}

fn instruction(settings: &Settings) -> String {
    let limit = settings.max_lines;
    if settings.file_size_exclude.is_empty() {
        return format!(
            "Files over {limit} lines are not allowed. Split oversized files along responsibility boundaries."
        );
    }
    let excluded = settings
        .file_size_exclude
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Files over {limit} lines are not allowed outside {excluded}. Split oversized implementation files along responsibility boundaries."
    )
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        repository_factory: None,
        instruction,
    }
}

impl FileRule for FileSizeRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "file exceeds the configured line budget",
            default_enabled: self.max_lines > 0,
        }
    }

    fn applies_to(&self, _: &LanguageProfile) -> bool {
        true
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
        let lines = file.text.lines().count();
        if self.max_lines == 0 || lines <= self.max_lines {
            return;
        }
        let mut finding = Finding::new(
            KEY,
            Severity::Error,
            Location::point(file.path, self.max_lines + 1, 1),
            format!("{lines} lines"),
            format!(
                "file has {lines} lines, over the {}-line limit",
                self.max_lines
            ),
        );
        finding.help = Some("split the file along a meaningful responsibility boundary".into());
        candidates.push(Candidate::file(finding));
    }
}
