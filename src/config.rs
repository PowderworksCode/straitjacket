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
    pub theme_files: Option<Vec<String>>,
    pub max_nesting: Option<usize>,
    pub no_comments: Option<bool>,
    pub include_json: Option<bool>,
    pub no_ignore: Option<bool>,
    pub no_fail: Option<bool>,
    pub fail_on_unused_markers: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub paths: Vec<PathBuf>,
    pub format: OutputFormat,
    pub only: Vec<String>,
    pub skip: Vec<String>,
    pub max_lines: usize,
    pub file_size_exclude: Vec<PathBuf>,
    pub theme_files: Vec<PathBuf>,
    pub max_nesting: usize,
    pub no_comments: bool,
    pub include_json: bool,
    pub no_ignore: bool,
    pub no_fail: bool,
    pub fail_on_unused_markers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            format: OutputFormat::Text,
            only: Vec::new(),
            skip: Vec::new(),
            max_lines: DEFAULT_MAX_LINES,
            file_size_exclude: Vec::new(),
            theme_files: Vec::new(),
            max_nesting: DEFAULT_MAX_NESTING,
            no_comments: false,
            include_json: false,
            no_ignore: false,
            no_fail: false,
            fail_on_unused_markers: true,
        }
    }
}

impl Settings {
    pub fn apply_file(mut self, file: FileConfig) -> Self {
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
        if let Some(value) = file.max_lines {
            self.max_lines = value;
        }
        if let Some(paths) = file.file_size_exclude {
            self.file_size_exclude = paths.into_iter().map(PathBuf::from).collect();
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
    toml::from_str(&source)
        .with_context(|| format!("parsing TOML configuration {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::FileConfig;

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
}
