//! `stray-const` against the languages straitjacket calls structured code.
//!
//! The point of this file is breadth, as `tests/test_quality.rs` and
//! `tests/env_vars.rs` are for their rules -- but where those two reach for a
//! grammar and cover nine languages, this one needs no parser and so has to
//! answer for eighteen. A declaration written the way each language writes
//! one is what keeps a whole language from going quietly silent.
//!
//! Nothing here touches the network: the rule reads declaration syntax, not
//! trees, which is the trade it exists to make.

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
        .map(|finding| format!("{}:{}", finding.location.line, finding.matched))
        .collect()
}

/// One constant declaration per language, written the way that language
/// writes one: the label, the file, the source, and the name the finding must
/// carry. Each `source` declares exactly one constant, so exactly one finding
/// is correct in every row.
const CASES: &[(&str, &str, &str, &str)] = &[
    ("rust", "src/a.rs", "const MAX_SIZE: u8 = 3;\n", "MAX_SIZE"),
    (
        "rust-static",
        "src/b.rs",
        "static DEFAULT_PATH: &str = \"/tmp\";\n",
        "DEFAULT_PATH",
    ),
    ("c", "src/a.c", "#define MAX_RETRIES 5\n", "MAX_RETRIES"),
    (
        "cpp",
        "src/a.cc",
        "constexpr int MAX_BUFFER = 1024;\n",
        "MAX_BUFFER",
    ),
    (
        "c-sharp",
        "src/A.cs",
        "private const int MAX_ITEMS = 10;\n",
        "MAX_ITEMS",
    ),
    (
        "go",
        "src/a.go",
        "const (\n\tMAX_SIZE = 100\n)\n",
        "MAX_SIZE",
    ),
    (
        "java",
        "src/A.java",
        "public static final int MAX_SIZE = 3;\n",
        "MAX_SIZE",
    ),
    (
        "javascript",
        "src/a.js",
        "const MAX_SIZE = 3;\n",
        "MAX_SIZE",
    ),
    ("kotlin", "src/a.kt", "const val MAX_SIZE = 3\n", "MAX_SIZE"),
    ("php", "src/a.php", "const MAX_SIZE = 3;\n", "MAX_SIZE"),
    ("python", "src/a.py", "MAX_SIZE = 3\n", "MAX_SIZE"),
    ("ruby", "src/a.rb", "MAX_SIZE = 3\n", "MAX_SIZE"),
    ("scala", "src/a.scala", "val MAX_SIZE = 3\n", "MAX_SIZE"),
    ("shell", "src/a.sh", "MAX_SIZE=3\n", "MAX_SIZE"),
    ("swift", "src/a.swift", "let MAX_SIZE = 3\n", "MAX_SIZE"),
    (
        "typescript",
        "src/a.ts",
        "const MAX_SIZE: number = 3;\n",
        "MAX_SIZE",
    ),
    ("zig", "src/a.zig", "const MAX_SIZE = 3;\n", "MAX_SIZE"),
];

#[test]
fn every_language_reports_its_constant_declaration() {
    for (language, path, source, name) in CASES {
        let found = findings(path, source);
        assert_eq!(
            found.len(),
            1,
            "{language}: expected exactly one finding in {path}, got {found:?}"
        );
        assert!(
            found[0].ends_with(name),
            "{language}: the finding should name {name}, got {found:?}"
        );
    }
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
fn using_a_constant_everywhere_is_the_point_of_having_one() {
    let source = "fn f(n: u8) -> bool {\n    n > MAX_SIZE && n < OTHER_LIMIT\n}\n";

    assert_eq!(
        findings("src/main.rs", source),
        Vec::<String>::new(),
        "flagging uses would make the rule impossible to satisfy"
    );
}

#[test]
fn a_single_word_name_is_too_ambiguous_to_flag() {
    assert_eq!(
        findings("src/main.rs", "const MAX: u8 = 3;\nconst PI: f64 = 3.0;\n"),
        Vec::<String>::new()
    );
}

#[test]
fn a_declaration_that_is_not_code_is_not_a_declaration() {
    assert_eq!(
        findings("src/main.rs", "// const MAX_SIZE: u8 = 3;\n"),
        Vec::<String>::new(),
        "commented out"
    );
    assert_eq!(
        findings("src/main.rs", "let s = \"const MAX_SIZE = 3\";\n"),
        Vec::<String>::new(),
        "quoted in a string"
    );
}

#[test]
fn an_enum_member_is_not_a_constant_anyone_can_move() {
    assert_eq!(
        findings(
            "src/colour.py",
            "class Colour(Enum):\n    RED_ONE = 1\n    GREEN_TWO = 2\n"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        findings("src/colour.c", "enum Colour {\n    RED_ONE = 1,\n};\n"),
        Vec::<String>::new()
    );
}

#[test]
fn data_and_prose_files_are_not_this_rules_to_read() {
    assert_eq!(
        findings("config/a.yaml", "MAX_SIZE: 3\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        findings("data/a.json", "{\"MAX_SIZE\": 3}\n"),
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
