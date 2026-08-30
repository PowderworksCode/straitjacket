//! Test-quality rules, run by beamte over a treebank pack.
//!
//! Straitjacket implements none of the rules here and must not start. Beamte
//! owns what a bad test is; this file owns everything beamte deliberately
//! refuses to: getting a grammar, parsing, deciding a severity, and putting
//! findings through suppression and reporting. Beamte's DESIGN.md §6.1 splits
//! the concerns line by line and this is the straitjacket column.
//!
//! So there is exactly one call into it -- `inspect_with` -- and adding a rule
//! to beamte lights it up here with no change to this file. The alternative,
//! a module per rule, is how the previous generation of these rules
//! fragmented until each one knew a little about parsing and none of them
//! agreed.
//!
//! Which rules run is configuration, because a project that wants one rule
//! and a project that dislikes one rule are both real. `test-rules` in
//! `straitjacket.toml` names them; unset means all of them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use beamte::node::Unit;
use beamte::{Property, RuleId, Selection};

use crate::Settings;
use crate::finding::{EvidenceStep, Finding, Location, Severity};
use crate::language::LanguageProfile;
use crate::pack::Pack;
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("test-quality");

/// Off unless a configuration asks for it.
///
/// The first run downloads a grammar. A scan that reaches the network because
/// the tool was upgraded is not a surprise anyone should get for free.
const DEFAULT_ENABLED: bool = false;

/// A language straitjacket can run test rules for.
///
/// `pack` is the name treebank serves the grammar under and `model` is the
/// name beamte knows the frameworks by. They differ exactly once, and that is
/// the point of having both: javascript has no pack of its own because
/// typescript's grammar parses it, but `it(...)` in a `.js` file is still
/// javascript and a finding should not claim otherwise.
struct Supported {
    /// Straitjacket's own language id, from `src/language.rs`.
    id: &'static str,
    pack: &'static str,
    model: &'static str,
    /// Cheap substrings that mean a file is worth parsing.
    ///
    /// Rust and Zig put their tests inside ordinary source files, so a path
    /// prefilter alone would make this rule silent for both -- the failure it
    /// exists to prevent. Beamte's DESIGN.md §7.2 allows the path *and one
    /// substring*, and this is that substring.
    markers: &'static [&'static str],
}

const SUPPORTED: &[Supported] = &[
    Supported {
        id: "python",
        pack: "python",
        model: "python",
        markers: &["def test", "pytest", "unittest"],
    },
    Supported {
        id: "ruby",
        pack: "ruby",
        model: "ruby",
        markers: &["def test_", "RSpec", "describe ", "it '", "it \""],
    },
    Supported {
        id: "rust",
        pack: "rust",
        model: "rust",
        markers: &["#[test]", "#[tokio::test]", "#[rstest]", "mod tests"],
    },
    Supported {
        id: "java",
        pack: "java",
        model: "java",
        markers: &["@Test", "@ParameterizedTest", "junit"],
    },
    Supported {
        id: "typescript",
        pack: "typescript",
        model: "typescript",
        markers: &["it(", "test(", "describe(", "it.each"],
    },
    Supported {
        id: "javascript",
        pack: "typescript",
        model: "javascript",
        markers: &["it(", "test(", "describe(", "it.each"],
    },
    Supported {
        id: "c",
        pack: "c",
        model: "c",
        markers: &["TEST(", "TEST_F(", "RUN_TEST", "void test_"],
    },
    Supported {
        id: "cpp",
        pack: "cpp",
        model: "cpp",
        markers: &["TEST(", "TEST_F(", "TEST_CASE(", "SCENARIO(", "void test_"],
    },
    Supported {
        id: "shell",
        pack: "bash",
        model: "bash",
        markers: &["test_", "@test", "should_"],
    },
    Supported {
        id: "zig",
        pack: "zig",
        model: "zig",
        markers: &["test \"", "test {"],
    },
];

fn supported(language: &LanguageProfile) -> Option<&'static Supported> {
    SUPPORTED.iter().find(|entry| entry.id == language.id)
}

/// Whether a path names a test file by convention alone.
fn path_names_a_test(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    let lowered = lowered.replace('\\', "/");
    lowered.contains("test") || lowered.contains("spec")
}

thread_local! {
    /// Loaded packs, and the reasons for the ones that would not load.
    ///
    /// A `FileRule` must be `Send + Sync` and a wasmer `Store` is neither, so
    /// the packs cannot live in the rule. They live beside it instead, which
    /// costs nothing today -- the walk in `src/walk.rs` is a single sequential
    /// iterator -- and stays correct rather than unsound if that ever changes.
    /// A parallel walk would pay one JIT per thread per grammar.
    ///
    /// The failure is cached with the same weight as the success: a machine
    /// with no network pays one failed fetch, not one per file.
    static PACKS: RefCell<HashMap<&'static str, Result<Rc<Pack>, String>>> =
        RefCell::new(HashMap::new());
}

/// The pack for a grammar, fetched once and then reused.
///
/// Fetched per language, and only once a file of that language has already
/// looked like a test, so a Python repository never downloads the Java
/// grammar.
fn pack(grammar: &'static str) -> Result<Rc<Pack>, String> {
    PACKS.with_borrow_mut(|packs| {
        packs
            .entry(grammar)
            .or_insert_with(|| {
                acquire(grammar)
                    .map(Rc::new)
                    .map_err(|error| format!("{error:#}"))
            })
            .clone()
    })
}

fn acquire(grammar: &'static str) -> anyhow::Result<Pack> {
    let bytes = treebank::fetch::fetch_bytes(grammar)?;
    Pack::from_bytes(&bytes, &format!("the treebank {grammar} pack"))
}

pub struct TestQualityRule {
    /// Empty means every rule beamte has, which is also what it will mean
    /// after beamte grows one.
    only: Vec<RuleId>,
}

impl TestQualityRule {
    pub fn new(only: Vec<RuleId>) -> Self {
        Self { only }
    }

    fn selection(&self) -> Selection<'_> {
        if self.only.is_empty() {
            Selection::All
        } else {
            Selection::Only(&self.only)
        }
    }
}

/// Build the rule from settings.
///
/// An unknown name was already rejected when the settings were read, by
/// `rules::resolve_test_rules`, so anything that fails to resolve here cannot
/// reach a scan.
fn build(settings: &Settings) -> Box<dyn FileRule> {
    let only = settings
        .test_rules
        .iter()
        .filter_map(|name| beamte::rule(name).map(|rule| rule.id))
        .collect();
    Box::new(TestQualityRule::new(only))
}

fn instruction(_settings: &Settings) -> String {
    let mut sentences = Vec::new();
    for rule in beamte::catalogue() {
        sentences.push(format!("{} ({})", rule.instruction, rule.citation.title));
    }
    sentences.join(" ")
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}

/// Beamte states a property; a severity is a policy about a repository, and
/// straitjacket is the one holding the policy.
///
/// A test that does not fail when the code is broken is the failure that
/// costs something -- the suite reports green over a bug. The other two make
/// a suite harder to trust and to read, which is worth saying and not worth
/// failing a build over on its own.
fn severity_of(property: Property) -> Severity {
    match property {
        Property::Fidelity => Severity::Error,
        Property::Resilience | Property::Precision => Severity::Warning,
    }
}

impl FileRule for TestQualityRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "a test is written in a way that weakens what it proves",
            default_enabled: DEFAULT_ENABLED,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        supported(language).is_some()
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        let Some(entry) = supported(file.language) else {
            return;
        };
        if !path_names_a_test(file.path) && !entry.markers.iter().any(|m| file.text.contains(m)) {
            return;
        }
        let Some(model) = beamte::TestModel::for_language(entry.model) else {
            return;
        };

        let pack = match pack(entry.pack) {
            Ok(pack) => pack,
            Err(reason) => {
                candidates.push(not_read(
                    file.path,
                    format!(
                        "not read: the {} grammar could not be loaded, so this file was \
                         not checked for test quality ({reason})",
                        entry.pack
                    ),
                    Some(
                        "Packs are downloaded once and cached. Check network access, or \
                         skip `test-quality` if this environment is offline by design."
                            .to_string(),
                    ),
                ));
                return;
            }
        };

        let tree = match pack.parse(file.text) {
            Ok(tree) => tree,
            Err(error) => {
                candidates.push(not_read(
                    file.path,
                    format!(
                        "not read: this file did not parse as {} ({error:#})",
                        entry.model
                    ),
                    None,
                ));
                return;
            }
        };

        let unit = Unit::new(file.path, tree.source(), tree.root());
        for finding in beamte::inspect_with(&unit, &model, self.selection()) {
            let line = finding.span.line;
            let column = finding.span.column;
            candidates.push(Candidate::line(Finding {
                rule: KEY,
                severity: severity_of(finding.property),
                location: Location::point(file.path, line, column),
                matched: matched_text(file.text, line),
                message: message_of(&finding),
                help: help_of(&finding),
                related: Vec::new(),
                evidence: finding
                    .evidence
                    .into_iter()
                    .map(|step| EvidenceStep {
                        location: Location::point(file.path, step.span.line, step.span.column),
                        message: step.message,
                    })
                    .collect(),
            }));
        }
    }
}

/// A file that could not be checked, said out loud.
///
/// Beamte's DESIGN.md §7.3: a file that was not read is reported as unread,
/// never as clean. Returning nothing here would mean a failed pack fetch
/// reads exactly like a suite with nothing wrong in it, which is the failure
/// this whole rule exists to stop making.
fn not_read(path: &str, message: String, help: Option<String>) -> Candidate {
    Candidate::file(Finding {
        rule: KEY,
        severity: Severity::Warning,
        location: Location::point(path, 1, 1),
        matched: String::new(),
        message,
        help,
        related: Vec::new(),
        evidence: Vec::new(),
    })
}

/// How a beamte finding reads once straitjacket owns it.
///
/// The beamte rule is named in the message rather than in the key, because
/// straitjacket registers one rule for all of them and `test-logic` would be
/// a key nothing in its manifest declares.
fn message_of(finding: &beamte::Finding) -> String {
    format!("{}: {}", finding.rule, finding.message)
}

/// The fix, and the post the rule was issued under.
///
/// Beamte's DESIGN.md §6.3 puts citing the post on the host: it turns an
/// argument with a linter into a much shorter argument with Titus Winters,
/// and makes the rule set auditable rather than one person's taste.
fn help_of(finding: &beamte::Finding) -> Option<String> {
    let citation = beamte::rule(finding.rule.as_str()).map(|rule| rule.citation);
    match (&finding.help, citation) {
        (Some(help), Some(citation)) => {
            Some(format!("{help} — {} ({})", citation.title, citation.url))
        }
        (Some(help), None) => Some(help.clone()),
        (None, Some(citation)) => Some(format!("{} ({})", citation.title, citation.url)),
        (None, None) => None,
    }
}

/// The line a finding sits on, trimmed, for the `matched` field every other
/// rule fills in from its own regex.
fn matched_text(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{KEY, SUPPORTED, path_names_a_test, severity_of};
    use beamte::Property;
    use beamte::finding::RuleId;

    #[test]
    fn every_supported_language_has_a_beamte_model() {
        for entry in SUPPORTED {
            assert!(
                beamte::TestModel::for_language(entry.model).is_some(),
                "{} maps to model {}, which beamte does not have",
                entry.id,
                entry.model
            );
        }
    }

    #[test]
    fn every_supported_language_is_one_straitjacket_knows() {
        for entry in SUPPORTED {
            assert!(
                crate::language::language_profile(entry.id).is_some(),
                "{} is not a language straitjacket has a profile for",
                entry.id
            );
        }
    }

    #[test]
    fn every_supported_language_can_be_recognised_without_a_test_shaped_path() {
        for entry in SUPPORTED {
            assert!(
                !entry.markers.is_empty(),
                "{} has no content markers, so its inline tests are invisible",
                entry.id
            );
        }
    }

    #[test]
    fn a_test_shaped_path_is_recognised_however_it_is_written() {
        assert!(path_names_a_test("tests/pack_host.rs"));
        assert!(path_names_a_test("src/foo_test.go"));
        assert!(path_names_a_test("spec/models/user_spec.rb"));
        assert!(path_names_a_test("src\\Widget.Test.cs"));
        assert!(!path_names_a_test("src/main.rs"));
    }

    #[test]
    fn a_broken_suite_is_an_error_and_a_muddled_one_is_a_warning() {
        assert_eq!(
            severity_of(Property::Fidelity),
            crate::finding::Severity::Error
        );
        assert_eq!(
            severity_of(Property::Precision),
            crate::finding::Severity::Warning
        );
    }

    #[test]
    fn the_key_is_the_one_the_registry_carries() {
        assert_eq!(KEY.as_str(), "test-quality");
        let _: RuleId = beamte::catalogue()[0].id;
    }
}
