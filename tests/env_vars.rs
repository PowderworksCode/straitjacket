//! `env-vars` against every language beamte's `env-read` covers.
//!
//! The point of this file is breadth, exactly as `tests/test_quality.rs`: the
//! rule reads an environment read the way each language writes one --
//! `std::env::var` in Rust, `os.environ` in Python, `process.env` in
//! TypeScript, `ENV[...]` in Ruby, `System.getenv` in Java, `getenv` in C --
//! and one case per language is what keeps a whole language from going
//! silent quietly.
//!
//! These fetch real packs, for the reason `tests/pack_host.rs` gives at
//! length: a test that passes because it found no grammar is worse than no
//! test.

use straitjacket::config::Settings;
use straitjacket::finding::Severity;
use straitjacket::scanner::Scanner;

/// A scanner with `env-vars` on, the way a configuration turns it on.
fn scanner(env_files: Vec<std::path::PathBuf>) -> Scanner {
    let settings = Settings {
        env_vars: true,
        env_files,
        ..Settings::default()
    };
    Scanner::new(&settings).expect("the scanner builds")
}

fn findings(path: &str, source: &str) -> Vec<String> {
    let extension = path.rsplit('.').next().unwrap_or("");
    scanner(Vec::new())
        .scan(source, path, extension)
        .findings
        .into_iter()
        .map(|finding| {
            format!(
                "{}:{} {}",
                finding.location.line, finding.rule, finding.message
            )
        })
        .collect()
}

/// One environment read per language, written the way that language writes
/// one. Each `source` reads exactly one variable and does nothing else
/// wrong, so exactly one finding is correct in every row.
const CASES: &[(&str, &str, &str)] = &[
    (
        "python",
        "src/loader.py",
        "def load():\n    return os.getenv(\"HOME\")\n",
    ),
    (
        "rust",
        "src/loader.rs",
        "fn load() -> Option<String> {\n    std::env::var(\"HOME\").ok()\n}\n",
    ),
    (
        "typescript",
        "src/loader.ts",
        "export function load(): string | undefined {\n  return process.env.HOME;\n}\n",
    ),
    (
        "javascript",
        "src/loader.js",
        "function load() {\n  return process.env.HOME;\n}\n",
    ),
    (
        "ruby",
        "lib/loader.rb",
        "def load_home\n  ENV[\"HOME\"]\nend\n",
    ),
    (
        "java",
        "src/Loader.java",
        "class Loader {\n  String load() {\n    return System.getenv(\"HOME\");\n  }\n}\n",
    ),
    (
        "c",
        "src/loader.c",
        "#include <stdlib.h>\nconst char *load(void) {\n  return getenv(\"HOME\");\n}\n",
    ),
    (
        "cpp",
        "src/loader.cc",
        "#include <cstdlib>\nconst char *load() {\n  return std::getenv(\"HOME\");\n}\n",
    ),
    (
        "zig",
        "src/loader.zig",
        "const std = @import(\"std\");\nfn load() ?[]const u8 {\n    return std.posix.getenv(\"HOME\");\n}\n",
    ),
];

#[test]
fn every_language_reports_its_environment_read() {
    for (language, path, source) in CASES {
        let found = findings(path, source);
        assert_eq!(
            found.len(),
            1,
            "{language}: expected exactly one finding in {path}, got {found:?}"
        );
        assert!(
            found[0].contains("env-read"),
            "{language}: the finding should carry beamte's rule id, got {found:?}"
        );
        assert!(
            found[0].contains("HOME"),
            "{language}: the finding should name the variable, got {found:?}"
        );
    }
}

#[test]
fn a_read_is_an_error_and_cites_the_post() {
    let result = scanner(Vec::new()).scan(
        "fn load() -> Option<String> {\n    std::env::var(\"HOME\").ok()\n}\n",
        "src/loader.rs",
        "rs",
    );

    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Error);
    let help = result.findings[0].help.as_deref().unwrap_or_default();
    assert!(
        help.contains("testing.googleblog.com"),
        "the help should cite the post, got: {help}"
    );
}

#[test]
fn the_declared_edge_is_licensed_and_everything_else_is_not() {
    let scanner = scanner(vec!["src/config.rs".into()]);
    let source = "fn load() -> Option<String> {\n    std::env::var(\"HOME\").ok()\n}\n";

    let licensed = scanner.scan(source, "src/config.rs", "rs");
    assert_eq!(
        licensed.findings,
        Vec::new(),
        "the declared edge may read the environment"
    );

    let unlicensed = scanner.scan(source, "src/main.rs", "rs");
    assert_eq!(unlicensed.findings.len(), 1);
}

/// The read sits inside a test. `env-vars` reports it -- a test that reads
/// the environment is exactly the non-hermetic test the citation describes --
/// and `test-quality` does not, because the file-scoped rule is env-vars' to
/// run and one read should not become two findings.
///
/// The read is a plain call rather than a macro argument: a Rust macro's
/// arguments are lexed as a token tree, not parsed, so a read inside
/// `assert!(...)` carries no invocation node for beamte to match -- a miss
/// beamte's rule documents by name.
#[test]
fn a_test_file_is_read_by_env_vars_and_not_by_test_quality() {
    let source = "#[test]\nfn test_home() {\n    let home = std::env::var(\"HOME\");\n    home.unwrap();\n}\n";

    let by_env = findings("tests/home.rs", source);
    assert_eq!(by_env.len(), 1, "env-vars reads test files too: {by_env:?}");

    let settings = Settings {
        only: vec!["test-quality".to_string()],
        ..Settings::default()
    };
    let by_tests =
        Scanner::new(&settings)
            .expect("the scanner builds")
            .scan(source, "tests/home.rs", "rs");
    assert_eq!(
        by_tests.findings,
        Vec::new(),
        "test-quality holds the file-scoped rule out of its default selection"
    );
}

#[test]
fn a_shell_file_is_not_this_rules_to_read() {
    let found = findings("scripts/release.sh", "#!/bin/sh\necho \"$HOME\"\n");
    assert_eq!(
        found,
        Vec::<String>::new(),
        "every expansion in shell is an environment read; flagging the \
         language is not a rule"
    );
}

#[test]
fn naming_the_file_scoped_rule_in_test_rules_is_refused_with_directions() {
    let settings = Settings {
        test_rules: vec!["env-read".to_string()],
        ..Settings::default()
    };

    let error = match Scanner::new(&settings) {
        Ok(_) => panic!("env-read is not a test rule and should be refused"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("env-vars"),
        "the error should point at the rule that runs it: {error}"
    );
}

#[test]
fn a_mention_in_a_comment_is_not_a_read() {
    let found = findings(
        "src/loader.rs",
        "// std::env::var(\"HOME\") would be wrong here\nfn load() {}\n",
    );
    assert_eq!(found, Vec::<String>::new());
}
