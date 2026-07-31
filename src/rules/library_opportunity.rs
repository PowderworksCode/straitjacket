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
    aspirations: Vec<String>,
    packages: BTreeSet<String>,
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    let aspirations = settings.facts.aspirations.clone();
    let packages = aspirations
        .iter()
        .filter_map(|aspiration| {
            aspiration
                .split_once(':')
                .map(|(_, subject)| subject.split_once('@').map_or(subject, |(name, _)| name))
        })
        .map(str::to_owned)
        .collect();
    Box::new(LibraryOpportunityRule {
        aspirations,
        packages,
    })
}

fn instruction(settings: &Settings) -> String {
    if settings.facts.aspirations.is_empty() {
        return "Library behavior opportunities are not configured.".to_owned();
    }
    format!(
        "Use established APIs from {} when Straitjacket reports an equivalent local implementation.",
        settings.facts.aspirations.join(", ")
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
            summary: "local implementation matches an aspirational library API",
            default_enabled: !self.aspirations.is_empty(),
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
