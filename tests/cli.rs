use std::fs;
use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_straitjacket"))
}

#[test]
fn findings_fail_and_no_fail_overrides_the_gate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bad.css");
    fs::write(&path, "a { color: #abcdef; }\n").unwrap();

    let failed = binary()
        .args(["--no-config", "--only", "color"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    assert!(
        String::from_utf8(failed.stdout)
            .unwrap()
            .contains("[color]")
    );

    let reported = binary()
        .args(["--no-config", "--only", "color", "--no-fail"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(reported.status.success());
}

#[test]
fn json_and_sarif_are_valid_machine_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("bad.css");
    let sarif = directory.path().join("report.sarif");
    fs::write(&source, "a { color: #abcdef; }\n").unwrap();

    let output = binary()
        .args([
            "--no-config",
            "--only",
            "color",
            "--format",
            "json",
            "--no-fail",
            "--sarif",
        ])
        .arg(&sarif)
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["rule"], "color");
    let sarif: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sarif).unwrap()).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
}

#[test]
fn unknown_rule_is_an_operational_error() {
    let output = binary()
        .args(["--no-config", "--only", "not-a-rule"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unknown rule")
    );
}

#[test]
fn instructions_describe_the_repository_config() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        "no-comments = true\nmax-lines = 90\nfile-size-exclude = [\"notes/\"]\nskip = [\"emoji\", \"motion\"]\n",
    )
    .unwrap();

    let output = binary()
        .arg("instructions")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Straitjacket repository policy"));
    assert!(text.contains("Files over 90 lines"));
    assert!(text.contains("outside notes/"));
    assert!(text.contains("Source comments are not allowed"));
    assert!(!text.contains("emoji"));
    assert!(!text.contains("motion"));
}

#[test]
fn extensionless_scripts_use_entl_language_detection() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("release");
    fs::write(
        &script,
        "#!/usr/bin/env python3\nprint('shipping 🚀')\n", // straitjacket-allow:emoji — fixture
    )
    .unwrap();

    let output = binary()
        .args(["--no-config", "--only", "emoji", "--no-fail"])
        .arg(&script)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("[emoji]")
    );
}

#[test]
fn filename_detected_languages_reach_rules_without_an_extension() {
    let directory = tempfile::tempdir().unwrap();
    let dockerfile = directory.path().join("Dockerfile");
    fs::write(&dockerfile, "# explanation\nFROM scratch\n").unwrap();

    let output = binary()
        .args(["--no-config", "--only", "no-comments", "--no-fail"])
        .arg(&dockerfile)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("[no-comments]")
    );
}

#[test]
fn global_git_ignores_do_not_change_scan_results() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("bad.css");
    let excludes = directory.path().join("global-ignore");
    let git_config = directory.path().join("global-gitconfig");
    fs::write(&source, "a { color: #abcdef; }\n").unwrap();
    fs::write(&excludes, "bad.css\n").unwrap();
    fs::write(
        &git_config,
        format!("[core]\n\texcludesFile = {}\n", excludes.display()),
    )
    .unwrap();

    let output = binary()
        .env("GIT_CONFIG_GLOBAL", git_config)
        .args(["--no-config", "--only", "color"])
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("[color]")
    );
}

#[test]
fn directory_scans_inherit_parent_ignores() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("nested")).unwrap();
    fs::write(directory.path().join(".gitignore"), "ignored.css\n").unwrap();
    fs::write(
        directory.path().join("nested/ignored.css"),
        "a { color: #abcdef; }\n",
    )
    .unwrap();

    let ignored = binary()
        .args(["--no-config", "--only", "color", "nested"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(ignored.status.success());

    let included = binary()
        .args(["--no-config", "--only", "color", "--no-ignore", "nested"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert_eq!(included.status.code(), Some(1));
    assert!(
        String::from_utf8(included.stdout)
            .unwrap()
            .contains("[color]")
    );
}
