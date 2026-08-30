use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

use straitjacket::config::{self, Settings};
use straitjacket::finding::Severity;
use straitjacket::instructions;
use straitjacket::language::LanguageProfile;
use straitjacket::manifest::Manifest;
use straitjacket::report::{self, OutputFormat};
use straitjacket::walk::{SourceFileEntry, WalkOptions, walk};
use straitjacket::{Finding, PendingFileScan, PendingScan, Scanner};

struct PreparedFile {
    text: String,
    path: String,
    language: &'static LanguageProfile,
    pending: PendingScan,
}

#[derive(Debug, Parser)]
#[command(
    name = "straitjacket",
    version,
    about = "Flag the weird code and text LLMs produce"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(help = "Files or directories to scan. Defaults to configuration paths, then `.`")]
    paths: Vec<PathBuf>,

    #[arg(long, value_enum, help = "Output format written to stdout")]
    format: Option<OutputFormat>,

    #[arg(long, value_delimiter = ',', help = "Run only these rules")]
    only: Vec<String>,

    #[arg(long, value_delimiter = ',', help = "Disable these rules")]
    skip: Vec<String>,

    #[arg(long, help = "Maximum lines per file. Zero disables `file-size`")]
    max_lines: Option<usize>,

    #[arg(long, help = "Maximum indentation depth. Zero disables `deep-nesting`")]
    max_nesting: Option<usize>,

    #[arg(long, help = "Enable the opt-in `no-comments` rule")]
    no_comments: bool,
    #[arg(long, help = "Enable the opt-in `test-quality` rule")]
    test_quality: bool,

    #[arg(long, help = "Scan JSON files, which are skipped by default")]
    include_json: bool,

    #[arg(long, help = "Ignore .gitignore, .ignore, and hidden-file conventions")]
    no_ignore: bool,

    #[arg(long, help = "Report findings without failing the process")]
    no_fail: bool,

    #[arg(long, help = "Do not report suppression markers that suppress nothing")]
    no_fail_on_unused_markers: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write a SARIF report to this path in addition to stdout"
    )]
    sarif: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Use this configuration file instead of discovering one"
    )]
    config: Option<PathBuf>,

    #[arg(long, help = "Ignore checked-in configuration")]
    no_config: bool,

    #[arg(
        long,
        help = "List all known rules and exit. `--format json` emits the rule manifest"
    )]
    list_rules: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Print the active repository policy for agents and contributors")]
    Instructions,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("straitjacket: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let settings = resolve(&cli)?;
    let scanner = Scanner::new(&settings)?;

    if let Some(Command::Instructions) = &cli.command {
        print!(
            "{}",
            instructions::render(&settings, &scanner.enabled_descriptors())?
        );
        return Ok(ExitCode::SUCCESS);
    }

    if cli.list_rules {
        let manifest = Manifest::build(&scanner.descriptors(), &settings);
        match settings.format {
            OutputFormat::Text => print!("{}", manifest.to_text()),
            OutputFormat::Json => print!("{}", manifest.to_json()?),
            OutputFormat::Sarif => anyhow::bail!(
                "--list-rules supports `--format text` and `--format json`; SARIF describes findings, not rules"
            ),
        }
        return Ok(ExitCode::SUCCESS);
    }

    let mut findings = Vec::<Finding>::new();
    let mut scanned = 0usize;
    let mut suppressed = 0usize;
    let mut seen = BTreeSet::new();
    let options = WalkOptions {
        respect_ignore_files: !settings.no_ignore,
        include_hidden: settings.no_ignore,
    };
    for requested in &settings.paths {
        let (root, selected_file) = scan_root(requested)?;
        let files = walk(&root, &options)?;
        reject_unscannable_file(&files, selected_file.as_deref(), requested)?;
        let mut prepared = Vec::new();
        for file in files {
            if selected_file
                .as_ref()
                .is_some_and(|selected| file.path != *selected)
            {
                continue;
            }
            if !seen.insert(root.join(&file.path)) {
                continue;
            }
            if !scanner.handles_language(file.language) {
                continue;
            }
            let source = fs::read_to_string(root.join(&file.path))
                .with_context(|| format!("reading {}", file.path.display()))?;
            let path = if selected_file.is_some() {
                requested.clone()
            } else {
                requested.join(&file.path)
            };
            scanned += 1;
            let path = display_path(&path);
            let pending = scanner.collect_language(&source, &path, file.language);
            prepared.push(PreparedFile {
                text: source,
                path,
                language: file.language,
                pending,
            });
        }

        let pending = prepared
            .iter()
            .map(|file| PendingFileScan {
                text: &file.text,
                path: &file.path,
                language: file.language,
                pending: &file.pending,
            })
            .collect();
        let result = scanner.finish_repository(pending);
        suppressed += result.suppressed;
        findings.extend(result.findings);
    }
    findings.sort_by(|left, right| {
        left.location
            .path
            .cmp(&right.location.path)
            .then(left.location.line.cmp(&right.location.line))
            .then(left.location.col.cmp(&right.location.col))
            .then(left.rule.cmp(&right.rule))
    });

    let descriptors = scanner.descriptors();
    let rendered = report::render(
        settings.format,
        &findings,
        &descriptors,
        env!("CARGO_PKG_VERSION"),
    );
    if settings.format == OutputFormat::Text && findings.is_empty() {
        println!("straitjacket: ok — no findings in {scanned} file(s)");
    } else {
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
    }

    if let Some(path) = &cli.sarif {
        let sarif = report::sarif(&findings, &descriptors, env!("CARGO_PKG_VERSION"));
        fs::write(path, sarif)
            .with_context(|| format!("writing SARIF report to {}", path.display()))?;
    }

    if settings.format == OutputFormat::Text && !findings.is_empty() {
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count();
        let warnings = findings.len() - errors;
        eprintln!(
            "straitjacket: {errors} error(s), {warnings} warning(s) across {scanned} file(s); {suppressed} suppressed"
        );
    }

    let has_error = findings
        .iter()
        .any(|finding| finding.severity == Severity::Error);
    Ok(if has_error && !settings.no_fail {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn scan_root(path: &Path) -> anyhow::Result<(PathBuf, Option<PathBuf>)> {
    if !path.exists() {
        anyhow::bail!("scan path does not exist: {}", path.display());
    }
    if path.is_file() {
        let selected = path
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("scan path has no file name: {}", path.display()))?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        Ok((parent.to_path_buf(), Some(selected)))
    } else if path.is_dir() {
        Ok((path.to_path_buf(), None))
    } else {
        anyhow::bail!("scan path is not a file or directory: {}", path.display())
    }
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix("./").unwrap_or(&value).to_string()
}

/// Refuse to report clean on a file that was never read.
///
/// Naming a single file says to scan it. When the walk does not return it the
/// scan covers nothing, and a silent success there is indistinguishable from a
/// file that passed every rule.
fn reject_unscannable_file(
    files: &[SourceFileEntry],
    selected: Option<&Path>,
    requested: &Path,
) -> anyhow::Result<()> {
    let Some(selected) = selected else {
        return Ok(());
    };
    if files.iter().any(|file| file.path == selected) {
        return Ok(());
    }
    anyhow::bail!(
        "nothing to scan in {}: it is excluded by an ignore file, or written in a language Straitjacket does not know",
        requested.display()
    )
}

/// Build the settings for this run from the CLI and any checked-in config.
///
/// An unreadable working directory would otherwise skip discovery and scan
/// with default policy, which looks exactly like a clean repository that
/// configured nothing.
fn resolve(cli: &Cli) -> anyhow::Result<Settings> {
    let mut settings = Settings::default();
    if !cli.no_config {
        let path = match &cli.config {
            Some(path) => Some(path.clone()),
            None => config::find_config(
                &std::env::current_dir()
                    .context("reading the working directory to discover straitjacket.toml")?,
            ),
        };
        if let Some(path) = path {
            let root = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            settings = settings.apply_file_at(config::load_config(&path)?, root);
            eprintln!("straitjacket: using config {}", path.display());
        }
    }

    if !cli.paths.is_empty() {
        settings.paths.clone_from(&cli.paths);
    }
    if let Some(format) = cli.format {
        settings.format = format;
    }
    if !cli.only.is_empty() {
        settings.only.clone_from(&cli.only);
    }
    if !cli.skip.is_empty() {
        settings.skip.clone_from(&cli.skip);
    }
    if let Some(value) = cli.max_lines {
        settings.max_lines = value;
    }
    if let Some(value) = cli.max_nesting {
        settings.max_nesting = value;
    }
    settings.no_comments |= cli.no_comments;
    settings.test_quality |= cli.test_quality;
    settings.include_json |= cli.include_json;
    settings.no_ignore |= cli.no_ignore;
    settings.no_fail |= cli.no_fail;
    settings.fail_on_unused_markers &= !cli.no_fail_on_unused_markers;
    Ok(settings)
}
