use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

use crate::report::OutputFormat;

pub const CONFIG_NAME: &str = "straitjacket.toml";
pub const DEFAULT_MAX_LINES: usize = 1_500;
pub const DEFAULT_MAX_NESTING: usize = 8;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FileConfig {
    pub paths: Option<Vec<String>>,
    pub format: Option<OutputFormat>,
    pub only: Option<Vec<String>>,
    pub skip: Option<Vec<String>>,
    pub max_lines: Option<usize>,
    pub file_size_exclude: Option<Vec<String>>,
    pub todo_exclude: Option<Vec<String>>,
    pub theme_files: Option<Vec<String>>,
    pub max_nesting: Option<usize>,
    pub no_comments: Option<bool>,
    pub test_quality: Option<bool>,
    pub include_json: Option<bool>,
    pub no_ignore: Option<bool>,
    pub no_fail: Option<bool>,
    pub fail_on_unused_markers: Option<bool>,
    /// Which of beamte's test-quality rules to run. Unset means all of them,
    /// including any beamte adds later.
    pub test_rules: Option<Vec<String>>,
    /// Sections that configured rules Straitjacket no longer has. They are
    /// accepted by the parser only so that [`reject_removed_sections`] can
    /// name the rule that went away.
    #[serde(default)]
    pub facts: Option<toml::Value>,
    #[serde(default)]
    pub effects: Option<toml::Value>,
    #[serde(default)]
    pub errors: Option<toml::Value>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub config_root: PathBuf,
    pub paths: Vec<PathBuf>,
    pub format: OutputFormat,
    pub only: Vec<String>,
    pub skip: Vec<String>,
    pub max_lines: usize,
    pub file_size_exclude: Vec<PathBuf>,
    pub todo_exclude: Vec<PathBuf>,
    pub theme_files: Vec<PathBuf>,
    pub max_nesting: usize,
    pub no_comments: bool,
    pub test_quality: bool,
    pub include_json: bool,
    pub no_ignore: bool,
    pub no_fail: bool,
    pub fail_on_unused_markers: bool,
    pub test_rules: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            config_root: PathBuf::from("."),
            paths: vec![PathBuf::from(".")],
            format: OutputFormat::Text,
            only: Vec::new(),
            skip: Vec::new(),
            max_lines: DEFAULT_MAX_LINES,
            file_size_exclude: Vec::new(),
            todo_exclude: Vec::new(),
            theme_files: Vec::new(),
            max_nesting: DEFAULT_MAX_NESTING,
            no_comments: false,
            test_quality: false,
            include_json: false,
            no_ignore: false,
            no_fail: false,
            fail_on_unused_markers: true,
            test_rules: Vec::new(),
        }
    }
}

impl Settings {
    pub fn apply_file(self, file: FileConfig) -> Self {
        self.apply_file_at(file, Path::new("."))
    }

    pub fn apply_file_at(mut self, file: FileConfig, root: &Path) -> Self {
        self.config_root = root.to_path_buf();
        if let Some(paths) = file.paths {
            self.paths = paths.into_iter().map(PathBuf::from).collect();
        }
        if let Some(format) = file.format {
            self.format = format;
        }
        if let Some(only) = file.only {
            self.only = only;
        }
        if let Some(skip) = file.skip {
            self.skip = skip;
        }
        if let Some(test_rules) = file.test_rules {
            self.test_rules = test_rules;
        }
        if let Some(value) = file.max_lines {
            self.max_lines = value;
        }
        if let Some(paths) = file.file_size_exclude {
            self.file_size_exclude = paths.into_iter().map(PathBuf::from).collect();
        }
        if let Some(paths) = file.todo_exclude {
            self.todo_exclude = paths.into_iter().map(PathBuf::from).collect();
        }
        if let Some(paths) = file.theme_files {
            self.theme_files = paths.into_iter().map(PathBuf::from).collect();
        }
        if let Some(value) = file.max_nesting {
            self.max_nesting = value;
        }
        if let Some(value) = file.no_comments {
            self.no_comments = value;
        }
        if let Some(value) = file.test_quality {
            self.test_quality = value;
        }
        if let Some(value) = file.include_json {
            self.include_json = value;
        }
        if let Some(value) = file.no_ignore {
            self.no_ignore = value;
        }
        if let Some(value) = file.no_fail {
            self.no_fail = value;
        }
        if let Some(value) = file.fail_on_unused_markers {
            self.fail_on_unused_markers = value;
        }
        self
    }
}

/// Refuse a configuration written for a Straitjacket that had more rules.
///
/// `deny_unknown_fields` would already reject these, but it would report an
/// unknown key, which reads like a typo. The rules were withdrawn on purpose
/// and the message has to say so, because deleting the section is the fix.
fn reject_removed_sections(file: &FileConfig, path: &Path) -> anyhow::Result<()> {
    let sections = [
        (
            "facts",
            file.facts.is_some(),
            "exact-clone, near-clone and library-opportunity",
        ),
        (
            "effects",
            file.effects.is_some(),
            "effect-barrier and effect-capability",
        ),
        ("errors", file.errors.is_some(), "error-discard"),
    ];
    for (name, present, rules) in sections {
        if present {
            anyhow::bail!(
                "{}: [{name}] configures {rules}, which Straitjacket no longer has. Delete the section.",
                path.display()
            );
        }
    }
    Ok(())
}

pub fn find_config(start: &Path) -> Option<PathBuf> {
    for directory in start.ancestors() {
        let candidate = directory.join(CONFIG_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn load_config(path: &Path) -> anyhow::Result<FileConfig> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading configuration {}", path.display()))?;
    let file: FileConfig = toml::from_str(&source)
        .with_context(|| format!("parsing TOML configuration {}", path.display()))?;
    reject_removed_sections(&file, path)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{FileConfig, Settings, reject_removed_sections};

    #[test]
    fn config_uses_kebab_case_and_rejects_unknown_keys() {
        let config: FileConfig = toml::from_str(
            "max-lines = 800\nfile-size-exclude = [\"notes/\"]\ntheme-files = [\"src/theme.css\"]\nno-comments = true\n",
        )
        .expect("valid TOML");
        assert_eq!(config.max_lines, Some(800));
        assert_eq!(config.file_size_exclude, Some(vec!["notes/".into()]));
        assert_eq!(config.theme_files, Some(vec!["src/theme.css".into()]));
        assert_eq!(config.no_comments, Some(true));
        assert!(toml::from_str::<FileConfig>("max_lines = 800").is_err());
    }

    #[test]
    fn settings_come_from_the_configuration_file() {
        let config: FileConfig =
            toml::from_str("paths = [\"src\"]\nmax-nesting = 3\nskip = [\"emoji\"]\n").unwrap();
        let settings = Settings::default().apply_file_at(config, Path::new("repo"));
        assert_eq!(settings.paths, [Path::new("src")]);
        assert_eq!(settings.max_nesting, 3);
        assert_eq!(settings.skip, ["emoji"]);
        assert_eq!(settings.config_root, Path::new("repo"));
    }

    #[test]
    fn removed_sections_name_the_rule_that_went_away() {
        for (section, rule) in [
            ("[facts]\nexact-clones = true\n", "exact-clone"),
            (
                "[effects]\n\n[[effects.capabilities]]\nname = 'filesystem'\n",
                "effect-capability",
            ),
            ("[errors]\ndeny = ['let-underscore']\n", "error-discard"),
        ] {
            let config: FileConfig = toml::from_str(section).unwrap();
            let error = reject_removed_sections(&config, Path::new("straitjacket.toml"))
                .unwrap_err()
                .to_string();
            assert!(error.contains(rule), "{error} should name {rule}");
            assert!(error.contains("no longer has"), "{error}");
        }
    }
}
