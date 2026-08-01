#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use infact_fact_pack::{FactPackCache, FactPackLock, FactPackManifest, build_oci_layout};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_straitjacket"))
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn install_pack(cache: &FactPackCache, name: &str) -> infact_fact_pack::CachedFactPack {
    let root = workspace().join("infact/infact-packs").join(name);
    let manifest =
        FactPackManifest::parse(&fs::read_to_string(root.join("pack.toml")).unwrap()).unwrap();
    let output = tempfile::tempdir().unwrap();
    let layout = output.path().join("layout");
    build_oci_layout(&manifest, &root, &layout).unwrap();
    cache.import_oci_layout(layout).unwrap()
}

fn build_pack_layout(name: &str, layout: &Path) {
    let root = workspace().join("infact/infact-packs").join(name);
    let manifest =
        FactPackManifest::parse(&fs::read_to_string(root.join("pack.toml")).unwrap()).unwrap();
    build_oci_layout(&manifest, &root, layout).unwrap();
}

fn write_fact_config(root: &Path, cache: &Path, lock: &Path) {
    fs::write(
        root.join("straitjacket.toml"),
        format!(
            "paths = [\"src\"]\n\n[facts]\ncache = {:?}\nlock = {:?}\nparser-paths = [{:?}]\ndependencies = \"none\"\n",
            cache,
            lock,
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();
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
    assert!(text.contains("leading 10 lines"));
    assert!(text.contains("rustdoc and JSDoc"));
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
    fs::write(&dockerfile, "FROM scratch\n# explanation\n").unwrap();

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

#[test]
fn locked_fact_pack_produces_a_library_opportunity_offline() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        "use std::collections::HashMap;\n\npub fn occurrence_counts(values: Vec<String>) -> HashMap<String, usize> {\n    let mut counts = HashMap::<String, usize>::new();\n    for value in values {\n        *counts.entry(value).or_default() += 1;\n    }\n    counts\n}\n",
    )
    .unwrap();
    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-itertools");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-itertools".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    write_fact_config(directory.path(), &cache_path, &lock_path);

    let status = binary()
        .args(["facts", "status"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8(status.stdout).unwrap().contains("cached"));
    let synchronized = binary()
        .args(["facts", "sync", "--offline"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        synchronized.status.success(),
        "{}",
        String::from_utf8_lossy(&synchronized.stderr)
    );

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[library-opportunity]"), "{stdout}");
    assert!(stdout.contains("itertools::Itertools::counts"), "{stdout}");

    let sarif_path = directory.path().join("facts.sarif");
    let machine = binary()
        .args(["--format", "json", "--no-fail", "--sarif"])
        .arg(&sarif_path)
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(machine.status.success());
    let json: serde_json::Value = serde_json::from_slice(&machine.stdout).unwrap();
    assert!(
        json.as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule"] == "library-opportunity")
    );
    let sarif: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sarif_path).unwrap()).unwrap();
    assert!(
        sarif["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["ruleId"] == "library-opportunity")
    );

    fs::write(
        directory.path().join("src/lib.rs"),
        "// straitjacket-allow-file:library-opportunity\nuse std::collections::HashMap;\n\npub fn occurrence_counts(values: Vec<String>) -> HashMap<String, usize> {\n    let mut counts = HashMap::<String, usize>::new();\n    for value in values {\n        *counts.entry(value).or_default() += 1;\n    }\n    counts\n}\n",
    )
    .unwrap();
    let suppressed = binary().current_dir(directory.path()).output().unwrap();
    assert!(
        suppressed.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&suppressed.stdout),
        String::from_utf8_lossy(&suppressed.stderr)
    );
}

#[test]
fn effect_capabilities_distinguish_providers_from_transitive_access() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src/adapters")).unwrap();
    fs::write(
        directory.path().join("src/adapters/filesystem.rs"),
        "pub fn load() { let _ = std::fs::read(\"input\"); }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("src/application.rs"),
        "pub fn service() { crate::adapters::filesystem::load(); }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("src/domain.rs"),
        "pub fn forbidden() { crate::application::service(); }\n",
    )
    .unwrap();

    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-core");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-core".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "paths=['src']\n\n[facts]\ncache={cache_path:?}\nlock={lock_path:?}\nparser-paths=[{:?}]\ndependencies='none'\n\n[effects]\nunlisted='deny'\nincomplete='error'\n\n[[effects.capabilities]]\nname='filesystem'\nincludes=['file-read', 'file-write']\nprovided-by=['src/adapters/**']\navailable-to=['src/application.rs']\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("[effect-capability]").count(), 1, "{stdout}");
    assert!(
        stdout.contains("src/domain.rs:1:1  [effect-capability]  filesystem"),
        "{stdout}"
    );
    assert!(
        stdout.contains("filesystem capability is not available to this path"),
        "{stdout}"
    );
    assert!(stdout.contains("std::fs::read"), "{stdout}");

    fs::write(
        directory.path().join("src/application.rs"),
        "pub fn misplaced() { let _ = std::fs::read(\"input\"); }\n",
    )
    .unwrap();
    fs::write(directory.path().join("src/domain.rs"), "pub fn pure() {}\n").unwrap();
    let direct = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(direct.status.code(), Some(1));
    let stdout = String::from_utf8(direct.stdout).unwrap();
    assert_eq!(stdout.matches("[effect-capability]").count(), 1, "{stdout}");
    assert!(
        stdout.contains("filesystem capability is not provided by this path"),
        "{stdout}"
    );
}

/// A barrier reaches through calls and files, and nothing else answers for it.
///
/// The allocation is two calls below the marker and in another file, so a rule
/// reading only the marked callable's own body would miss it. `cold.rs` carries
/// the same marker over a callable that reaches nothing, which has nothing to
/// report.
#[test]
fn an_effect_barrier_denies_what_a_marked_callable_reaches() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/support.rs"),
        "pub fn render() -> String {\n    format!(\"frame\")\n}\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("src/hot.rs"),
        "// straitjacket-barrier:hot-loop\npub fn tick() -> String {\n    crate::support::render()\n}\n", // straitjacket-allow:unknown-barrier — fixture
    )
    .unwrap();
    fs::write(
        directory.path().join("src/cold.rs"),
        "// straitjacket-barrier:hot-loop\npub fn idle() -> usize {\n    7\n}\n", // straitjacket-allow:unknown-barrier — fixture
    )
    .unwrap();

    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-core");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-core".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    let config = format!(
        "paths=['src']\n\n[facts]\ncache={cache_path:?}\nlock={lock_path:?}\nparser-paths=[{:?}]\ndependencies='none'\n\n[effects]\nunlisted='allow'\nincomplete='ignore'\n\n[[effects.barriers]]\nname='hot-loop'\ndenies=['allocate']\n",
        workspace().join("entl/parser-packs")
    );
    fs::write(directory.path().join("straitjacket.toml"), &config).unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.matches("[effect-barrier]").count(),
        1,
        "only the marked callable answers for it, not every caller on the way: {stdout}"
    );
    assert!(
        stdout.contains("src/support.rs:2:1  [effect-barrier]  hot-loop"),
        "{stdout}"
    );
    assert!(
        stdout.contains("allocate effect is denied below the hot-loop barrier"),
        "{stdout}"
    );

    fs::write(
        directory.path().join("src/hot.rs"),
        "pub fn tick() -> String {\n    crate::support::render()\n}\n",
    )
    .unwrap();
    let cleared = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(cleared.stdout).unwrap();
    assert!(
        !stdout.contains("[effect-barrier]"),
        "the marker is the barrier, so removing it removes the finding: {stdout}"
    );
    assert_eq!(cleared.status.code(), Some(0), "{stdout}");
}

/// A barrier that forbids nothing must say so rather than pass quietly.
#[test]
fn a_barrier_marker_naming_no_barrier_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/hot.rs"),
        "// straitjacket-barrier:hot-lop\npub fn tick() -> String {\n    format!(\"frame\")\n}\n", // straitjacket-allow:unknown-barrier — fixture
    )
    .unwrap();

    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-core");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-core".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "paths=['src']\n\n[facts]\ncache={cache_path:?}\nlock={lock_path:?}\nparser-paths=[{:?}]\ndependencies='none'\n\n[effects]\nunlisted='allow'\nincomplete='ignore'\n\n[[effects.barriers]]\nname='hot-loop'\ndenies=['allocate']\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("src/hot.rs:1:4  [unknown-barrier]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("`hot-lop` is not a configured effect barrier"),
        "{stdout}"
    );
    assert!(
        stdout.contains("name one of hot-loop"),
        "the help has to name what was meant: {stdout}"
    );
    assert!(
        !stdout.contains("[effect-barrier]"),
        "a misspelled marker forbids nothing, which is the whole problem: {stdout}"
    );

    fs::write(
        directory.path().join("src/hot.rs"),
        "// straitjacket-barrier:hot-loop\npub fn tick() -> String {\n    format!(\"frame\")\n}\n", // straitjacket-allow:unknown-barrier — fixture
    )
    .unwrap();
    let corrected = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(corrected.stdout).unwrap();
    assert!(!stdout.contains("[unknown-barrier]"), "{stdout}");
    assert!(
        stdout.contains("[effect-barrier]"),
        "spelled right, the barrier bites: {stdout}"
    );
}

#[test]
fn missing_locked_fact_pack_is_an_operational_error() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(directory.path().join("src/lib.rs"), "pub fn value() {}\n").unwrap();
    let source_cache = FactPackCache::open(directory.path().join("source-cache")).unwrap();
    let pack = install_pack(&source_cache, "rust-itertools");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-itertools".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    let missing_cache = directory.path().join("missing-cache");
    write_fact_config(directory.path(), &missing_cache, &lock_path);

    let output = binary()
        .args(["facts", "sync", "--offline"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not installed")
    );
}

#[test]
fn instructions_include_enabled_fact_backed_expectations() {
    let directory = tempfile::tempdir().unwrap();
    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-itertools");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-itertools".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "[facts]\ncache={cache_path:?}\nlock={lock_path:?}\ndependencies='none'\nexact-clones=true\nnear-clones=true\nclone-exclude=['tests/fixtures']\n\n[facts.exact]\nmin-tokens=24\nmin-lines=4\n\n[facts.near]\nmin-tokens=30\nmin-lines=5\nmax-changed-percent=12\n"
        ),
    )
    .unwrap();

    let output = binary()
        .arg("instructions")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("already depends on itertools"), "{text}");
    assert!(text.contains("exact clone of at least 24 tokens"));
    assert!(text.contains("near clone of at least 30 tokens"));
    assert!(text.contains("no more than 12% changed tokens"));
    assert!(text.contains("tests/fixtures"));
    assert!(text.contains("fact-backed check can complete"));
}

#[test]
fn instructions_describe_effect_capabilities() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        "[effects]\nunlisted='deny'\nincomplete='warn'\n\n[[effects.capabilities]]\nname='filesystem'\nincludes=['file-read', 'file-write']\nprovided-by=['src/adapters/filesystem/**']\navailable-to=['src/application/**']\n",
    )
    .unwrap();

    let output = binary()
        .arg("instructions")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("filesystem provides [file-read, file-write]"));
    assert!(text.contains("src/adapters/filesystem/**"));
    assert!(text.contains("src/application/**"));
    assert!(text.contains("Effects not assigned to a capability are denied"));
}

#[cfg(unix)]
#[test]
fn sync_can_build_a_missing_pack_with_an_explicit_local_builder() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source-layout");
    build_pack_layout("rust-itertools", &source);
    let builder = directory.path().join("builder.sh");
    fs::write(
        &builder,
        format!(
            "#!/bin/sh\nfor output do :; done\ncp -R {:?} \"$output\"\n",
            source
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&builder).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&builder, permissions).unwrap();
    let cache = directory.path().join("cache");
    let lock = directory.path().join("straitjacket.lock.toml");
    fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.0.0'\n\n[dependencies]\nitertools='0.15'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname='fixture'\nversion='0.0.0'\ndependencies=['itertools']\n\n[[package]]\nname='itertools'\nversion='0.15.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='0123456789abcdef'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "[facts]\ncache={cache:?}\nlock={lock:?}\nregistries=[]\nbuild-missing=true\n\n[[facts.builders]]\necosystem='cargo'\ncommand=[{builder:?}]\n"
        ),
    )
    .unwrap();

    let synchronized = binary()
        .args(["--config", "straitjacket.toml", "facts", "sync"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        synchronized.status.success(),
        "{}",
        String::from_utf8_lossy(&synchronized.stderr)
    );
    assert!(
        String::from_utf8(synchronized.stdout)
            .unwrap()
            .contains("rust-itertools")
    );
    let offline = binary()
        .args([
            "--config",
            "straitjacket.toml",
            "facts",
            "sync",
            "--offline",
        ])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(offline.status.success());
}

#[test]
fn exact_clone_can_be_suppressed_from_either_location() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    let source = "pub fn total(values: &[u32]) -> u32 {\n    let mut sum = 0;\n    for value in values {\n        sum += value;\n    }\n    sum\n}\n";
    fs::write(directory.path().join("src/a.rs"), source).unwrap();
    fs::write(directory.path().join("src/b.rs"), source).unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "paths = [\"src\"]\n\n[facts]\nparser-paths = [{:?}]\ndependencies = \"none\"\nexact-clones = true\n\n[facts.exact]\nmin-tokens = 8\nmin-lines = 2\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[exact-clone]"), "{stdout}");
    assert!(stdout.contains("related: src/b.rs"), "{stdout}");

    fs::write(
        directory.path().join("src/b.rs"),
        format!("// straitjacket-allow-file:exact-clone\n{source}"),
    )
    .unwrap();
    let suppressed = binary().current_dir(directory.path()).output().unwrap();
    assert!(
        suppressed.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&suppressed.stdout),
        String::from_utf8_lossy(&suppressed.stderr)
    );
}

#[test]
fn clone_exclusions_scope_repository_rules() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("fixtures")).unwrap();
    let source = "pub fn total(values: &[u32]) -> u32 {\n    let mut sum = 0;\n    for value in values {\n        sum += value;\n    }\n    sum\n}\n";
    fs::write(directory.path().join("fixtures/a.rs"), source).unwrap();
    fs::write(directory.path().join("fixtures/b.rs"), source).unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "[facts]\nparser-paths=[{:?}]\ndependencies='none'\nexact-clones=true\nclone-exclude=['fixtures']\n\n[facts.exact]\nmin-tokens=8\nmin-lines=2\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Resolved observations change what the scanner can see.
///
/// Both files reach `std::fs::read` through a module import, which syntax
/// resolution cannot follow. Only one of them is allowed to. Without
/// observations the scan is clean and that cleanliness means nothing; with
/// them the violation is found.
#[test]
fn observations_reveal_an_effect_that_syntax_resolution_cannot_see() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src/adapters")).unwrap();
    fs::write(
        directory.path().join("src/adapters/filesystem.rs"),
        "use std::fs;\n\npub fn load() -> Vec<u8> {\n    fs::read(\"input\").unwrap_or_default()\n}\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("src/domain.rs"),
        "use std::fs;\n\npub fn forbidden() -> Vec<u8> {\n    fs::read(\"secret\").unwrap_or_default()\n}\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        "pub mod adapters { pub mod filesystem; }\npub mod domain;\n",
    )
    .unwrap();

    let cache_path = directory.path().join("cache");
    let cache = FactPackCache::open(&cache_path).unwrap();
    let pack = install_pack(&cache, "rust-core");
    let lock_path = directory.path().join("straitjacket.lock.toml");
    let mut lock = FactPackLock::default();
    lock.insert(&pack, Some("fixture:rust-core".to_owned()))
        .unwrap();
    lock.write(&lock_path).unwrap();

    let observations = directory.path().join("observations");
    fs::create_dir_all(&observations).unwrap();
    fs::copy(
        workspace().join("straitjacket/tests/fixtures/observed-effects/observations.json"),
        observations.join("fixture.json"),
    )
    .unwrap();

    let config = |observations: Option<&Path>| {
        let line = observations
            .map(|path| format!("observations={path:?}\n"))
            .unwrap_or_default();
        format!(
            "paths=['src']\n\n[facts]\ncache={cache_path:?}\nlock={lock_path:?}\nparser-paths=[{:?}]\ndependencies='none'\n{line}\n[effects]\nunlisted='allow'\nincomplete='ignore'\n\n[[effects.capabilities]]\nname='filesystem'\nincludes=['file-read']\nprovided-by=['src/adapters/**']\navailable-to=['src/**']\n",
            workspace().join("entl/parser-packs")
        )
    };

    fs::write(directory.path().join("straitjacket.toml"), config(None)).unwrap();
    let syntax = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(syntax.stdout).unwrap();
    assert!(
        !stdout.contains("[effect-capability]"),
        "syntax should see nothing here, which is exactly the problem: {stdout}"
    );
    let stderr = String::from_utf8(syntax.stderr).unwrap();
    assert!(
        stderr.contains("resolved from syntax"),
        "syntax cannot follow `use std::fs; fs::read(..)`, and the run must say so, \
         or a clean report looks like a real answer: {stderr}"
    );

    fs::write(
        directory.path().join("straitjacket.toml"),
        config(Some(&observations)),
    )
    .unwrap();
    let observed = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(observed.stdout).unwrap();
    assert_eq!(observed.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("src/domain.rs") && stdout.contains("[effect-capability]"),
        "the disallowed read should be reported: {stdout}"
    );
    assert!(
        stdout.contains("filesystem capability is not provided by this path"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("src/adapters/filesystem.rs"),
        "the adapter provides the capability, so doing exactly this is allowed: {stdout}"
    );
    let stderr = String::from_utf8(observed.stderr).unwrap();
    assert!(
        !stderr.contains("resolved from syntax"),
        "observations were used, so nothing should be disclaimed: {stderr}"
    );
}

/// A failure that cannot reach a caller is the thing this rule exists to find.
///
/// The fixture pairs each discarding form with a handled equivalent, so a pass
/// means the rule separated them rather than flagging everything fallible.
#[test]
fn error_discard_reports_failures_that_cannot_reach_a_caller() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        "fn write_observations(data: &[u8]) {\n    \
             let _ = std::fs::write(\"out.json\", data);\n}\n\n\
         fn load(path: &str) -> Option<String> {\n    \
             std::fs::read_to_string(path).ok()\n}\n\n\
         fn propagates(path: &str) -> std::io::Result<String> {\n    \
             std::fs::read_to_string(path)\n}\n\n\
         fn handles(path: &str) {\n    \
             match std::fs::read_to_string(path) {\n        \
                 Ok(text) => println!(\"{text}\"),\n        \
                 Err(error) => eprintln!(\"{error}\"),\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "paths = [\"src\"]\nonly = [\"error-discard\"]\n\n[facts]\nparser-paths = [{:?}]\ndependencies = \"none\"\n\n[errors]\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");

    assert!(
        stdout.contains("src/lib.rs:2:5") && stdout.contains("let-underscore"),
        "the discard that cannot leave its callable at all: {stdout}"
    );
    assert!(
        stdout.contains("write_observations returns no error type"),
        "the report has to say why the error had nowhere to go: {stdout}"
    );
    assert!(
        stdout.contains("src/lib.rs:6:5") && stdout.contains("ok-discard"),
        "the discard that downgrades a cause into an absence: {stdout}"
    );

    assert!(
        !stdout.contains("src/lib.rs:10") && !stdout.contains("src/lib.rs:14"),
        "propagated and handled errors must not be reported: {stdout}"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("2 error(s)"), "{stderr}");
}

/// `.unwrap_or_default()` reads the same on `Option`, so it is opt-in.
#[test]
fn error_discard_leaves_ambiguous_forms_to_the_repository() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        "fn render(value: &Value) -> String {\n    \
             serde_json::to_string(value).unwrap_or_default()\n}\n",
    )
    .unwrap();
    let config = |body: &str| {
        format!(
            "paths = [\"src\"]\nonly = [\"error-discard\"]\n\n[facts]\nparser-paths = [{:?}]\ndependencies = \"none\"\n\n[errors]\n{body}",
            workspace().join("entl/parser-packs")
        )
    };

    fs::write(directory.path().join("straitjacket.toml"), config("")).unwrap();
    let default = binary().current_dir(directory.path()).output().unwrap();
    assert_eq!(
        default.status.code(),
        Some(0),
        "the default keeps quiet, because syntax cannot prove this is a Result"
    );

    fs::write(
        directory.path().join("straitjacket.toml"),
        config("deny = [\"unwrap-or\"]\nambiguous = \"warn\"\n"),
    )
    .unwrap();
    let asked = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(asked.stdout).unwrap();
    let stderr = String::from_utf8(asked.stderr).unwrap();
    assert!(stdout.contains("unwrap-or"), "{stdout}");
    assert!(stderr.contains("1 warning(s)"), "{stderr}");
}

/// The local signature cannot say whether ANY caller could have been told.
///
/// The two discards here are identical in form and both sit in an infallible
/// callable. Only the call graph separates them, so a pass means reach was
/// resolved rather than guessed from the signature.
#[test]
fn error_discard_traces_a_failure_up_to_the_callers() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        "fn run() -> Result<(), Error> {\n    analyze();\n    Ok(())\n}\n\n\
         fn analyze() {\n    reported();\n}\n\n\
         fn reported() {\n    let _ = std::fs::write(\"a\", b\"x\");\n}\n\n\
         fn top() {\n    middle();\n}\n\n\
         fn middle() {\n    sealed();\n}\n\n\
         fn sealed() {\n    let _ = std::fs::write(\"b\", b\"y\");\n}\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("straitjacket.toml"),
        format!(
            "paths = [\"src\"]\nonly = [\"error-discard\"]\n\n[facts]\nparser-paths = [{:?}]\ndependencies = \"none\"\n\n[errors]\n",
            workspace().join("entl/parser-packs")
        ),
    )
    .unwrap();

    let output = binary().current_dir(directory.path()).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");

    assert!(
        stdout.contains("crate::reported cannot report it, but crate::run above returns `Result`"),
        "a fallible ancestor exists, so the fix is to propagate to it: {stdout}"
    );
    assert!(
        stdout.contains("no caller from crate::top down to crate::sealed can report a failure"),
        "the whole chain is infallible, so no caller can ever learn of it: {stdout}"
    );
    assert!(
        stdout.contains("crate::run calls crate::analyze")
            && stdout.contains("crate::middle calls crate::sealed"),
        "the trace names the calls that carried it: {stdout}"
    );
}
