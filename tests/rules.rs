#![allow(clippy::unwrap_used, clippy::expect_used)]
use straitjacket::{RuleKey, Scanner, Settings};

fn scanner(only: &[&str]) -> Scanner {
    let settings = Settings {
        only: only.iter().map(|rule| (*rule).to_string()).collect(),
        ..Settings::default()
    };
    Scanner::new(&settings).expect("scanner builds")
}

fn rules(source: &str, extension: &str, only: &[&str]) -> Vec<String> {
    scanner(only)
        .scan(source, "test", extension)
        .findings
        .into_iter()
        .map(|finding| finding.rule.to_string())
        .collect()
}

#[test]
fn emoji_is_unicode_aware_without_flagging_plain_symbols() {
    assert_eq!(
        rules("let status = \"done 🚀\";", "rs", &["emoji"]), // straitjacket-allow:emoji — fixture
        ["emoji"]
    );
    assert!(rules("© ™ ✓ → ★", "md", &["emoji"]).is_empty());
}

#[test]
fn color_covers_hex_and_css_functions() {
    assert_eq!(
        rules(
            "a { color: #abc; background: oklch(1 0 0); }",
            "css",
            &["color"]
        ),
        ["color", "color"]
    );
    assert!(rules("const hash = '#abcdefgh';", "ts", &["color"]).is_empty());
}

#[test]
fn color_allows_configured_theme_files() {
    let settings = Settings {
        only: vec!["color".into()],
        theme_files: vec!["src/theme.css".into()],
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).unwrap();
    assert!(
        scanner
            .scan("--brand: #abc;", "src/theme.css", "css")
            .findings
            .is_empty()
    );
    assert_eq!(
        scanner
            .scan("color: #abc;", "src/card.css", "css")
            .findings
            .len(),
        1
    );
}

#[test]
fn inline_svg_is_limited_to_component_sources() {
    assert_eq!(
        rules("const Icon = () => <svg/>;", "tsx", &["inline-svg"]),
        ["inline-svg"]
    );
    assert!(rules("<svg></svg>", "html", &["inline-svg"]).is_empty());
}

#[test]
fn inline_font_allows_tokens_and_flags_literal_stacks() {
    assert!(rules("const x = { fontFamily: MONO };", "ts", &["inline-font"]).is_empty());
    assert!(rules("font-family: var(--body-font);", "css", &["inline-font"]).is_empty());
    assert_eq!(
        rules("font-family: Inter, sans-serif;", "css", &["inline-font"]),
        ["inline-font"]
    );
}

#[test]
fn motion_flags_declarations_but_not_property_reads() {
    assert_eq!(
        rules("transition: opacity 100ms;", "css", &["motion"]),
        ["motion"]
    );
    assert!(rules("const value = style.transition;", "ts", &["motion"]).is_empty());
}

#[test]
fn file_size_requires_a_file_marker() {
    let settings = Settings {
        only: vec!["file-size".into()],
        max_lines: 2,
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).unwrap();
    assert_eq!(scanner.scan("a\nb\nc\n", "test.rs", "rs").findings.len(), 1);
    assert_eq!(
        scanner
            .scan(
                "// straitjacket-allow-file:file-size — generated\na\nb\nc\n",
                "test.rs",
                "rs"
            )
            .suppressed,
        1
    );
    let line_marker = scanner.scan("a\nb\nc // straitjacket-allow:file-size\n", "test.rs", "rs");
    assert!(
        line_marker
            .findings
            .iter()
            .any(|finding| finding.rule == RuleKey::new("file-size"))
    );
}

#[test]
fn file_size_honors_configured_path_exclusions() {
    let settings = Settings {
        only: vec!["file-size".into()],
        max_lines: 2,
        file_size_exclude: vec!["notes/".into()],
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).unwrap();
    assert!(
        scanner
            .scan("a\nb\nc\n", "notes/design.md", "md")
            .findings
            .is_empty()
    );
    assert_eq!(
        scanner
            .scan("a\nb\nc\n", "src/design.md", "md")
            .findings
            .len(),
        1
    );
}

#[test]
fn deep_nesting_reports_once_per_over_budget_run() {
    let settings = Settings {
        only: vec!["deep-nesting".into()],
        max_nesting: 2,
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).unwrap();
    let result = scanner.scan("a\n  b\n    c\n      d\n      e\nf\n", "test.rs", "rs");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].location.line, 4);
}

#[test]
fn no_comments_is_opt_in_and_tracks_strings() {
    assert!(
        Scanner::new(&Settings::default())
            .unwrap()
            .scan("// note\n", "test.ts", "ts")
            .findings
            .iter()
            .all(|finding| finding.rule != RuleKey::new("no-comments"))
    );
    assert_eq!(
        rules(
            "const url = \"https://example.test\"; // explanation\n",
            "ts",
            &["no-comments"]
        ),
        ["no-comments"]
    );
}

#[test]
fn suppression_is_scoped_and_dead_markers_fail() {
    let scanner = scanner(&["color", "motion"]);
    let result = scanner.scan(
        "a { color: #abc; transition: all 1s; } // straitjacket-allow:color\n",
        "test.css",
        "css",
    );
    assert_eq!(result.suppressed, 1);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].rule, RuleKey::new("motion"));

    let dead = scanner.scan(
        "a { color: var(--fg); } /* straitjacket-allow:color */\n",
        "test.css",
        "css",
    );
    assert_eq!(dead.findings[0].rule, RuleKey::new("unused-marker"));
}

#[test]
fn json_is_skipped_unless_explicitly_included() {
    let settings = Settings {
        only: vec!["file-size".into()],
        max_lines: 1,
        ..Settings::default()
    };
    let scanner = Scanner::new(&settings).unwrap();
    assert!(!scanner.handles_extension("json"));
    assert!(!scanner.handles_extension("jsonc"));
    let included = Settings {
        include_json: true,
        ..settings
    };
    let scanner = Scanner::new(&included).unwrap();
    assert!(scanner.handles_extension("json"));
    assert!(scanner.handles_extension("jsonc"));
}

#[test]
fn rule_policy_uses_entl_profile_facts() {
    assert!(scanner(&["emoji"]).handles_extension("mts"));
    assert!(scanner(&["emoji"]).handles_extension("cts"));

    assert!(scanner(&["deep-nesting"]).handles_extension("scala"));
    assert!(scanner(&["file-size"]).handles_extension("scala"));
    assert!(scanner(&["emoji"]).handles_extension("scala"));

    assert!(scanner(&["file-size"]).handles_extension("toml"));
    assert!(!scanner(&["emoji"]).handles_extension("toml"));

    assert!(scanner(&["color"]).handles_extension("html"));
    assert!(scanner(&["emoji"]).handles_extension("html"));
    assert!(!scanner(&["inline-svg"]).handles_extension("html"));
}
