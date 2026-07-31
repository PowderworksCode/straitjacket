use std::path::Path;

use infact_analysis::{AnalysisSelection, FactBatch};

use crate::Settings;
use crate::config::IncompleteEffectPolicy;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("analysis-incomplete");

struct AnalysisIncompleteRule {
    enabled: bool,
    effects: Option<IncompleteEffectPolicy>,
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(AnalysisIncompleteRule {
        enabled: settings.facts.exact_clones
            || settings.facts.near_clones
            || !settings.facts.aspirations.is_empty()
            || settings
                .effects
                .as_ref()
                .is_some_and(|effects| effects.incomplete != IncompleteEffectPolicy::Ignore),
        effects: settings.effects.as_ref().map(|effects| effects.incomplete),
    })
}

fn instruction(_: &Settings) -> String {
    "Keep configured source parseable so every enabled fact-backed check can complete.".to_owned()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: None,
        repository_factory: Some(build),
        instruction,
    }
}

impl RepositoryRule for AnalysisIncompleteRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "fact-backed analysis could not inspect a source file",
            default_enabled: self.enabled,
        }
    }

    fn select_analysis(&self, _: &mut AnalysisSelection) {}

    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        for diagnostic in &facts.diagnostics {
            let severity = if diagnostic.analyzer == "effects" {
                match self.effects {
                    Some(IncompleteEffectPolicy::Error) => Severity::Error,
                    Some(IncompleteEffectPolicy::Warn) => Severity::Warning,
                    Some(IncompleteEffectPolicy::Ignore) | None => continue,
                }
            } else {
                Severity::Error
            };
            let finding = Finding::new(
                KEY,
                severity,
                Location::point(
                    display_root.join(&diagnostic.path).to_string_lossy(),
                    diagnostic.line as usize,
                    1,
                ),
                &diagnostic.analyzer,
                &diagnostic.message,
            );
            candidates.push(Candidate::file(finding));
        }
    }
}
