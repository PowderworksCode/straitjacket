use std::collections::BTreeSet;
use std::path::Path;

use infact_analysis::{AnalysisSelection, FactBatch};
use infact_core::LibraryTarget;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("library-opportunity");

struct LibraryOpportunityRule {
    packages: BTreeSet<String>,
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(LibraryOpportunityRule {
        packages: crate::facts::library_behavior_packages(&settings.facts),
    })
}

fn instruction(settings: &Settings) -> String {
    let packages = crate::facts::library_behavior_packages(&settings.facts);
    if packages.is_empty() {
        return "No dependency has locked library-behavior facts.".to_owned();
    }
    format!(
        "This repository already depends on {}. Use their established APIs instead of reimplementing them; Straitjacket reports equivalent local implementations.",
        packages.into_iter().collect::<Vec<_>>().join(", ")
    )
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: None,
        repository_factory: Some(build),
        instruction,
    }
}

impl RepositoryRule for LibraryOpportunityRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "local implementation matches an API from a locked dependency",
            default_enabled: !self.packages.is_empty(),
        }
    }

    fn select_analysis(&self, selection: &mut AnalysisSelection) {
        selection.library_behaviors = true;
    }

    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        for fact in &facts.library_behaviors {
            let package = match &fact.value.target {
                LibraryTarget::Callable { package, .. }
                | LibraryTarget::DeriveMacro { package, .. } => package,
            };
            if !self.packages.contains(package) {
                continue;
            }
            let target = fact.value.target.path();
            let span = &fact.value.span;
            let mut location = Location::point(
                display_root.join(&span.path).to_string_lossy(),
                span.start_line as usize,
                1,
            );
            location.end_line = Some(span.end_line as usize);
            let mut finding = Finding::new(
                KEY,
                Severity::Error,
                location,
                target,
                format!("local implementation matches {target}"),
            );
            finding.help = Some(format!("use {target}"));
            candidates.push(Candidate::line(finding));
        }
    }
}
