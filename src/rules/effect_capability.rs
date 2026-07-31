use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};
use infact_analysis::{AnalysisSelection, FactBatch};
use infact_core::{Effect, EffectTrace, SourceSpan};

use crate::Settings;
use crate::config::{EffectCapability, EffectSettings, UnlistedEffectPolicy};
use crate::finding::{EvidenceStep, Finding, Location, Severity};
use crate::rule::{Candidate, RepositoryRule, RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("effect-capability");

struct PathPolicy {
    matchers: Vec<GlobMatcher>,
}

impl PathPolicy {
    fn new(patterns: &[String]) -> Self {
        Self {
            matchers: patterns
                .iter()
                .map(|pattern| {
                    Glob::new(pattern)
                        .expect("effect path patterns were validated")
                        .compile_matcher()
                })
                .collect(),
        }
    }

    fn matches(&self, root: &Path, display_root: &Path, path: &Path) -> bool {
        let joined = display_root.join(path);
        let mut candidates = vec![normalize(path), normalize(&joined)];
        if let Ok(relative) = joined.strip_prefix(root) {
            candidates.push(normalize(relative));
        }
        self.matchers
            .iter()
            .any(|matcher| candidates.iter().any(|path| matcher.is_match(path)))
    }
}

struct CapabilityPolicy {
    name: String,
    includes: BTreeSet<Effect>,
    provided_by: PathPolicy,
    available_to: PathPolicy,
    provided_by_text: Vec<String>,
    available_to_text: Vec<String>,
}

impl CapabilityPolicy {
    fn new(capability: &EffectCapability) -> Self {
        Self {
            name: capability.name.clone(),
            includes: capability.includes.iter().copied().collect(),
            provided_by: PathPolicy::new(&capability.provided_by),
            available_to: PathPolicy::new(&capability.available_to),
            provided_by_text: capability.provided_by.clone(),
            available_to_text: capability.available_to.clone(),
        }
    }
}

struct EffectCapabilityRule {
    root: PathBuf,
    unlisted: UnlistedEffectPolicy,
    capabilities: Vec<CapabilityPolicy>,
    configured: bool,
}

impl EffectCapabilityRule {
    fn new(root: PathBuf, effects: Option<&EffectSettings>) -> Self {
        let configured = effects.is_some();
        let effects = effects.cloned().unwrap_or_default();
        Self {
            root,
            unlisted: effects.unlisted,
            capabilities: effects
                .capabilities
                .iter()
                .map(CapabilityPolicy::new)
                .collect(),
            configured,
        }
    }

    fn capability(&self, effect: Effect) -> Option<&CapabilityPolicy> {
        self.capabilities
            .iter()
            .find(|capability| capability.includes.contains(&effect))
    }

    fn direct_finding(&self, trace: &EffectTrace, display_root: &Path) -> Option<Finding> {
        let call = trace.path.last()?;
        let capability = self.capability(trace.effect);
        let Some(capability) = capability else {
            return (self.unlisted == UnlistedEffectPolicy::Deny).then(|| {
                let mut finding = effect_finding(
                    trace,
                    display_root,
                    &call.call,
                    trace.effect.as_str(),
                    format!(
                        "{} effect is not assigned to a capability",
                        trace.effect.as_str()
                    ),
                );
                finding.help = Some(
                    "assign the effect to [[effects.capabilities]], or explicitly allow unlisted effects"
                        .to_owned(),
                );
                finding
            });
        };
        if capability
            .provided_by
            .matches(&self.root, display_root, &call.call.path)
        {
            return None;
        }
        let mut finding = effect_finding(
            trace,
            display_root,
            &call.call,
            &capability.name,
            format!(
                "{} capability is not provided by this path",
                capability.name
            ),
        );
        finding.help = Some(format!(
            "put the direct operation in {}",
            capability.provided_by_text.join(", ")
        ));
        Some(finding)
    }

    fn access_finding(&self, trace: &EffectTrace, display_root: &Path) -> Option<Finding> {
        let capability = self.capability(trace.effect)?;
        let call = trace.path.first()?;
        if capability
            .provided_by
            .matches(&self.root, display_root, &call.call.path)
            || capability
                .available_to
                .matches(&self.root, display_root, &call.call.path)
        {
            return None;
        }
        let mut finding = effect_finding(
            trace,
            display_root,
            &call.call,
            &capability.name,
            format!(
                "{} capability is not available to this path",
                capability.name
            ),
        );
        finding.help = Some(if capability.available_to_text.is_empty() {
            format!(
                "call through a permitted owner, or configure where {} is available",
                capability.name
            )
        } else {
            format!(
                "keep transitive access within {}",
                capability.available_to_text.join(", ")
            )
        });
        Some(finding)
    }
}

fn build(settings: &Settings) -> Box<dyn RepositoryRule> {
    Box::new(EffectCapabilityRule::new(
        settings.config_root.clone(),
        settings.effects.as_ref(),
    ))
}

fn instruction(settings: &Settings) -> String {
    let Some(effects) = &settings.effects else {
        return "Effect capabilities are not configured.".to_owned();
    };
    let mut output = String::from(
        "Effect capabilities are checked from Infact call traces. Put direct effectful calls only in each capability's provided-by paths; reach them transitively only from its available-to paths.",
    );
    for capability in &effects.capabilities {
        output.push_str(&format!(
            " {} provides [{}] from [{}] and makes it available to [{}].",
            capability.name,
            capability
                .includes
                .iter()
                .map(|effect| effect.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            capability.provided_by.join(", "),
            capability.available_to.join(", ")
        ));
    }
    output.push_str(match effects.unlisted {
        UnlistedEffectPolicy::Deny => " Effects not assigned to a capability are denied.",
        UnlistedEffectPolicy::Allow => " Effects not assigned to a capability are allowed.",
    });
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

impl RepositoryRule for EffectCapabilityRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "effect capability is provided or reached from a disallowed path",
            default_enabled: self.configured,
        }
    }

    fn select_analysis(&self, selection: &mut AnalysisSelection) {
        selection.call_effects = true;
    }

    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>) {
        for fact in &facts.effect_traces {
            let finding = if fact.value.path.len() == 1 {
                self.direct_finding(&fact.value, display_root)
            } else {
                self.access_finding(&fact.value, display_root)
            };
            if let Some(finding) = finding {
                candidates.push(Candidate::line(finding));
            }
        }
    }
}

fn effect_finding(
    trace: &EffectTrace,
    display_root: &Path,
    span: &SourceSpan,
    matched: &str,
    message: String,
) -> Finding {
    let mut finding = Finding::new(
        KEY,
        Severity::Error,
        location(display_root, span),
        matched,
        message,
    );
    finding.evidence = trace
        .path
        .iter()
        .map(|edge| EvidenceStep {
            location: location(display_root, &edge.call),
            message: format!("{} calls {}", edge.caller, edge.callee),
        })
        .collect();
    finding
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

fn normalize(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::CurDir => None,
            _ => Some(component.as_os_str().to_string_lossy()),
        })
        .collect::<Vec<_>>()
        .join("/")
}
