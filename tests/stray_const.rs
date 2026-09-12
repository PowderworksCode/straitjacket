//! `stray-const` against the languages treebank publishes a grammar for.
//!
//! The point of this file is breadth, as `tests/test_quality.rs` is for its
//! rule: a constant declared the way each language declares one. beamte's
//! analysis reads the node vocabulary rather than any language's declaration
//! syntax, so a language going quiet here is a grammar problem rather than a
//! missing table — which is the difference between this rule and the regex it
//! replaced.
//!
//! These fetch real packs, for the reason `tests/pack_host.rs` gives at
//! length: a test that passes because it found no grammar is worse than no
//! test.

use straitjacket::config::Settings;
use straitjacket::finding::Severity;
use straitjacket::scanner::Scanner;

/// A scanner with `stray-const` on and one designated home, the way a
/// configuration turns it on.
fn scanner(const_files: Vec<std::path::PathBuf>) -> Scanner {
    let settings = Settings {
        stray_const: true,
        const_files,
        ..Settings::default()
    };
    Scanner::new(&settings).expect("the scanner builds")
}

fn findings(path: &str, source: &str) -> Vec<String> {
    let extension = path.rsplit('.').next().unwrap_or("");
    scanner(vec!["src/consts.rs".into()])
        .scan(source, path, extension)
        .findings
        .into_iter()
        .map(|finding| finding.message)
        .collect()
}

/// One constant declaration per language, written the way that language
/// writes one, and the name the finding must carry.
const CASES: &[(&str, &str, &str, &str)] = &[
    ("rust", "src/a.rs", "const MAX_SIZE: u8 = 3;\n", "MAX_SIZE"),
    (
        "rust-static",
        "src/b.rs",
        "static DEFAULT_PATH: &str = \"/tmp\";\n",
        "DEFAULT_PATH",
    ),
    ("python", "src/a.py", "MAX_SIZE = 3\n", "MAX_SIZE"),
    ("ruby", "lib/a.rb", "MAX_SIZE = 3\n", "MAX_SIZE"),
    (
        "typescript",
        "src/a.ts",
        "const MAX_SIZE: number = 3;\n",
        "MAX_SIZE",
    ),
    (
        "javascript",
        "src/a.js",
        "const MAX_SIZE = 3;\n",
        "MAX_SIZE",
    ),
    (
        "java",
        "src/A.java",
        "class A {\n  static final int MAX_SIZE = 3;\n}\n",
        "MAX_SIZE",
    ),
    ("zig", "src/a.zig", "const MAX_SIZE = 3;\n", "MAX_SIZE"),
];

#[test]
fn every_language_reports_its_constant_declaration() {
    for (language, path, source, name) in CASES {
        let found = findings(path, source);
        assert!(
            found.iter().any(|message| message.contains(name)),
            "{language}: expected a finding naming {name} in {path}, got {found:?}"
        );
    }
}

#[test]
fn a_use_is_not_a_declaration() {
    let source = "fn f(n: u8) -> bool {\n    n > MAX_SIZE && n < OTHER_LIMIT\n}\n";

    assert_eq!(
        findings("src/main.rs", source),
        Vec::<String>::new(),
        "flagging uses would make the rule impossible to satisfy"
    );
}

#[test]
fn an_import_binds_a_name_without_declaring_it() {
    assert_eq!(
        findings("src/loader.py", "from settings import MAX_SIZE\n"),
        Vec::<String>::new()
    );
}

#[test]
fn a_local_inside_a_body_is_nobodys_to_gather() {
    assert_eq!(
        findings(
            "src/loader.py",
            "def f():\n    LOCAL_MAX = 4\n    return LOCAL_MAX\n"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn a_single_word_name_is_too_ambiguous_to_flag() {
    assert_eq!(
        findings("src/a.rs", "const MAX: u8 = 3;\nconst PI: f64 = 3.0;\n"),
        Vec::<String>::new()
    );
}

#[test]
fn a_declaration_that_is_not_code_is_not_a_declaration() {
    assert_eq!(
        findings("src/main.rs", "// const MAX_SIZE: u8 = 3;\nfn f() {}\n"),
        Vec::<String>::new(),
        "commented out"
    );
    assert_eq!(
        findings(
            "src/main.rs",
            "fn f() {\n    let s = \"const MAX_SIZE = 3\";\n}\n"
        ),
        Vec::<String>::new(),
        "quoted in a string"
    );
}

#[test]
fn a_declaration_is_an_error_and_says_where_it_belongs() {
    let result = scanner(vec!["src/consts.rs".into()]).scan(
        "const MAX_SIZE: u8 = 3;\n",
        "src/main.rs",
        "rs",
    );

    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Error);
    let help = result.findings[0].help.as_deref().unwrap_or_default();
    assert!(
        help.contains("src/consts.rs"),
        "the help should name the designated file, got: {help}"
    );
}

#[test]
fn the_designated_file_may_declare_and_everything_else_may_not() {
    let scanner = scanner(vec!["src/consts.rs".into()]);
    let source = "const MAX_SIZE: u8 = 3;\n";

    assert_eq!(
        scanner.scan(source, "src/consts.rs", "rs").findings,
        Vec::new(),
        "the designated file is where constants are supposed to be"
    );
    assert_eq!(scanner.scan(source, "src/main.rs", "rs").findings.len(), 1);
}

#[test]
fn data_and_prose_files_are_not_this_rules_to_read() {
    assert_eq!(
        findings("config/a.yaml", "MAX_SIZE: 3\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        findings("docs/a.md", "MAX_SIZE = 3\n"),
        Vec::<String>::new()
    );
}

#[test]
fn turning_the_rule_on_with_nowhere_to_put_a_constant_is_refused() {
    let settings = Settings {
        stray_const: true,
        ..Settings::default()
    };

    let error = match Scanner::new(&settings) {
        Ok(_) => panic!("a rule with no designated file should be refused"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("const-files"),
        "the error should name the key that fixes it: {error}"
    );
}

#[test]
fn the_rule_is_off_until_a_configuration_asks_for_it() {
    let quiet = Scanner::new(&Settings::default())
        .expect("the default scanner builds")
        .scan("const MAX_SIZE: u8 = 3;\n", "src/main.rs", "rs");

    assert_eq!(
        quiet.findings,
        Vec::new(),
        "an opt-in rule must stay silent until it is opted into"
    );
}

/// The analysis is beamte's, and the split is the point: beamte reports every
/// declaration, straitjacket decides which are licensed.
#[test]
fn the_analysis_belongs_to_beamte() {
    let rule = beamte::rule("const-declaration").expect("beamte carries the rule");

    assert_eq!(rule.scope, beamte::Scope::File);
    assert_eq!(rule.property, None);
    assert!(rule.citation.is_none());
}
