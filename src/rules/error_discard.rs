use std::collections::BTreeSet;
use std::path::Path;

use globset::{Glob, GlobMatcher};
use infact_analysis::{AnalysisSelection, FactBatch};
use infact_core::{Certainty, Containment, DiscardForm, ErrorDiscard, Reach, SourceSpan};

use crate::Settings;
use crate::config::{AmbiguousPolicy, ErrorSettings, TestPolicy};
use crate::finding::{EvidenceStep, Finding, Location, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("error-discard");

struct ErrorDiscardRule {
    denied: BTreeSet<DiscardForm>,
    ambiguous: AmbiguousPolicy,
    tests: TestPolicy,
    allowed_in: Vec<GlobMatcher>,
    allowed_in_text: Vec<String>,
    configured: bool,
}

impl ErrorDiscardRule {
    fn new(errors: Option<&ErrorSettings>) -> Self {
        let configured = errors.is_some();
        let errors = errors.cloned().unwrap_or_default();
        Self {
            denied: errors.deny.iter().copied().collect(),
            ambiguous: errors.ambiguous,
            tests: errors.tests,
            allowed_in: errors
                .allowed_in
                .iter()
                .map(|pattern| {
                    Glob::new(pattern)
                        .expect("error path patterns were validated")
                        .compile_matcher()
                })
                .collect(),
            allowed_in_text: errors.allowed_in.clone(),
            configured,
        }
    }

    fn allowed(&self, path: &Path) -> bool {
        let text = path.to_string_lossy();
        self.allowed_in
            .iter()
            .any(|matcher| matcher.is_match(text.as_ref()))
    }

    /// Decide whether one discard is a finding, and how loud it is.
    ///
    /// Syntax cannot separate `Option::unwrap_or_default` from the `Result`
    /// one, so an unresolved receiver is reported only when asked for.
    fn severity(&self, discard: &ErrorDiscard) -> Option<Severity> {
        if !self.denied.contains(&discard.form) {
            return None;
        }
        if discard.in_test {
            return match self.tests {
                TestPolicy::Ignore => None,
                TestPolicy::Warn => Some(Severity::Warning),
                TestPolicy::Error => Some(Severity::Error),
            };
        }
        match discard.certainty {
            Certainty::Certain => Some(Severity::Error),
            Certainty::Possible => match self.ambiguous {
                AmbiguousPolicy::Skip => None,
                AmbiguousPolicy::Warn => Some(Severity::Warning),
                AmbiguousPolicy::Error => Some(Severity::Error),
            },
        }
    }
}

/// State what was dropped, and how far it could have travelled.
///
/// An ancestor that returns `Result` could have been told, so the fix is a
/// signature or two. When nothing above can report it, no caller can ever
/// learn of the failure, and the message has to say so.
fn message(discard: &ErrorDiscard) -> String {
    let dropped = match discard.form {
        DiscardForm::LetUnderscore => "the error is dropped by binding to `_`",
        DiscardForm::OkDiscard => "`.ok()` turns the error into an absence",
        DiscardForm::UnwrapOr => "a fallback replaces the error",
        DiscardForm::ErrArm => "the `Err(_)` arm reads nothing from the failure",
        DiscardForm::OkBinding => "no arm binds the error",
        DiscardForm::IteratorDrop => "failed items are dropped mid-iteration",
        DiscardForm::CauseErased => "`map_err(|_| ..)` discards the cause",
        DiscardForm::Panic => "the failure aborts instead of returning",
    };
    let route = match discard.reach {
        Reach::Local => format!(
            "{} returns `Result` and could have returned it",
            discard.callable
        ),
        Reach::Ancestor => match discard.path.first() {
            Some(edge) => format!(
                "{} cannot report it, but {} above returns `Result`",
                discard.callable, edge.caller
            ),
            None => format!("{} cannot report it", discard.callable),
        },
        Reach::Sealed => match discard.path.first() {
            Some(edge) => format!(
                "no caller from {} down to {} can report a failure",
                edge.caller, discard.callable
            ),
            None => format!(
                "{} cannot report it, and neither can any caller",
                discard.callable
            ),
        },
        Reach::Unknown => match discard.containment {
            Containment::Optional => format!(
                "{} returns `Option`, so a failure can only leave as an absence",
                discard.callable
            ),
            _ => format!(
                "{} returns no error type, so the failure cannot leave it",
                discard.callable
            ),
        },
    };
    format!("{dropped}; {route}")
}

/// Say what would actually fix it, which depends on how far the failure got.
///
/// A sealed chain is not fixed by one signature: every caller down to the
/// discard has to be able to carry the failure, or it has to be handled here.
fn help(discard: &ErrorDiscard) -> String {
    match discard.reach {
        Reach::Local => "propagate with `?` instead of discarding".to_owned(),
        Reach::Ancestor => format!(
            "return `Result` from {} and propagate to the caller that already does",
            discard.callable
        ),
        Reach::Sealed => format!(
            "give {} and every caller below a `Result` return type, or handle the failure here",
            discard
                .path
                .first()
                .map_or(discard.callable.as_str(), |edge| edge.caller.as_str())
        ),
        Reach::Unknown => format!(
            "give {} a `Result` return type and propagate with `?`",
            discard.callable
        ),
    }
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(ErrorDiscardRule::new(settings.errors.as_ref()))
}

fn instruction(settings: &Settings) -> String {
    let Some(errors) = &settings.errors else {
        return "Discarded errors are not checked.".to_owned();
    };
    let forms = errors
        .deny
        .iter()
        .map(|form| form.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "A failure has to reach the caller. These forms drop it and are denied: [{forms}]. Return `Result` and propagate with `?`."
    );
    output.push_str(match errors.ambiguous {
        AmbiguousPolicy::Skip => " Forms that syntax cannot prove are `Result` are not reported.",
        AmbiguousPolicy::Warn => " Forms that may be `Option` are reported as warnings.",
        AmbiguousPolicy::Error => " Forms that may be `Option` are reported as errors.",
    });
    if !errors.allowed_in.is_empty() {
        output.push_str(&format!(
            " Discards are permitted in [{}].",
            errors.allowed_in.join(", ")
        ));
    }
    output
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: None,
        repository_factory: Some(build),
        instruction,
    }
}

impl RepositoryRule for ErrorDiscardRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "a fallible expression's error is discarded instead of returned",
            default_enabled: self.configured,
        }
    }

    fn select_analysis(&self, selection: &mut AnalysisSelection) {
        selection.error_discards = true;
    }

    /// Report the denied discards, each with the callable that contained it.
    ///
    /// The enclosing callable is the evidence: its return type is what decided
    /// whether the error had a route out.
    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        for fact in &facts.error_discards {
            let discard = &fact.value;
            if self.allowed(&discard.span.path) {
                continue;
            }
            let Some(severity) = self.severity(discard) else {
                continue;
            };
            let mut finding = Finding::new(
                KEY,
                severity,
                location(display_root, &discard.span),
                discard.form.as_str(),
                message(discard),
            );
            finding.help = Some(if self.allowed_in_text.is_empty() {
                help(discard)
            } else {
                format!(
                    "{}, or move the site into [{}]",
                    help(discard),
                    self.allowed_in_text.join(", ")
                )
            });
            finding.evidence = discard
                .path
                .iter()
                .map(|edge| EvidenceStep {
                    location: location(display_root, &edge.call),
                    message: format!("{} calls {}", edge.caller, edge.callee),
                })
                .chain([EvidenceStep {
                    location: location(display_root, &discard.callable_span),
                    message: format!(
                        "{} is {} and discards `{}`",
                        discard.callable,
                        discard.containment.as_str(),
                        discard.expression
                    ),
                }])
                .collect();
            candidates.push(Candidate::lines(
                finding,
                suppression_lines(display_root, &discard.span),
            ));
        }
    }
}

/// Every line a marker may sit on to suppress a discard.
///
/// A discard can span several lines, and `rustfmt` moves a trailing comment on
/// a `let .. else {` into the block below it, which would otherwise carry the
/// marker off the reported line and silently stop suppressing.
fn suppression_lines(display_root: &Path, span: &SourceSpan) -> Vec<Location> {
    let display = display_root.join(&span.path).to_string_lossy().into_owned();
    ((span.start_line + 1)..=span.end_line)
        .map(|line| Location::point(display.clone(), line as usize, 1))
        .collect()
}

fn location(display_root: &Path, span: &SourceSpan) -> Location {
    let mut location = Location::point(
        display_root.join(&span.path).to_string_lossy(),
        span.start_line as usize,
        span.start_column.unwrap_or(1) as usize,
    );
    location.end_line = Some(span.end_line as usize);
    location
}
