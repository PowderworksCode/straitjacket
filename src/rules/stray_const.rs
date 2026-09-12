//! Constants declared outside the files that hold them, found by beamte's
//! `const-declaration` rule over a treebank pack.
//!
//! Straitjacket implements none of the analysis and must not start. Beamte
//! owns what a constant declaration *is*; this file owns everything beamte
//! refuses to: getting a grammar, parsing, deciding a severity, and -- the
//! part that is policy about a repository rather than a fact about a tree --
//! naming the files that hold the constants. `const-files` in
//! `straitjacket.toml` names them, and a declaration inside one is what the
//! rule is asking for rather than something to report.
//!
//! The analysis is beamte's because the question is a tree question. The
//! whole content of the rule is the difference between *declaring* a name and
//! *using* one, and text cannot tell those apart without a per-language table
//! of declaration keywords -- which is a parser written badly, and was the
//! first attempt at this rule. Beamte asks the vocabulary instead: a
//! `_binding` that is neither an import nor a parameter, outside any
//! callable. That needs no language table at all, so unlike `test-quality`
//! there is nothing here to keep in step per language beyond which pack
//! serves which grammar.
//!
//! A finding is an error rather than a mapping of beamte's property -- the
//! rule has no property, being a fact about how code is arranged rather than
//! a claim about a test. It is opt-in, and a repository that turned it on
//! wants the declaration moved, not mentioned.

use beamte::node::Unit;
use beamte::{RuleId, Selection};

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::language::LanguageProfile;
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

pub const KEY: RuleKey = RuleKey::new("stray-const");

/// Off unless a configuration asks for it.
///
/// Two reasons, either enough on its own. The first scan of a language
/// downloads its grammar, and a scan that reaches the network because the
/// tool was upgraded is not a surprise anyone should get for free. And the
/// rule has nothing to say until somebody says where constants belong, which
/// is a decision no default can make.
const DEFAULT_ENABLED: bool = false;

/// A language this rule can read: the ten treebank publishes a grammar for.
///
/// Only the pack name is needed. beamte's rule reads the node vocabulary
/// rather than any language's declaration syntax, so there is no per-language
/// table to drift -- which is the difference between this rule and both of
/// straitjacket's other beamte hosts.
const SUPPORTED: &[(&str, &str)] = &[
    ("c", "c"),
    ("cpp", "cpp"),
    ("java", "java"),
    ("javascript", "typescript"),
    ("python", "python"),
    ("ruby", "ruby"),
    ("rust", "rust"),
    ("shell", "bash"),
    ("typescript", "typescript"),
    ("zig", "zig"),
];

fn supported(language: &LanguageProfile) -> Option<&'static str> {
    SUPPORTED
        .iter()
        .find(|(id, _)| *id == language.id)
        .map(|(_, pack)| *pack)
}

pub struct StrayConstRule {
    /// The designated homes. A declaration inside one is what the rule is
    /// asking for, so it is not reported. Same matching as
    /// `file-size-exclude`, so one notion of "this path" covers both.
    allow: Vec<std::path::PathBuf>,
    /// `Only(const-declaration)`, resolved through beamte's catalogue so a
    /// name this crate no longer has cannot reach a scan.
    only: Vec<RuleId>,
}

impl StrayConstRule {
    pub fn new(allow: Vec<std::path::PathBuf>) -> Self {
        let only = beamte::catalogue()
            .iter()
            .filter(|rule| rule.id.as_str() == "const-declaration")
            .map(|rule| rule.id)
            .collect();
        Self { allow, only }
    }

    fn designated(&self, path: &str) -> bool {
        let path = std::path::Path::new(path);
        self.allow.iter().any(|allowed| {
            path == allowed
                || path.starts_with(allowed)
                || allowed.is_relative() && path.is_absolute() && path.ends_with(allowed)
        })
    }

    fn hint(&self) -> String {
        match self.allow.first() {
            Some(home) => format!(
                "declare it in {} and reference it from here",
                home.display()
            ),
            None => "declare it in one of the files named by `const-files`".to_string(),
        }
    }
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(StrayConstRule::new(settings.const_files.clone()))
}

fn instruction(settings: &Settings) -> String {
    let sentence = beamte::rule("const-declaration")
        .map(|rule| rule.instruction.to_string())
        .unwrap_or_default();
    if settings.const_files.is_empty() {
        return sentence;
    }
    let designated = settings
        .const_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{sentence} The files that hold them are {designated}.")
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}

impl FileRule for StrayConstRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "a constant is declared outside the designated constant files",
            default_enabled: DEFAULT_ENABLED,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        supported(language).is_some()
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        let Some(pack_name) = supported(file.language) else {
            return;
        };
        if self.designated(file.path) {
            return;
        }
        if !looks_like_a_constant(file.text) {
            return;
        }
        let Some(model) = beamte::TestModel::for_language(file.language.id)
            .or_else(|| beamte::TestModel::for_language(pack_name))
        else {
            return;
        };

        let pack = match crate::pack::cached(pack_name) {
            Ok(pack) => pack,
            Err(reason) => {
                candidates.push(not_read(
                    file.path,
                    format!(
                        "not read: the {pack_name} grammar could not be loaded, so this \
                         file was not checked for constant declarations ({reason})"
                    ),
                    Some(
                        "Packs are downloaded once and cached. Check network access, or \
                         skip `stray-const` if this environment is offline by design."
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
                    format!("not read: this file did not parse as {pack_name} ({error:#})"),
                    None,
                ));
                return;
            }
        };

        let unit = Unit::new(file.path, tree.source(), tree.root());
        for finding in beamte::inspect_with(&unit, &model, Selection::Only(&self.only)) {
            let line = finding.span.line;
            let column = finding.span.column;
            let mut owned = Finding::new(
                KEY,
                Severity::Error,
                Location::point(file.path, line, column),
                matched_text(file.text, line),
                finding.message,
            );
            owned.help = Some(self.hint());
            candidates.push(Candidate::line(owned));
        }
    }
}

/// Whether the text holds anything shaped like a screaming-snake name.
///
/// The cheap half of the decision, and the reason a file that cannot produce
/// a finding is never handed to a grammar. It over-reports on purpose: `A_B` inside a
/// comment passes here and is dismissed by the parse, which is the right way
/// round for a prefilter.
fn looks_like_a_constant(text: &str) -> bool {
    let mut upper_run = 0usize;
    let mut seen_underscore = false;
    for character in text.chars() {
        if character.is_ascii_uppercase() || character.is_ascii_digit() {
            upper_run += 1;
        } else if character == '_' && upper_run > 0 {
            seen_underscore = true;
        } else {
            if seen_underscore && upper_run > 1 {
                return true;
            }
            upper_run = 0;
            seen_underscore = false;
        }
    }
    seen_underscore && upper_run > 0
}

/// A file that could not be checked, said out loud.
///
/// Beamte's DESIGN.md §7.3: a file that was not read is reported as unread,
/// never as clean. Returning nothing would mean a failed pack fetch reads
/// exactly like a file with nothing wrong in it.
fn not_read(path: &str, message: String, help: Option<String>) -> Candidate {
    let mut finding = Finding::new(
        KEY,
        Severity::Warning,
        Location::point(path, 1, 1),
        String::new(),
        message,
    );
    finding.help = help;
    Candidate::file(finding)
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
    use super::{KEY, SUPPORTED, StrayConstRule, looks_like_a_constant};

    #[test]
    fn every_supported_language_is_one_straitjacket_knows() {
        for (id, _) in SUPPORTED {
            assert!(
                crate::language::language_profile(id).is_some(),
                "{id} is not a language straitjacket has a profile for"
            );
        }
    }

    #[test]
    fn every_supported_language_has_a_beamte_model() {
        for (id, pack) in SUPPORTED {
            assert!(
                beamte::TestModel::for_language(id).is_some()
                    || beamte::TestModel::for_language(pack).is_some(),
                "{id} maps to no beamte model"
            );
        }
    }

    #[test]
    fn the_prefilter_keeps_what_could_be_a_constant_and_drops_what_could_not() {
        assert!(looks_like_a_constant("const MAX_SIZE = 3;"));
        assert!(looks_like_a_constant("A_B"));
        assert!(!looks_like_a_constant("let max_size = 3;"));
        assert!(!looks_like_a_constant("fn main() {}"));
        assert!(!looks_like_a_constant("PI"));
    }

    #[test]
    fn a_designated_file_is_designated_however_the_walk_spells_it() {
        let rule = StrayConstRule::new(vec!["src/consts.rs".into(), "src/env/".into()]);

        assert!(rule.designated("src/consts.rs"));
        assert!(rule.designated("/repo/src/consts.rs"));
        assert!(rule.designated("src/env/keys.rs"));
        assert!(!rule.designated("src/main.rs"));
    }

    #[test]
    fn the_key_is_the_one_the_registry_carries() {
        assert_eq!(KEY.as_str(), "stray-const");
    }
}
