use std::collections::BTreeSet;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use entl_codebase::language_profile_for_extension;
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

/// The words a finding uses for the language it is about.
///
/// Infact's facts name no language on purpose: `DiscardForm::ErrArm` is one
/// fact whether it was written `Err(_) =>` or `catch { }`, which is what lets
/// one analyzer serve every language. A finding is prose for a human, so the
/// language's own vocabulary belongs here, in the layer that owns policy and
/// presentation.
///
/// **The default is neutral, not Rust.** A language nobody has written prose
/// for gets a description that is merely general. Before this it got one that
/// was specifically false: TypeScript and JavaScript discards were reported as
/// `Err(_)` arms on functions that "return `Result`", to be fixed by
/// propagating "with `?`". None of those exist in either language, and the
/// three findings were correct — only the words were wrong, which is the worse
/// failure of the two because the reader has no way to tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vocabulary {
    Rust,
    /// TypeScript and JavaScript, which spell failure the same way.
    EcmaScript,
    Neutral,
}

impl Vocabulary {
    /// Which words to use, from the file the discard sits in.
    ///
    /// The language comes from the path rather than from the fact, because
    /// putting it on the fact would make a neutral type carry a language for
    /// the sake of one consumer's prose.
    fn of(path: &Path) -> Self {
        let Some(language) = path
            .extension()
            .and_then(|extension| extension.to_str())
            .and_then(language_profile_for_extension)
        else {
            return Self::Neutral;
        };
        match language.id {
            "rust" => Self::Rust,
            "typescript" | "javascript" => Self::EcmaScript,
            _ => Self::Neutral,
        }
    }

    /// What happened to the failure, in the spelling that produced it.
    ///
    /// A form a language cannot produce falls through to the neutral wording
    /// rather than being invented: the ECMAScript packs describe three forms,
    /// and claiming a fourth reads as knowledge nobody has.
    ///
    /// The last four forms take one wording for every language because neither
    /// the fact nor the sentence differs: a fallback, an unbound arm, a dropped
    /// item and an abort are described the same way wherever they happen.
    fn dropped(self, form: DiscardForm) -> &'static str {
        match (self, form) {
            (Self::Rust, DiscardForm::LetUnderscore) => "the error is dropped by binding to `_`",
            (Self::Rust, DiscardForm::OkDiscard) => "`.ok()` turns the error into an absence",
            (Self::Rust, DiscardForm::ErrArm) => "the `Err(_)` arm reads nothing from the failure",
            (Self::Rust, DiscardForm::CauseErased) => "`map_err(|_| ..)` discards the cause",

            (Self::EcmaScript, DiscardForm::LetUnderscore) => {
                "`void` discards the result, and the rejection with it"
            }
            (Self::EcmaScript, DiscardForm::OkDiscard) => {
                "`.catch(() => ..)` turns the rejection into a value"
            }
            (Self::EcmaScript, DiscardForm::ErrArm) => {
                "the `catch` binds nothing, so the cause is unread"
            }

            (_, DiscardForm::LetUnderscore) => "the failure is bound to nothing",
            (_, DiscardForm::OkDiscard) => "the failure is turned into an absence",
            (_, DiscardForm::ErrArm) => "the failure handler reads nothing from it",
            (_, DiscardForm::CauseErased) => "the cause is replaced and lost",

            (_, DiscardForm::UnwrapOr) => "a fallback replaces the error",
            (_, DiscardForm::OkBinding) => "no arm binds the error",
            (_, DiscardForm::IteratorDrop) => "failed items are dropped mid-iteration",
            (_, DiscardForm::Panic) => "the failure aborts instead of returning",
        }
    }

    /// That the discarding callable could itself have reported the failure.
    ///
    /// A whole clause rather than a noun to drop into one, because the shape of
    /// the sentence differs and not only its vocabulary: Rust says a signature
    /// offered a route and it was declined, while ECMAScript has no signature
    /// to point at and says the failure was already on its way out.
    fn local_route(self, callable: &str) -> String {
        match self {
            Self::Rust => format!("{callable} returns `Result` and could have returned it"),
            Self::EcmaScript => {
                format!("{callable} can reject, so letting it would have carried the cause")
            }
            Self::Neutral => {
                format!("{callable} can report a failure and could have reported this one")
            }
        }
    }

    /// What a callable that can carry a failure out of itself returns.
    ///
    /// ECMAScript takes the neutral wording here and in the two below rather
    /// than a spelling of its own, and that is deliberate. Every ECMAScript
    /// callable can throw, so no signature declines a failure: `Reach` is
    /// always `Local` and `Containment` always `Fallible`, and every branch
    /// these three feed is unreachable for it. Inventing prose for a branch
    /// that cannot fire would read as knowledge nobody has, and if the packs
    /// ever change so that it can, general wording is right and specific
    /// wording would be wrong.
    fn carrier(self) -> &'static str {
        match self {
            Self::Rust => "`Result`",
            Self::EcmaScript | Self::Neutral => "a type that carries the failure",
        }
    }

    /// What such a callable has to gain for the failure to leave it.
    fn signature(self) -> &'static str {
        match self {
            Self::Rust => "a `Result` return type",
            Self::EcmaScript | Self::Neutral => "a return type that carries the failure",
        }
    }

    /// How a callable that can only report an absence says so.
    fn optional(self) -> &'static str {
        match self {
            Self::Rust => "returns `Option`",
            Self::EcmaScript | Self::Neutral => "is optional",
        }
    }

    /// How this language hands a failure to its caller, as a verb phrase.
    fn propagate(self) -> &'static str {
        match self {
            Self::Rust => "propagate with `?`",
            Self::EcmaScript => "rethrow it",
            Self::Neutral => "propagate the failure",
        }
    }

    /// The whole fix, where the discarding callable can already report it.
    ///
    /// Separate from [`Self::propagate`] because this is the complete advice
    /// and that is a fragment other advice is built from. ECMAScript's is not
    /// "rethrow it instead of discarding": not catching at all is the better
    /// answer and there is no sentence that carries both as a suffix.
    fn propagate_here(self) -> &'static str {
        match self {
            Self::Rust => "propagate with `?` instead of discarding",
            Self::EcmaScript => "rethrow it, or do not catch it at all",
            Self::Neutral => "propagate the failure instead of discarding it",
        }
    }
}

/// State what was dropped, and how far it could have travelled.
///
/// An ancestor that can carry the failure could have been told, so the fix is a
/// signature or two. When nothing above can report it, no caller can ever
/// learn of the failure, and the message has to say so.
fn message(discard: &ErrorDiscard, words: Vocabulary) -> String {
    let dropped = words.dropped(discard.form);
    let carrier = words.carrier();
    let route = match discard.reach {
        Reach::Local => words.local_route(&discard.callable),
        Reach::Ancestor => match discard.path.first() {
            Some(edge) => format!(
                "{} cannot report it, but {} above returns {carrier}",
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
                "{} {}, so a failure can only leave as an absence",
                discard.callable,
                words.optional()
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
fn help(discard: &ErrorDiscard, words: Vocabulary) -> String {
    match discard.reach {
        Reach::Local => words.propagate_here().to_owned(),
        Reach::Ancestor => format!(
            "return {} from {} and propagate to the caller that already does",
            words.carrier(),
            discard.callable
        ),
        Reach::Sealed => format!(
            "give {} and every caller below {}, or handle the failure here",
            discard
                .path
                .first()
                .map_or(discard.callable.as_str(), |edge| edge.caller.as_str()),
            words.signature()
        ),
        Reach::Unknown => format!(
            "give {} {} and {}",
            discard.callable,
            words.signature(),
            words.propagate()
        ),
    }
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(ErrorDiscardRule::new(settings.errors.as_ref()))
}

/// The repository's standing policy, which no one language can speak for.
///
/// Unlike a finding this names no file, so there is no language to take words
/// from, and it must not pick one: the rule runs on Rust, TypeScript and
/// JavaScript, and a polyglot repository reading "propagate with `?`" is being
/// told to use syntax two of its three languages do not have.
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
        "A failure has to reach the caller. These forms drop it and are denied: [{forms}]. Give it a way out instead of dropping it."
    );
    output.push_str(match errors.ambiguous {
        AmbiguousPolicy::Skip => {
            " Forms that syntax cannot prove carry a failure are not reported."
        }
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
            let words = Vocabulary::of(&discard.span.path);
            let mut finding = Finding::new(
                KEY,
                severity,
                location(display_root, &discard.span),
                discard.form.as_str(),
                message(discard, words),
            );
            finding.help = Some(if self.allowed_in_text.is_empty() {
                help(discard, words)
            } else {
                format!(
                    "{}, or move the site into [{}]",
                    help(discard, words),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn discard(path: &str, form: DiscardForm, reach: Reach) -> ErrorDiscard {
        let span = SourceSpan {
            path: PathBuf::from(path),
            start_line: 1,
            end_line: 1,
            start_column: Some(1),
            end_column: Some(2),
            start_byte: Some(0),
            end_byte: Some(1),
        };
        ErrorDiscard {
            callable: "m::f".to_owned(),
            callable_span: span.clone(),
            form,
            containment: Containment::Fallible,
            certainty: Certainty::Certain,
            expression: "g()".to_owned(),
            span,
            in_test: false,
            reach,
            path: Vec::new(),
        }
    }

    /// A language nobody wrote prose for must degrade to general wording.
    ///
    /// The failure this guards is not a missing translation, it is a confident
    /// wrong one. Before this, every language was handed Rust's vocabulary, so
    /// a TypeScript author read about `Err(_)` arms and `?` with nothing in the
    /// output saying the words did not apply. A language added tomorrow must
    /// fall back to general wording, not to Rust's.
    #[test]
    fn a_language_nobody_wrote_prose_for_gets_general_words_not_rust_ones() {
        for path in ["a.zig", "a.py", "a.go", "a"] {
            let words = Vocabulary::of(Path::new(path));
            assert_eq!(words, Vocabulary::Neutral, "{path}");
            for form in [
                DiscardForm::LetUnderscore,
                DiscardForm::OkDiscard,
                DiscardForm::ErrArm,
                DiscardForm::CauseErased,
                DiscardForm::UnwrapOr,
                DiscardForm::OkBinding,
                DiscardForm::IteratorDrop,
                DiscardForm::Panic,
            ] {
                for reach in [Reach::Local, Reach::Ancestor, Reach::Sealed, Reach::Unknown] {
                    let discard = discard(path, form, reach);
                    let text = format!("{} {}", message(&discard, words), help(&discard, words));
                    for rust_only in ["`Result`", "`Option`", "`?`", "Err(_)", ".ok()", "map_err"] {
                        assert!(
                            !text.contains(rust_only),
                            "{path} {form:?} {reach:?} says {rust_only:?}: {text}"
                        );
                    }
                }
            }
        }
    }

    /// Rust must not have been made vaguer to make the others correct.
    #[test]
    fn rust_still_says_exactly_what_it_said() {
        let words = Vocabulary::of(Path::new("src/a.rs"));
        assert_eq!(words, Vocabulary::Rust);
        let local = discard("src/a.rs", DiscardForm::LetUnderscore, Reach::Local);
        assert_eq!(
            message(&local, words),
            "the error is dropped by binding to `_`; m::f returns `Result` and could have returned it"
        );
        assert_eq!(
            help(&local, words),
            "propagate with `?` instead of discarding"
        );

        let unknown = discard("src/a.rs", DiscardForm::OkDiscard, Reach::Unknown);
        assert_eq!(
            help(&unknown, words),
            "give m::f a `Result` return type and propagate with `?`"
        );
    }

    /// Every extension the ECMAScript packs read, including `.tsx`.
    #[test]
    fn ecmascript_is_told_about_its_own_forms() {
        for path in ["a.ts", "a.tsx", "a.js", "a.jsx", "a.mjs", "a.cjs", "a.mts"] {
            assert_eq!(
                Vocabulary::of(Path::new(path)),
                Vocabulary::EcmaScript,
                "{path}"
            );
        }
        let words = Vocabulary::EcmaScript;
        let arm = discard("a.ts", DiscardForm::ErrArm, Reach::Local);
        assert_eq!(
            message(&arm, words),
            "the `catch` binds nothing, so the cause is unread; \
             m::f can reject, so letting it would have carried the cause"
        );
        assert_eq!(help(&arm, words), "rethrow it, or do not catch it at all");
    }
}
