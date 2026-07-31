use std::path::Path;

use infact_analysis::{AnalysisSelection, FactBatch};

use crate::Settings;
use crate::finding::{Finding, Location, RelatedLocation, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("near-clone");

struct NearCloneRule {
    enabled: bool,
    config: infact_duplication::NearConfig,
    repository_root: std::path::PathBuf,
    exclude: Vec<std::path::PathBuf>,
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(NearCloneRule {
        enabled: settings.facts.near_clones,
        config: settings.facts.near,
        repository_root: settings.facts.repository_root.clone(),
        exclude: settings.facts.clone_exclude.clone(),
    })
}

fn instruction(settings: &Settings) -> String {
    let mut instruction = format!(
        "Extract shared behavior when Straitjacket reports a near clone of at least {} tokens across at least {} lines with no more than {}% changed tokens.",
        settings.facts.near.min_tokens,
        settings.facts.near.min_lines,
        settings.facts.near.max_changed_percent
    );
    if !settings.facts.clone_exclude.is_empty() {
        instruction.push_str(&format!(
            " Clone checks exclude {}.",
            settings
                .facts
                .clone_exclude
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    instruction
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: None,
        repository_factory: Some(build),
        instruction,
    }
}

impl RepositoryRule for NearCloneRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "syntax-token sequence is nearly duplicated",
            default_enabled: self.enabled,
        }
    }

    fn select_analysis(&self, selection: &mut AnalysisSelection) {
        selection.near_clones = Some(self.config);
    }

    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        for fact in &facts.near_clones {
            let clone = &fact.value;
            if self.excluded(display_root, &clone.left) || self.excluded(display_root, &clone.right)
            {
                continue;
            }
            let (first, second) = if clone.left <= clone.right {
                (&clone.left, &clone.right)
            } else {
                (&clone.right, &clone.left)
            };
            let mut finding = Finding::new(
                KEY,
                Severity::Error,
                location(display_root, first),
                format!("{} tokens, {} changed", clone.tokens, clone.changed_tokens),
                "syntax-token sequence is duplicated with consistent substitutions",
            );
            finding.help = Some("extract the shared behavior".to_owned());
            let second_location = location(display_root, second);
            finding.related.push(RelatedLocation {
                location: second_location.clone(),
                message: "similar sequence".to_owned(),
            });
            candidates.push(Candidate::lines(finding, vec![second_location]));
        }
    }
}

impl NearCloneRule {
    fn excluded(&self, display_root: &Path, span: &infact_core::SourceSpan) -> bool {
        let root = if display_root.is_absolute() {
            display_root.to_path_buf()
        } else {
            self.repository_root.join(display_root)
        };
        let path = root.join(&span.path);
        self.exclude.iter().any(|exclude| path.starts_with(exclude))
    }
}

fn location(root: &Path, span: &infact_core::SourceSpan) -> Location {
    let mut location = Location::point(
        root.join(&span.path).to_string_lossy(),
        span.start_line as usize,
        1,
    );
    location.end_line = Some(span.end_line as usize);
    location
}
