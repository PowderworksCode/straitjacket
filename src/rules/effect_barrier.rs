//! Effects a callable is forbidden to reach, however far below it.
//!
//! straitjacket-allow-file:unknown-barrier — the markers here are test data

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use infact_analysis::{AnalysisSelection, FactBatch};
use infact_core::{Effect, EffectTrace, SourceSpan};

use crate::Settings;
use crate::config::{EffectBarrier, EffectSettings};
use crate::finding::{EvidenceStep, Finding, Location, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("effect-barrier");

struct Barrier {
    name: String,
    marker: String,
    denies: BTreeSet<Effect>,
}

impl Barrier {
    fn new(barrier: &EffectBarrier) -> Self {
        Self {
            name: barrier.name.trim().to_owned(),
            marker: barrier.marker(),
            denies: barrier.denies.iter().copied().collect(),
        }
    }
}

struct EffectBarrierRule {
    root: PathBuf,
    barriers: Vec<Barrier>,
}

/// Where a callable begins and ends, and what it is called.
type CallableSpan<'a> = (u32, u32, &'a str);

impl EffectBarrierRule {
    fn new(root: PathBuf, effects: Option<&EffectSettings>) -> Self {
        Self {
            root,
            barriers: effects
                .map(|effects| effects.barriers.iter().map(Barrier::new).collect())
                .unwrap_or_default(),
        }
    }

    /// The source a span points at, tried the ways this run may have spelled it.
    ///
    /// Spans are repository-relative, and the root a scan displays is not
    /// always the root it ran from.
    fn read(&self, display_root: &Path, path: &Path) -> Option<String> {
        [
            display_root.join(path),
            self.root.join(path),
            path.to_owned(),
        ]
        .into_iter()
        .find_map(|candidate| std::fs::read_to_string(candidate).ok()) // straitjacket-allow:error-discard — a path this run cannot spell is one of several tried, not a failure
    }

    /// Every callable a barrier marker has been put on.
    ///
    /// Only callables that reach an effect appear in the traces, so this sees
    /// only the ones worth asking about. A marker on a callable that reaches
    /// nothing has nothing to report, which is the right answer anyway.
    fn marked<'a>(
        &'a self,
        facts: &'a FactBatch,
        display_root: &Path,
    ) -> BTreeMap<&'a str, &'a Barrier> {
        let mut spans: BTreeMap<&Path, BTreeSet<CallableSpan<'a>>> = BTreeMap::new();
        for fact in &facts.effect_traces {
            let span = &fact.value.callable_span;
            spans.entry(span.path.as_path()).or_default().insert((
                span.start_line,
                span.end_line,
                fact.value.callable.as_str(),
            ));
        }

        let mut marked = BTreeMap::new();
        for (path, callables) in spans {
            let Some(text) = self.read(display_root, path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
                for barrier in &self.barriers {
                    if declares(line, &barrier.marker)
                        && let Some(callable) = marked_callable(&callables, number)
                    {
                        marked.insert(callable, barrier);
                    }
                }
            }
        }
        marked
    }
}

/// The callable a marker on this line applies to.
///
/// A marker is written on the callable it describes or on the line above it,
/// so the callable containing the line is preferred and the next one to begin
/// is the fallback.
fn marked_callable<'a>(callables: &BTreeSet<CallableSpan<'a>>, marker: u32) -> Option<&'a str> {
    let containing = callables
        .iter()
        .filter(|(start, end, _)| *start <= marker && marker <= *end)
        .min_by_key(|(start, end, _)| end.saturating_sub(*start));
    if let Some((_, _, callable)) = containing {
        return Some(callable);
    }
    callables
        .iter()
        .filter(|(start, _, _)| *start >= marker)
        .min_by_key(|(start, _, _)| *start)
        .map(|(_, _, callable)| *callable)
}

/// Whether a line declares this marker.
///
/// Two barriers can share a prefix — `hot` and `hot-loop` — so the marker has
/// to end where the name ends rather than merely appear. Suppression cannot be
/// confused for a barrier either way, since `barrier:` and `allow:` are
/// different directives.
fn declares(line: &str, marker: &str) -> bool {
    let mut rest = line;
    while let Some(offset) = rest.find(marker) {
        rest = &rest[offset + marker.len()..];
        if !rest
            .starts_with(|character: char| character.is_ascii_alphanumeric() || character == '-')
        {
            return true;
        }
    }
    false
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(EffectBarrierRule::new(
        settings.config_root.clone(),
        settings.effects.as_ref(),
    ))
}

fn instruction(settings: &Settings) -> String {
    let barriers = settings
        .effects
        .as_ref()
        .map(|effects| effects.barriers.as_slice())
        .unwrap_or_default();
    if barriers.is_empty() {
        return "Effect barriers are not configured.".to_owned();
    }
    let mut output = String::from(
        "Effect barriers are checked from Infact call traces. A marker comment on a callable forbids the listed effects in that callable and in everything it calls, however deep.",
    );
    for barrier in barriers {
        output.push_str(&format!(
            " `{}` on a callable denies [{}] below it.",
            barrier.marker(),
            barrier
                .denies
                .iter()
                .map(|effect| effect.as_str())
                .collect::<Vec<_>>()
                .join(", ")
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

impl RepositoryRule for EffectBarrierRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "an effect is reached from a callable that forbids it",
            default_enabled: !self.barriers.is_empty(),
        }
    }

    fn select_analysis(&self, selection: &mut AnalysisSelection) {
        selection.call_effects = true;
    }

    /// Report every denied effect a marked callable reaches.
    ///
    /// The finding goes on the operation rather than on the barrier, because
    /// the operation is what has to change. The barrier is where the reader
    /// finds out that it matters, and the evidence chain connects the two.
    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        if self.barriers.is_empty() {
            return;
        }
        let marked = self.marked(facts, display_root);
        for fact in &facts.effect_traces {
            let Some(barrier) = marked.get(fact.value.callable.as_str()) else {
                continue;
            };
            if !barrier.denies.contains(&fact.value.effect) {
                continue;
            }
            let Some(call) = fact.value.path.last() else {
                continue;
            };
            let mut finding = Finding::new(
                KEY,
                Severity::Error,
                location(display_root, &call.call),
                &barrier.name,
                format!(
                    "{} effect is denied below the {} barrier on {}",
                    fact.value.effect.as_str(),
                    barrier.name,
                    fact.value.callable
                ),
            );
            finding.help = Some(help(&fact.value));
            finding.evidence = evidence(&fact.value, display_root);
            candidates.push(Candidate::line(finding));
        }
    }
}

fn help(trace: &EffectTrace) -> String {
    if trace.path.len() == 1 {
        return format!(
            "the {} here comes from {}; move it outside the barrier or hoist it",
            trace.effect.as_str(),
            trace.origin
        );
    }
    format!(
        "{} reaches this through {} calls; move the {} out of the path or hoist it above the barrier",
        trace.callable,
        trace.path.len(),
        trace.effect.as_str()
    )
}

/// The chain from the barrier down to the operation, so the reader can see
/// which link to cut.
fn evidence(trace: &EffectTrace, display_root: &Path) -> Vec<EvidenceStep> {
    trace
        .path
        .iter()
        .map(|edge| EvidenceStep {
            location: location(display_root, &edge.call),
            message: format!("{} calls {}", edge.caller, edge.callee),
        })
        .collect()
}

fn location(display_root: &Path, span: &SourceSpan) -> Location {
    let mut location = Location::point(
        display_root.join(&span.path).to_string_lossy(),
        span.start_line as usize,
        1,
    );
    location.end_line = Some(span.end_line as usize);
    location
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans() -> BTreeSet<CallableSpan<'static>> {
        BTreeSet::from([(10, 20, "crate::tick"), (30, 40, "crate::draw")])
    }

    #[test]
    fn a_marker_above_a_callable_marks_it() {
        assert_eq!(marked_callable(&spans(), 9), Some("crate::tick"));
        assert_eq!(marked_callable(&spans(), 29), Some("crate::draw"));
    }

    #[test]
    fn a_marker_inside_a_callable_marks_that_callable() {
        assert_eq!(marked_callable(&spans(), 15), Some("crate::tick"));
    }

    #[test]
    fn a_marker_below_every_callable_marks_nothing() {
        assert_eq!(marked_callable(&spans(), 41), None);
    }

    /// Two barriers can share a prefix, and the shorter must not claim the
    /// longer's marker.
    #[test]
    fn a_marker_ends_where_its_name_ends() {
        assert!(declares(
            "// straitjacket-barrier:hot",
            "straitjacket-barrier:hot"
        ));
        assert!(!declares(
            "// straitjacket-barrier:hot-loop",
            "straitjacket-barrier:hot"
        ));
        assert!(declares(
            "// straitjacket-barrier:hot-loop",
            "straitjacket-barrier:hot-loop"
        ));
        assert!(declares(
            "// straitjacket-barrier:hot-loop — runs every frame",
            "straitjacket-barrier:hot-loop"
        ));
        assert!(!declares(
            "// nothing to see",
            "straitjacket-barrier:hot-loop"
        ));
    }

    /// A barrier shares the directive namespace with suppression but cannot be
    /// spelled like one, so no name turns every `straitjacket-allow` in the
    /// repository into a barrier.
    #[test]
    fn a_barrier_never_collides_with_suppression() {
        let barrier = EffectBarrier {
            name: "allow".to_owned(),
            denies: vec![Effect::Allocate],
        };
        assert_eq!(barrier.marker(), "straitjacket-barrier:allow");
        assert!(!declares(
            "// straitjacket-allow:error-discard — deliberate",
            &barrier.marker()
        ));
    }
}
