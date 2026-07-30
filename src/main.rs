use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use entl_codebase::{DiagnosticKind, InventoryOptions, walk};

use straitjacket::config::{self, Settings};
use straitjacket::finding::Severity;
use straitjacket::instructions;
use straitjacket::report::{self, OutputFormat};
use straitjacket::{Finding, Scanner};

#[derive(Debug, Parser)]
#[command(
    name = "straitjacket",
    version,
    about = "Find opinionated source-code smells and report them consistently"
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

    #[arg(long, help = "List all known rules and exit")]
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

    if matches!(cli.command, Some(Command::Instructions)) {
        print!(
            "{}",
            instructions::render(&settings, &scanner.enabled_descriptors())?
        );
        return Ok(ExitCode::SUCCESS);
    }

    if cli.list_rules {
        for descriptor in scanner.descriptors() {
            let default = if descriptor.default_enabled {
                "default"
            } else {
                "opt-in"
            };
            println!("{} ({default})\n    {}", descriptor.id, descriptor.summary);
        }
        return Ok(ExitCode::SUCCESS);
    }

    let mut findings = Vec::<Finding>::new();
    let mut scanned = 0usize;
    let mut suppressed = 0usize;
    let mut seen = BTreeSet::new();
    for requested in &settings.paths {
        let (root, selected_file) = scan_root(requested)?;
        let tree = walk(
            &root,
            &InventoryOptions {
                respect_gitignore: !settings.no_ignore,
                respect_global_gitignore: false,
                respect_parent_ignores: !settings.no_ignore,
                include_generated: true,
                include_hidden: settings.no_ignore,
                ..InventoryOptions::default()
            },
        )?;
        if selected_file.is_none()
            && let Some(diagnostic) = tree
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.kind == DiagnosticKind::Walk)
        {
            anyhow::bail!(
                "walking {} at {}: {}",
                requested.display(),
                diagnostic.path.display(),
                diagnostic.message
            );
        }
        if let Some(selected) = &selected_file {
            let selected_entry = tree.file(selected).ok_or_else(|| {
                anyhow::anyhow!(
                    "selected file was not returned by the walk: {}",
                    requested.display()
                )
            })?;
            if selected_entry.language.is_none()
                && let Some(diagnostic) = tree
                    .diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.path == *selected)
            {
                anyhow::bail!(
                    "inspecting {} at {}: {}",
                    requested.display(),
                    diagnostic.path.display(),
                    diagnostic.message
                );
            }
        }

        for file in &tree.files {
            if selected_file
                .as_ref()
                .is_some_and(|selected| file.path != *selected)
            {
                continue;
            }
            let absolute = tree.root.join(&file.path);
            if !seen.insert(absolute) {
                continue;
            }
            let Some(detection) = &file.language else {
                continue;
            };
            let language = detection.profile().ok_or_else(|| {
                anyhow::anyhow!(
                    "Entl detected unregistered language `{}` for {}",
                    detection.language,
                    file.path.display()
                )
            })?;
            if !scanner.handles_language(language) {
                continue;
            }
            let source = tree
                .read_text(&file.path)
                .with_context(|| format!("reading {}", file.path.display()))?;
            let path = if selected_file.is_some() {
                requested.clone()
            } else {
                requested.join(&file.path)
            };
            scanned += 1;
            let result = scanner.scan_language(&source, &display_path(&path), language);
            suppressed += result.suppressed;
            findings.extend(result.findings);
        }
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

fn resolve(cli: &Cli) -> anyhow::Result<Settings> {
    let mut settings = Settings::default();
    if !cli.no_config {
        let path = match &cli.config {
            Some(path) => Some(path.clone()),
            None => std::env::current_dir()
                .ok()
                .and_then(|directory| config::find_config(&directory)),
        };
        if let Some(path) = path {
            settings = settings.apply_file(config::load_config(&path)?);
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
    settings.include_json |= cli.include_json;
    settings.no_ignore |= cli.no_ignore;
    settings.no_fail |= cli.no_fail;
    settings.fail_on_unused_markers &= !cli.no_fail_on_unused_markers;
    Ok(settings)
}
