use std::path::{Path, PathBuf};

use anyhow::Context;
use directories::ProjectDirs;
use globset::Glob;
use infact_core::Effect;
use infact_duplication::{ExactConfig, NearConfig};
use serde::Deserialize;

use crate::report::OutputFormat;

pub const CONFIG_NAME: &str = "straitjacket.toml";
pub const DEFAULT_MAX_LINES: usize = 1_500;
pub const DEFAULT_MAX_NESTING: usize = 8;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FactFileConfig {
    pub cache: Option<String>,
    pub lock: Option<String>,
    pub parser_paths: Option<Vec<String>>,
    pub registries: Option<Vec<String>>,
    pub dependencies: Option<DependencySelection>,
    pub build_missing: Option<bool>,
    pub exact_clones: Option<bool>,
    pub near_clones: Option<bool>,
    pub clone_exclude: Option<Vec<String>>,
    #[serde(default)]
    pub builders: Vec<FactBuilderFileConfig>,
    #[serde(default)]
    pub exact: ExactFactFileConfig,
    #[serde(default)]
    pub near: NearFactFileConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FactBuilderFileConfig {
    pub ecosystem: String,
    pub command: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExactFactFileConfig {
    pub min_tokens: Option<u32>,
    pub min_lines: Option<u32>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NearFactFileConfig {
    pub min_tokens: Option<u32>,
    pub min_lines: Option<u32>,
    pub normalize_identifiers: Option<bool>,
    pub normalize_literals: Option<bool>,
    pub max_changed_percent: Option<u8>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AspirationFileConfig {
    #[serde(default)]
    pub libraries: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnlistedEffectPolicy {
    Allow,
    #[default]
    Deny,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IncompleteEffectPolicy {
    #[default]
    Error,
    Warn,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EffectCapability {
    pub name: String,
    #[serde(default)]
    pub includes: Vec<Effect>,
    #[serde(default)]
    pub provided_by: Vec<String>,
    #[serde(default)]
    pub available_to: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EffectSettings {
    #[serde(default)]
    pub unlisted: UnlistedEffectPolicy,
    #[serde(default)]
    pub incomplete: IncompleteEffectPolicy,
    #[serde(default)]
    pub capabilities: Vec<EffectCapability>,
}

impl Default for EffectSettings {
    fn default() -> Self {
        Self {
            unlisted: UnlistedEffectPolicy::Deny,
            incomplete: IncompleteEffectPolicy::Error,
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencySelection {
    #[default]
    Automatic,
    None,
}

#[derive(Debug, Clone)]
pub struct FactSettings {
    pub repository_root: PathBuf,
    pub cache: PathBuf,
    pub lock: PathBuf,
    pub parser_paths: Vec<PathBuf>,
    pub registries: Vec<String>,
    pub dependencies: DependencySelection,
    pub build_missing: bool,
    pub exact_clones: bool,
    pub near_clones: bool,
    pub clone_exclude: Vec<PathBuf>,
    pub builders: Vec<FactBuilder>,
    pub exact: ExactConfig,
    pub near: NearConfig,
    pub aspirations: Vec<String>,
    pub require_call_effects: bool,
}

#[derive(Debug, Clone)]
pub struct FactBuilder {
    pub ecosystem: String,
    pub command: Vec<String>,
}

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
    pub include_json: Option<bool>,
    pub no_ignore: Option<bool>,
    pub no_fail: Option<bool>,
    pub fail_on_unused_markers: Option<bool>,
    #[serde(default)]
    pub facts: FactFileConfig,
    #[serde(default)]
    pub aspirations: AspirationFileConfig,
    pub effects: Option<EffectSettings>,
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
    pub include_json: bool,
    pub no_ignore: bool,
    pub no_fail: bool,
    pub fail_on_unused_markers: bool,
    pub facts: FactSettings,
    pub effects: Option<EffectSettings>,
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
            include_json: false,
            no_ignore: false,
            no_fail: false,
            fail_on_unused_markers: true,
            facts: FactSettings {
                repository_root: PathBuf::from("."),
                cache: default_fact_cache(),
                lock: PathBuf::from("straitjacket.lock.toml"),
                parser_paths: Vec::new(),
                registries: vec!["ghcr.io/zmaril/infact-facts".to_owned()],
                dependencies: DependencySelection::Automatic,
                build_missing: false,
                exact_clones: false,
                near_clones: false,
                clone_exclude: Vec::new(),
                builders: Vec::new(),
                exact: ExactConfig::default(),
                near: NearConfig::default(),
                aspirations: Vec::new(),
                require_call_effects: false,
            },
            effects: None,
        }
    }
}

impl Settings {
    pub fn apply_file(self, file: FileConfig) -> Self {
        self.apply_file_at(file, Path::new("."))
    }

    pub fn apply_file_at(mut self, file: FileConfig, root: &Path) -> Self {
        self.config_root = root.to_path_buf();
        self.facts.repository_root = root.to_path_buf();
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
        let facts = file.facts;
        if let Some(path) = facts.cache {
            self.facts.cache = resolve_path(root, path);
        }
        self.facts.lock = facts
            .lock
            .map(|path| resolve_path(root, path))
            .unwrap_or_else(|| root.join("straitjacket.lock.toml"));
        if let Some(paths) = facts.parser_paths {
            self.facts.parser_paths = paths
                .into_iter()
                .map(|path| resolve_path(root, path))
                .collect();
        }
        if let Some(registries) = facts.registries {
            self.facts.registries = registries;
        }
        if let Some(dependencies) = facts.dependencies {
            self.facts.dependencies = dependencies;
        }
        if let Some(build_missing) = facts.build_missing {
            self.facts.build_missing = build_missing;
        }
        if let Some(enabled) = facts.exact_clones {
            self.facts.exact_clones = enabled;
        }
        if let Some(enabled) = facts.near_clones {
            self.facts.near_clones = enabled;
        }
        if let Some(paths) = facts.clone_exclude {
            self.facts.clone_exclude = paths
                .into_iter()
                .map(|path| resolve_path(root, path))
                .collect();
        }
        self.facts.builders = facts
            .builders
            .into_iter()
            .map(|builder| FactBuilder {
                ecosystem: builder.ecosystem,
                command: builder.command,
            })
            .collect();
        let exact_defaults = self.facts.exact;
        self.facts.exact = ExactConfig {
            min_tokens: facts.exact.min_tokens.unwrap_or(exact_defaults.min_tokens),
            min_lines: facts.exact.min_lines.unwrap_or(exact_defaults.min_lines),
        };
        let near_defaults = self.facts.near;
        self.facts.near = NearConfig {
            min_tokens: facts.near.min_tokens.unwrap_or(near_defaults.min_tokens),
            min_lines: facts.near.min_lines.unwrap_or(near_defaults.min_lines),
            normalize_identifiers: facts
                .near
                .normalize_identifiers
                .unwrap_or(near_defaults.normalize_identifiers),
            normalize_literals: facts
                .near
                .normalize_literals
                .unwrap_or(near_defaults.normalize_literals),
            max_changed_percent: facts
                .near
                .max_changed_percent
                .unwrap_or(near_defaults.max_changed_percent),
        };
        self.effects = file.effects;
        self.facts.aspirations = file.aspirations.libraries;
        self.facts.require_call_effects = self.effects.is_some();
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let Some(effects) = &self.effects else {
            return Ok(());
        };
        let mut names = std::collections::BTreeSet::new();
        let mut included = std::collections::BTreeMap::new();
        for capability in &effects.capabilities {
            if capability.name.trim().is_empty() {
                anyhow::bail!("effect capability name cannot be empty");
            }
            if !names.insert(capability.name.as_str()) {
                anyhow::bail!("duplicate effect capability `{}`", capability.name);
            }
            if capability.includes.is_empty() {
                anyhow::bail!(
                    "effect capability `{}` must include at least one effect",
                    capability.name
                );
            }
            if capability.provided_by.is_empty() {
                anyhow::bail!(
                    "effect capability `{}` must have at least one provided-by path",
                    capability.name
                );
            }
            for effect in &capability.includes {
                if let Some(previous) = included.insert(*effect, capability.name.as_str()) {
                    anyhow::bail!(
                        "effect `{}` is included by both `{previous}` and `{}`",
                        effect.as_str(),
                        capability.name
                    );
                }
            }
            for pattern in capability
                .provided_by
                .iter()
                .chain(&capability.available_to)
            {
                Glob::new(pattern).with_context(|| {
                    format!(
                        "invalid path pattern `{pattern}` in effect capability `{}`",
                        capability.name
                    )
                })?;
            }
        }
        Ok(())
    }
}

fn resolve_path(root: &Path, path: String) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn default_fact_cache() -> PathBuf {
    ProjectDirs::from("dev", "Powderworks", "straitjacket")
        .map(|directories| directories.cache_dir().join("facts"))
        .unwrap_or_else(|| PathBuf::from(".straitjacket/facts"))
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
    use std::path::Path;

    use infact_core::Effect;

    use super::{
        DependencySelection, FileConfig, IncompleteEffectPolicy, Settings, UnlistedEffectPolicy,
    };

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
    fn fact_paths_are_relative_to_the_configuration() {
        let config: FileConfig = toml::from_str(
            "[facts]\ncache = \"cache\"\nlock = \"facts.lock.toml\"\nparser-paths = [\"parsers\"]\ndependencies = \"none\"\nexact-clones = true\nclone-exclude = [\"tests/fixtures\"]\n\n[[facts.builders]]\necosystem = \"cargo\"\ncommand = [\"infact-builder\", \"build\"]\n\n[aspirations]\nlibraries = [\"cargo:itertools@0.15\"]\n",
        )
        .unwrap();
        let settings = Settings::default().apply_file_at(config, Path::new("repo"));
        assert_eq!(settings.facts.cache, Path::new("repo/cache"));
        assert_eq!(settings.facts.lock, Path::new("repo/facts.lock.toml"));
        assert_eq!(
            settings.facts.clone_exclude,
            [Path::new("repo/tests/fixtures")]
        );
        assert_eq!(settings.facts.builders[0].ecosystem, "cargo");
        assert_eq!(
            settings.facts.builders[0].command,
            ["infact-builder", "build"]
        );
        assert_eq!(settings.facts.parser_paths, vec![Path::new("repo/parsers")]);
        assert_eq!(settings.facts.dependencies, DependencySelection::None);
        assert!(settings.facts.exact_clones);
        assert_eq!(settings.facts.aspirations, ["cargo:itertools@0.15"]);
    }

    #[test]
    fn effect_capabilities_use_closed_world_defaults() {
        let config: FileConfig = toml::from_str(
            "[effects]\n\n[[effects.capabilities]]\nname = 'filesystem'\nincludes = ['file-read', 'file-write']\nprovided-by = ['src/adapters/filesystem/**']\navailable-to = ['src/application/**']\n",
        )
        .unwrap();
        let settings = Settings::default().apply_file(config);
        settings.validate().unwrap();
        assert!(settings.facts.require_call_effects);
        let effects = settings.effects.unwrap();
        assert_eq!(effects.unlisted, UnlistedEffectPolicy::Deny);
        assert_eq!(effects.incomplete, IncompleteEffectPolicy::Error);
        assert_eq!(
            effects.capabilities[0].includes,
            [Effect::FileRead, Effect::FileWrite]
        );
    }

    #[test]
    fn effect_capabilities_reject_overlapping_effects() {
        let config: FileConfig = toml::from_str(
            "[effects]\nincomplete = 'warn'\n\n[[effects.capabilities]]\nname = 'first'\nincludes = ['time']\nprovided-by = ['src/clock.rs']\n\n[[effects.capabilities]]\nname = 'second'\nincludes = ['time']\nprovided-by = ['src/runtime.rs']\n",
        )
        .unwrap();
        let settings = Settings::default().apply_file(config);
        assert!(
            settings
                .validate()
                .unwrap_err()
                .to_string()
                .contains("included by both")
        );
    }
}
