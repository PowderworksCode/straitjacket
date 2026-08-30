//! Test-quality rules, run by beamte over a treebank pack.
//!
//! Straitjacket implements none of the rules here and must not start. Beamte
//! owns what a bad test is; this file owns everything beamte deliberately
//! refuses to: getting a grammar, parsing, deciding a severity, and putting
//! findings through suppression and reporting. Beamte's DESIGN.md §6.1 splits
//! the concerns line by line and this is the straitjacket column.
//!
//! So there is exactly one call into it -- `inspect_with` -- and adding a
//! test-scoped rule to beamte lights it up here with no change to this file.
//! The alternative, a module per rule, is how the previous generation of
//! these rules fragmented until each one knew a little about parsing and none
//! of them agreed. A rule whose beamte scope is `File` is the one exception:
//! it reads every file rather than only the ones that look like tests, so it
//! runs under `env-vars` instead, and the default selection here holds it
//! out.
//!
//! Which rules run is configuration, because a project that wants one rule
//! and a project that dislikes one rule are both real. `test-rules` in
//! `straitjacket.toml` names them; unset means all of them.

use beamte::node::Unit;
use beamte::{Property, RuleId, Selection};

use crate::Settings;
use crate::finding::Severity;
use crate::language::LanguageProfile;
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::{RuleRegistration, beamte_findings};

pub const KEY: RuleKey = RuleKey::new("test-quality");

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

pub struct TestQualityRule {
    /// Empty means every test-scoped rule beamte has, which is also what it
    /// will mean after beamte grows one.
    only: Vec<RuleId>,
    /// The file-scoped rules, held out of the default selection: they are the
    /// `env-vars` rule's to run, over every file rather than only the ones
    /// that look like tests, and running them here as well would report each
    /// read in a test file twice under two keys.
    file_scoped: Vec<RuleId>,
}

impl TestQualityRule {
    pub fn new(only: Vec<RuleId>) -> Self {
        let file_scoped = beamte::catalogue()
            .iter()
            .filter(|rule| rule.scope == beamte::Scope::File)
            .map(|rule| rule.id)
            .collect();
        Self { only, file_scoped }
    }

    fn selection(&self) -> Selection<'_> {
        if self.only.is_empty() {
            Selection::Except(&self.file_scoped)
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
        if rule.scope != beamte::Scope::Tests {
            continue;
        }
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

        let pack = match crate::pack::cached(entry.pack) {
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
            candidates.push(beamte_findings::candidate(
                KEY,
                severity_of(finding.property),
                file.path,
                file.text,
                finding,
            ));
        }
    }
}

fn not_read(path: &str, message: String, help: Option<String>) -> Candidate {
    beamte_findings::not_read(KEY, path, message, help)
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
