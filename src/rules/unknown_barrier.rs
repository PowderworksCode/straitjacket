//! Barrier markers that name no configured barrier.
//!
//! straitjacket-allow-file:unknown-barrier — this file defines the syntax

use std::collections::BTreeSet;

use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;
use crate::rules::comments;

const KEY: RuleKey = RuleKey::new("unknown-barrier");
const DIRECTIVE: &str = "straitjacket-barrier:";

/// A barrier marker naming nothing is silent, which is the worst way to fail.
///
/// A misspelled `straitjacket-barrier:hot-lop` matches no configured barrier,
/// so it forbids nothing and nobody hears about it. The same happens when a
/// barrier is deleted from configuration and its markers are left behind. Both
/// read to a contributor as a live guarantee, and both are decoration.
///
/// This is the barrier counterpart to `unused-marker`, which asks the same
/// question of suppression.
struct UnknownBarrierRule {
    configured: BTreeSet<String>,
}

impl UnknownBarrierRule {
    fn new(settings: &Settings) -> Self {
        Self {
            configured: settings
                .effects
                .as_ref()
                .map(|effects| {
                    effects
                        .barriers
                        .iter()
                        .map(|barrier| barrier.name.trim().to_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Every barrier named on a line, with the column it starts at.
///
/// The name runs to the end of what a barrier name may contain, so trailing
/// prose stops it: `straitjacket-barrier:hot-loop — every frame` names
/// `hot-loop`.
fn named_barriers(line: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find(DIRECTIVE) {
        let start = cursor + offset;
        let rest = &line[start + DIRECTIVE.len()..];
        let end = rest
            .find(|character: char| {
                !(character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-')
            })
            .unwrap_or(rest.len());
        found.push((start, &rest[..end]));
        cursor = start + DIRECTIVE.len();
    }
    found
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(UnknownBarrierRule::new(settings))
}

fn instruction(settings: &Settings) -> String {
    let rule = UnknownBarrierRule::new(settings);
    if rule.configured.is_empty() {
        return "No effect barriers are configured, so no `straitjacket-barrier:` marker means anything. Remove any that remain.".to_owned();
    }
    format!(
        "A `straitjacket-barrier:` marker must name a configured barrier: {}. A marker naming anything else forbids nothing.",
        rule.configured
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
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

impl FileRule for UnknownBarrierRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "barrier marker names no configured barrier",
            default_enabled: true,
        }
    }

    /// Prose that documents the syntax is not a marker, so the language that
    /// exists to carry prose is left alone, as suppression leaves it alone.
    fn applies_to(&self, language: &LanguageProfile) -> bool {
        comments::supports(language) && language.id != "markdown"
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        for (index, line) in file.text.lines().enumerate() {
            for (column, name) in named_barriers(line) {
                if !name.is_empty() && self.configured.contains(name) {
                    continue;
                }
                let (matched, message) = if name.is_empty() {
                    (
                        DIRECTIVE.to_owned(),
                        "barrier marker names no barrier".to_owned(),
                    )
                } else {
                    (
                        format!("{DIRECTIVE}{name}"),
                        format!("`{name}` is not a configured effect barrier"),
                    )
                };
                let mut finding = Finding::new(
                    KEY,
                    Severity::Error,
                    Location::point(file.path.to_owned(), index + 1, column + 1),
                    &matched,
                    message,
                );
                finding.help = Some(if self.configured.is_empty() {
                    "no barriers are configured; add one under [[effects.barriers]] or remove the marker".to_owned()
                } else {
                    format!(
                        "name one of {}, or remove the marker",
                        self.configured
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                });
                candidates.push(Candidate::line(finding));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_yields_the_barrier_it_names() {
        assert_eq!(
            named_barriers("// straitjacket-barrier:hot-loop — every frame"),
            vec![(3, "hot-loop")]
        );
        assert_eq!(
            named_barriers("/* straitjacket-barrier:realtime */"),
            vec![(3, "realtime")]
        );
        assert_eq!(named_barriers("nothing here"), Vec::new());
    }

    /// A marker with no name forbids nothing just as surely as a typo.
    #[test]
    fn a_marker_with_no_name_is_reported_too() {
        assert_eq!(named_barriers("// straitjacket-barrier:"), vec![(3, "")]);
    }

    /// Suppression is a different directive and must not be mistaken for one.
    #[test]
    fn suppression_is_not_a_barrier() {
        assert_eq!(
            named_barriers("// straitjacket-allow:error-discard — deliberate"),
            Vec::new()
        );
    }
}
