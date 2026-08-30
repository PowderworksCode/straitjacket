//! `SCREAMING_SNAKE_CASE` constants declared outside the files that hold
//! them.
//!
//! A constant is a decision about the program: a limit, a path, a key, a
//! magic number somebody named. Scattered through the tree, those decisions
//! cannot be read as a set, and the same one gets made twice under two names.
//! Gathering them is not tidiness -- it is what makes the list of decisions
//! reviewable. `const-files` names where they live and everything outside is
//! reported.
//!
//! Deliberately not a parser. A rule that has to tell one expression from
//! another needs a tree, which is why `test-quality` fetches a treebank
//! grammar. This one asks a much narrower question -- does a line *declare* a
//! screaming-snake name -- and declaration syntax answers that on its own. So
//! it runs over all eighteen languages straitjacket calls structured code
//! rather than the nine with a pack, needs no network, and is off by default
//! only because designating the files is a decision no default can make.
//!
//! It reads [`comments::code`], not the raw text: a declaration commented out
//! or quoted inside a string is not a declaration, and those are the two
//! places source code most often appears without being code.
//!
//! **Declarations, not uses.** `MAX_SIZE` mentioned in an expression is the
//! point of having a constant, and flagging it would make the rule
//! unsatisfiable. Only the site that introduces the name is a finding.

use std::path::{Path, PathBuf};

use regex::Regex;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::language::{LanguageProfile, STRUCTURED_CODE};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;
use crate::rules::comments;

pub const KEY: RuleKey = RuleKey::new("stray-const");

/// Off unless a configuration asks for it.
///
/// Unlike every default-on rule, this one has nothing to say until somebody
/// says where constants belong. A default would be a guess about a layout
/// straitjacket cannot see.
const DEFAULT_ENABLED: bool = false;

/// A screaming-snake name: at least two words, joined by underscores.
///
/// The underscore is required, which is the whole difference between a rule
/// and a nuisance. A single all-caps word is ambiguous in every language that
/// has one -- `PI`, `OK`, `HTTP`, a Go export, a C macro guard, a type
/// parameter -- and flagging those would bury the constants among them.
const NAME: &str = r"[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+";

/// Whether a bare `NAME = value` declares a constant in this language.
///
/// In the C family and its descendants a declaration carries a keyword, so a
/// bare assignment is either a write to something already declared or an enum
/// member -- neither of which is a constant anyone can move. Reading it as a
/// declaration there is how this rule would come to flag every `enum` body in
/// the repository.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bare {
    /// Never; the language spells declarations with a keyword.
    No,
    /// Only at the left margin, which is where a module-level constant sits.
    /// Indented, the same line is a class attribute, an enum member or a
    /// local, and none of those belongs in another file.
    TopLevel,
    /// At any indentation: Go writes its constants inside a `const (` block.
    AnyIndent,
}

fn bare_form(language: &LanguageProfile) -> Bare {
    match language.id {
        "python" | "ruby" | "shell" => Bare::TopLevel,
        "go" => Bare::AnyIndent,
        _ => Bare::No,
    }
}

pub struct StrayConstRule {
    /// The designated homes. A declaration inside one is what the rule is
    /// asking for, so it is not reported.
    allow: Vec<PathBuf>,
    /// `const NAME`, `static final int NAME`, `let NAME` -- a declaration
    /// keyword, an optional type, then the name.
    ///
    /// The group between the keyword and the name is that type:
    /// `static final int MAX_SIZE`, `const char *MAX_NAME`. Where the keyword
    /// sits directly against the name it matches nothing, and where a second
    /// keyword intervenes the scan simply starts again at that keyword --
    /// which is how `static final int` finds its name on the third attempt
    /// rather than needing a rule of its own.
    keyword: Regex,
    /// `#define NAME`.
    define: Regex,
    /// `NAME = value` at the left margin.
    bare_top: Regex,
    /// `NAME = value` at any indentation.
    bare_any: Regex,
    /// A screaming-snake name anywhere at all.
    ///
    /// The prefilter, and deliberately not one of the patterns above: those
    /// are anchored to a line, so asking one of them about a whole file
    /// answers for its first line only. That is how an indented Go `const (`
    /// block came to be skipped, caught by the test that covers Go. A bare
    /// name is both cheaper to look for and free of anchors.
    name: Regex,
}

impl StrayConstRule {
    pub fn new(allow: Vec<PathBuf>) -> Self {
        let compile =
            |pattern: String| Regex::new(&pattern).expect("built-in rule patterns must compile");
        Self {
            allow,
            keyword: compile(format!(
                r"\b(?:const|constexpr|static|final|readonly|let|var|val)\s+(?:mut\s+)?(?:[A-Za-z_][A-Za-z0-9_:<>\[\].]*\s+)?(?:[*&]+\s*)?({NAME})\b"
            )),
            define: compile(format!(r"^\s*#\s*define\s+({NAME})\b")),
            bare_top: compile(format!(
                r"^(?:export\s+|readonly\s+)?({NAME})\s*(?::[^=]*)?="
            )),
            bare_any: compile(format!(
                r"^\s*(?:export\s+|readonly\s+)?({NAME})\s*(?::[^=]*)?="
            )),
            name: compile(NAME.to_owned()),
        }
    }

    fn designated(&self, path: &str) -> bool {
        let path = Path::new(path);
        self.allow.iter().any(|allowed| {
            path == allowed
                || path.starts_with(allowed)
                || allowed.is_relative() && path.is_absolute() && path.ends_with(allowed)
        })
    }

    /// Every declaration on one line, as (byte offset, name).
    ///
    /// The line is already masked, so anything found here is code.
    fn declarations(&self, line: &str, bare: Bare) -> Vec<(usize, String)> {
        let mut found: Vec<(usize, String)> = Vec::new();
        let mut take = |regex: &Regex, assignment: bool| {
            for captures in regex.captures_iter(line) {
                let Some(name) = captures.get(1) else {
                    continue;
                };
                if assignment && !assigns(line, captures.get(0).map_or(0, |whole| whole.end())) {
                    continue;
                }
                if found.iter().any(|(at, _)| *at == name.start()) {
                    continue;
                }
                found.push((name.start(), name.as_str().to_owned()));
            }
        };

        take(&self.keyword, false);
        take(&self.define, false);
        match bare {
            Bare::No => {}
            Bare::TopLevel => take(&self.bare_top, true),
            Bare::AnyIndent => take(&self.bare_any, true),
        }

        found.sort_by_key(|(at, _)| *at);
        found
    }
}

/// Whether the `=` a bare pattern stopped on is really an assignment.
///
/// The regex crate has no lookahead, so the character after the match is
/// checked here instead. `==` is a comparison, `=>` a hash rocket or a match
/// arm, and `=~` a Ruby match -- three ways to write a screaming-snake name
/// beside an equals sign without declaring anything.
fn assigns(line: &str, end: usize) -> bool {
    !matches!(line.as_bytes().get(end), Some(b'=' | b'>' | b'~'))
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(StrayConstRule::new(settings.const_files.clone()))
}

fn instruction(settings: &Settings) -> String {
    if settings.const_files.is_empty() {
        return "Declare SCREAMING_SNAKE_CASE constants in the designated constant \
                files named by `const-files` in straitjacket.toml, and reference them \
                from everywhere else."
            .to_string();
    }
    let designated = settings
        .const_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "SCREAMING_SNAKE_CASE constants are declared in {designated} and nowhere else. \
         Reference them from there rather than declaring one where it is used."
    )
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
        language.has_facet(&STRUCTURED_CODE)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        if self.designated(file.path) {
            return;
        }
        if !self.name.is_match(file.text) {
            return;
        }

        let bare = bare_form(file.language);
        for (line_index, line) in comments::code(file.text, file.language).iter().enumerate() {
            for (offset, name) in self.declarations(line, bare) {
                let mut finding = Finding::new(
                    KEY,
                    Severity::Error,
                    Location::point(file.path, line_index + 1, offset + 1),
                    name.clone(),
                    format!("`{name}` is declared here rather than in a constant file"),
                );
                finding.help = Some(self.hint());
                candidates.push(Candidate::line(finding));
            }
        }
    }
}

impl StrayConstRule {
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

#[cfg(test)]
mod tests {
    use super::StrayConstRule;
    use crate::language::language_profile;
    use crate::rule::{Candidate, FileRule, SourceFile};

    fn findings(source: &str, language: &str) -> Vec<(usize, usize, String)> {
        at(source, language, "src/thing.x")
    }

    fn at(source: &str, language: &str, path: &str) -> Vec<(usize, usize, String)> {
        rule(Vec::new(), source, language, path)
    }

    fn rule(
        allow: Vec<std::path::PathBuf>,
        source: &str,
        language: &str,
        path: &str,
    ) -> Vec<(usize, usize, String)> {
        let mut candidates: Vec<Candidate> = Vec::new();
        StrayConstRule::new(allow).check(
            SourceFile {
                path,
                language: language_profile(language).expect("a language straitjacket knows"),
                text: source,
            },
            &mut candidates,
        );
        candidates
            .into_iter()
            .map(|candidate| {
                (
                    candidate.finding.location.line,
                    candidate.finding.location.col,
                    candidate.finding.matched,
                )
            })
            .collect()
    }

    #[test]
    fn flags_a_keyword_declaration_and_points_at_the_name() {
        let hits = findings("const MAX_SIZE: u8 = 3;\n", "rust");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 1);
        assert_eq!(hits[0].1, 7);
        assert_eq!(hits[0].2, "MAX_SIZE");
    }

    #[test]
    fn flags_a_declaration_behind_a_type_and_a_pointer() {
        assert_eq!(
            findings("static final int MAX_SIZE = 3;\n", "java")[0].2,
            "MAX_SIZE"
        );
        assert_eq!(
            findings("static const char *DEFAULT_NAME = \"x\";\n", "c")[0].2,
            "DEFAULT_NAME"
        );
        assert_eq!(findings("#define MAX_RETRIES 5\n", "c")[0].2, "MAX_RETRIES");
    }

    #[test]
    fn a_use_is_not_a_declaration() {
        assert!(findings("if size > MAX_SIZE { return; }\n", "rust").is_empty());
        assert!(findings("foo(MAX_SIZE, OTHER_THING);\n", "rust").is_empty());
    }

    #[test]
    fn a_single_word_name_is_left_alone() {
        assert!(findings("const MAX: u8 = 3;\n", "rust").is_empty());
        assert!(findings("const PI: f64 = 3.14;\n", "rust").is_empty());
    }

    #[test]
    fn a_commented_out_or_quoted_declaration_is_not_one() {
        assert!(findings("// const MAX_SIZE: u8 = 3;\n", "rust").is_empty());
        assert!(findings("let s = \"const MAX_SIZE = 3\";\n", "rust").is_empty());
        assert!(findings("/* const MAX_SIZE = 3 */\n", "rust").is_empty());
    }

    /// Rust spells a declaration with a keyword, so a bare `MAX_SIZE = 3`
    /// there is a write to something declared elsewhere rather than a
    /// declaration this rule could ask anyone to move.
    #[test]
    fn a_bare_assignment_declares_only_where_the_language_says_so() {
        assert_eq!(findings("MAX_SIZE = 3\n", "python")[0].2, "MAX_SIZE");
        assert_eq!(findings("MAX_SIZE = 3\n", "ruby")[0].2, "MAX_SIZE");
        assert_eq!(findings("MAX_SIZE=3\n", "shell")[0].2, "MAX_SIZE");
        assert!(findings("MAX_SIZE = 3;\n", "rust").is_empty());
    }

    #[test]
    fn an_indented_assignment_is_a_member_rather_than_a_constant() {
        let source = "class Colour(Enum):\n    RED_ONE = 1\n    GREEN_TWO = 2\n";

        assert!(
            findings(source, "python").is_empty(),
            "an enum member cannot be moved to another file"
        );
    }

    #[test]
    fn go_declares_inside_an_indented_const_block() {
        let hits = findings("const (\n\tMAX_SIZE = 100\n)\n", "go");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].2, "MAX_SIZE");
    }

    #[test]
    fn a_comparison_is_not_an_assignment() {
        assert!(findings("MAX_SIZE == 3\n", "python").is_empty());
        assert!(findings("MAX_SIZE != 3\n", "python").is_empty());
        assert!(findings("MAX_SIZE >= 3\n", "python").is_empty());
        assert!(findings("MAX_SIZE => 3\n", "ruby").is_empty());
    }

    #[test]
    fn a_designated_file_may_declare_freely() {
        let source = "const MAX_SIZE: u8 = 3;\n";

        assert!(
            rule(
                vec!["src/config.rs".into()],
                source,
                "rust",
                "src/config.rs"
            )
            .is_empty()
        );
        assert_eq!(
            rule(vec!["src/config.rs".into()], source, "rust", "src/main.rs").len(),
            1
        );
    }

    #[test]
    fn a_designated_directory_covers_what_is_under_it() {
        let source = "const MAX_SIZE: u8 = 3;\n";

        assert!(
            rule(
                vec!["src/consts/".into()],
                source,
                "rust",
                "src/consts/a.rs"
            )
            .is_empty()
        );
    }

    #[test]
    fn several_declarations_on_one_line_are_reported_once_each() {
        let hits = findings("const A_ONE: u8 = 1; const B_TWO: u8 = 2;\n", "rust");

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].2, "A_ONE");
        assert_eq!(hits[1].2, "B_TWO");
    }

    #[test]
    fn a_python_docstring_full_of_declarations_is_prose() {
        let source = "\"\"\"\nMAX_SIZE = 3\n\"\"\"\nx = 1\n";

        assert!(findings(source, "python").is_empty());
    }
}
