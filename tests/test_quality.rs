//! `test-quality` against every language treebank publishes a pack for.
//!
//! The point of this file is breadth. The rule reads a test the way each
//! language actually writes one -- an attribute in Rust, an annotation in
//! Java, a callback in TypeScript, a macro in C++, a declaration in Zig --
//! and every one of those was silent before, in the way a clean suite is
//! silent. One case per language is what keeps that from happening again
//! quietly.
//!
//! These fetch real packs, for the reason `tests/pack_host.rs` gives at
//! length: a test that passes because it found no grammar is worse than no
//! test.

use straitjacket::config::Settings;
use straitjacket::scanner::Scanner;

/// A scanner with `test-quality` on. It is opt-in, so every test here has to
/// ask for it, which is also the documentation of how a user turns it on.
fn scanner(test_rules: Vec<String>) -> Scanner {
    let settings = Settings {
        only: vec!["test-quality".to_string()],
        test_rules,
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

/// One loop-in-a-test per language, written the way that language writes one.
///
/// Each `source` has a loop inside a test and nothing else wrong with it, so
/// exactly one finding is correct in every row.
const CASES: &[(&str, &str, &str)] = &[
    (
        "python",
        "tests/test_forum.py",
        "def test_registers_every_user():\n    for user in users:\n        forum.register(user)\n",
    ),
    (
        "rust",
        "src/forum.rs",
        "#[test]\nfn registers_every_user() {\n    for user in users {\n        forum.register(user);\n    }\n}\n",
    ),
    (
        "java",
        "src/test/java/ForumTest.java",
        "class ForumTest {\n  @Test\n  public void registersEveryUser() {\n    for (User user : users) {\n      forum.register(user);\n    }\n  }\n}\n",
    ),
    (
        "typescript",
        "src/forum.test.ts",
        "it('registers every user', () => {\n  for (const user of users) {\n    forum.register(user);\n  }\n});\n",
    ),
    (
        "javascript",
        "src/forum.test.js",
        "it('registers every user', () => {\n  for (const user of users) {\n    forum.register(user);\n  }\n});\n",
    ),
    (
        "ruby",
        "spec/forum_spec.rb",
        "it 'registers every user' do\n  users.each do |user|\n    forum.register(user)\n  end\nend\n",
    ),
    (
        "cpp",
        "test/forum_test.cc",
        "TEST(Forum, RegistersEveryUser) {\n  for (auto user : users) {\n    forum.Register(user);\n  }\n}\n",
    ),
    (
        "c",
        "test/forum_test.c",
        "void test_registers_every_user(void) {\n  for (int i = 0; i < n; i++) {\n    forum_register(users[i]);\n  }\n}\n",
    ),
    (
        "shell",
        "tests/forum_test.sh",
        "test_registers_every_user() {\n  for user in $users; do\n    register \"$user\"\n  done\n}\n",
    ),
    (
        "zig",
        "src/forum.zig",
        "test \"registers every user\" {\n    for (users) |user| {\n        forum.register(user);\n    }\n}\n",
    ),
];

#[test]
fn a_loop_in_a_test_is_found_in_every_language_with_a_pack() {
    let mut silent = Vec::new();
    for (language, path, source) in CASES {
        let found = findings(path, source);
        if found.len() != 1 {
            silent.push(format!(
                "{language} ({path}): expected 1 finding, got {found:?}"
            ));
            continue;
        }
        assert!(
            found[0].contains("test-logic"),
            "{language}: {:?} is not the rule that should have fired",
            found[0]
        );
    }
    assert!(silent.is_empty(), "{}", silent.join("\n"));
}

#[test]
fn a_test_that_states_its_cases_directly_is_left_alone() {
    let clean: &[(&str, &str)] = &[
        (
            "tests/test_forum.py",
            "def test_registers_alice():\n    forum.register(alice)\n    assert forum.has(alice)\n",
        ),
        (
            "src/forum.rs",
            "#[test]\nfn registers_alice() {\n    forum.register(alice);\n    assert!(forum.has(alice));\n}\n",
        ),
        (
            "src/forum.test.ts",
            "it('registers alice', () => {\n  forum.register(alice);\n  expect(forum.has(alice)).toBe(true);\n});\n",
        ),
        (
            "spec/forum_spec.rb",
            "it 'registers alice' do\n  forum.register(alice)\n  expect(forum.has?(alice)).to be true\nend\n",
        ),
    ];
    for (path, source) in clean {
        assert!(
            findings(path, source).is_empty(),
            "{path} is a clean test and should yield nothing"
        );
    }
}

/// The reason this rule exists at all: production code with the same loop in
/// it is not a test and must stay silent.
#[test]
fn a_loop_outside_a_test_is_not_a_finding() {
    let source =
        "def register_every_user():\n    for user in users:\n        forum.register(user)\n";
    assert!(findings("src/forum.py", source).is_empty());
}

/// A suite is not a test. A loop in `describe` generates cases, and flagging
/// it would fire on the normal way to write a parameterised suite.
#[test]
fn a_loop_that_generates_cases_in_a_suite_is_not_a_finding() {
    let source = "describe('forum', () => {\n  for (const user of users) {\n    it('registers ' + user, () => { expect(true).toBe(true); });\n  }\n});\n";
    assert!(findings("src/forum.test.ts", source).is_empty());
}

/// Rust and Zig put tests inside ordinary source files, so a prefilter that
/// only reads the path would make this rule silent for both.
#[test]
fn an_inline_test_in_a_file_no_path_rule_would_pick_is_still_read() {
    let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[test]\nfn adds_each() {\n    for n in [1, 2] {\n        assert_eq!(add(n, 0), n);\n    }\n}\n";
    let found = findings("src/arithmetic.rs", source);
    assert_eq!(found.len(), 1, "an inline #[test] was not read: {found:?}");
}

#[test]
fn a_named_selection_runs_only_that_rule() {
    let source =
        "def test_registers_every_user():\n    for user in users:\n        forum.register(user)\n";
    let extension = "py";

    let all = scanner(Vec::new())
        .scan(source, "tests/test_forum.py", extension)
        .findings;
    assert_eq!(all.len(), 1, "the default selection runs every rule");

    let chosen = scanner(vec!["test-logic".to_string()])
        .scan(source, "tests/test_forum.py", extension)
        .findings;
    assert_eq!(chosen.len(), 1, "naming the rule that fires keeps it");
}

/// A typo in `test-rules` would otherwise turn a rule off silently, which is
/// the exact failure this whole change is about.
#[test]
fn an_unknown_test_rule_is_refused_by_name() {
    let settings = Settings {
        test_rules: vec!["test-logick".to_string()],
        ..Settings::default()
    };
    let error = Scanner::new(&settings)
        .err()
        .expect("an unknown test rule is refused")
        .to_string();
    assert!(
        error.contains("test-logick") && error.contains("test-logic"),
        "the error should name the typo and what was meant: {error}"
    );
}

/// Off unless asked for: the first run downloads a grammar, and a scan that
/// reaches the network because the tool was upgraded is not a surprise anyone
/// should get for free.
#[test]
fn the_rule_is_opt_in() {
    let scanner = Scanner::new(&Settings::default()).expect("the scanner builds");
    let descriptor = scanner
        .descriptors()
        .into_iter()
        .find(|descriptor| descriptor.id.as_str() == "test-quality")
        .expect("test-quality is registered");
    assert!(!descriptor.default_enabled);
}

/// Findings go through the same suppression as every other rule, which is the
/// second of beamte's four host obligations: an exception has to be
/// declarable, and declaring one has to state a reason.
#[test]
fn a_finding_can_be_suppressed_like_any_other() {
    let source = "#[test]\nfn adds_each() {\n    for n in xs { // straitjacket-allow:test-quality — table-driven by design\n        assert_eq!(add(n), n);\n    }\n}\n";
    assert!(findings("src/arithmetic.rs", source).is_empty());
}

/// The third obligation: a finding names the post it came from, so the
/// argument is with Titus Winters rather than with a linter.
#[test]
fn a_finding_cites_the_post_it_restates() {
    let settings = Settings {
        only: vec!["test-quality".to_string()],
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).expect("the scanner builds");
    let source =
        "def test_registers_every_user():\n    for user in users:\n        forum.register(user)\n";
    let found = scanner.scan(source, "tests/test_forum.py", "py").findings;
    let help = found
        .first()
        .and_then(|finding| finding.help.clone())
        .expect("a finding with help");
    assert!(
        help.contains("testing.googleblog.com"),
        "the citation is missing from {help:?}"
    );
}
