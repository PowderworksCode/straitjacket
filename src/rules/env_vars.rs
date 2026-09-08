//! Environment reads outside the declared configuration edge, found by
//! beamte's `env-read` rule over a treebank pack.
//!
//! The finding is beamte's: an environment variable read mid-file is an input
//! no signature admits to, and no small test of that code can stay hermetic
//! (*Test Sizes*, 2010-12-13). This file owns everything beamte refuses to:
//! getting a grammar, parsing, deciding a severity, and — the part that is
//! policy about a repository rather than a fact about a tree — naming the
//! files that *are* the configuration edge. `env-files` in
//! `straitjacket.toml` names them, and a read inside one is licensed rather
//! than reported.
//!
//! Same door as `test-quality`, different files: that rule reads what looks
//! like a test, this one reads everything it can parse, which is what
//! beamte's `Scope::File` on the rule means. Both go through `inspect_with`,
//! so the two cannot come to format a finding differently.
//!
//! A finding here is an error rather than a mapping of beamte's property:
//! the rule is opt-in, and a repository that turned it on wants the read
//! stopped, not mentioned. The licensed edge is `env-files`; the exception
//! with a story is a suppression marker, which must carry one.

use beamte::node::Unit;
use beamte::{RuleId, Selection};

use crate::Settings;
use crate::finding::Severity;
use crate::language::LanguageProfile;
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::{RuleRegistration, beamte_findings};

pub const KEY: RuleKey = RuleKey::new("env-vars");

/// Off unless a configuration asks for it, for `test-quality`'s reason: the
/// first run downloads a grammar, and a scan that reaches the network because
/// the tool was upgraded is not a surprise anyone should get for free.
const DEFAULT_ENABLED: bool = false;

/// A language this rule can read.
///
/// The list is beamte's — `env_read::covers` — and the entries here add what
/// only a host knows: which pack serves the grammar and which cheap
/// substrings mean a file is worth parsing at all. A file with no marker
/// cannot contain a read the surface table would match, so it is skipped
/// before any parse, which is what makes running over *every* source file
/// affordable.
///
/// Shell is deliberately absent, agreeing with beamte: `$VAR` is the
/// language's own variable model, and flagging every expansion would be
/// flagging the language.
struct Supported {
    /// Straitjacket's own language id, from `src/language.rs`.
    id: &'static str,
    pack: &'static str,
    model: &'static str,
    markers: &'static [&'static str],
}

const SUPPORTED: &[Supported] = &[
    Supported {
        id: "c",
        pack: "c",
        model: "c",
        markers: &["getenv", "_dupenv_s"],
    },
    Supported {
        id: "cpp",
        pack: "cpp",
        model: "cpp",
        markers: &["getenv", "_dupenv_s"],
    },
    Supported {
        id: "java",
        pack: "java",
        model: "java",
        markers: &["System.getenv", "System.getProperty"],
    },
    Supported {
        id: "javascript",
        pack: "typescript",
        model: "javascript",
        markers: &["process.env", "import.meta.env", "Deno.env"],
    },
    Supported {
        id: "python",
        pack: "python",
        model: "python",
        markers: &["environ", "getenv"],
    },
    Supported {
        id: "ruby",
        pack: "ruby",
        model: "ruby",
        markers: &["ENV"],
    },
    Supported {
        id: "rust",
        pack: "rust",
        model: "rust",
        markers: &["env::var", "env::vars"],
    },
    Supported {
        id: "typescript",
        pack: "typescript",
        model: "typescript",
        markers: &["process.env", "import.meta.env", "Deno.env"],
    },
    Supported {
        id: "zig",
        pack: "zig",
        model: "zig",
        markers: &["getenv", "getEnvVar", "getEnvMap", "hasEnvVar"],
    },
];

fn supported(language: &LanguageProfile) -> Option<&'static Supported> {
    SUPPORTED.iter().find(|entry| entry.id == language.id)
}

pub struct EnvVarsRule {
    /// The files licensed to read the environment: the declared configuration
    /// edge. Everything else is reported. Same matching as
    /// `file-size-exclude`, so one notion of "this path" covers both.
    allow: Vec<std::path::PathBuf>,
    /// `Only(env-read)`: the one file-scoped rule beamte has today. Computed
    /// from the catalogue rather than named, so a second file-scoped rule
    /// lights up here the way a test-scoped one lights up `test-quality`.
    only: Vec<RuleId>,
}

impl EnvVarsRule {
    pub fn new(allow: Vec<std::path::PathBuf>) -> Self {
        let only = beamte::catalogue()
            .iter()
            .filter(|rule| rule.scope == beamte::Scope::File)
            .map(|rule| rule.id)
            .collect();
        Self { allow, only }
    }

    fn licensed(&self, path: &str) -> bool {
        let path = std::path::Path::new(path);
        self.allow.iter().any(|allowed| {
            path == allowed
                || path.starts_with(allowed)
                || allowed.is_relative() && path.is_absolute() && path.ends_with(allowed)
        })
    }
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(EnvVarsRule::new(settings.env_files.clone()))
}

fn instruction(settings: &Settings) -> String {
    let rules: Vec<String> = beamte::catalogue()
        .iter()
        .filter(|rule| rule.scope == beamte::Scope::File)
        .map(|rule| format!("{} ({})", rule.instruction, rule.citation.title))
        .collect();
    let edge = if settings.env_files.is_empty() {
        "No file is currently declared as that edge; name one with `env-files` \
         in straitjacket.toml."
            .to_string()
    } else {
        format!(
            "The declared edge is: {}.",
            settings
                .env_files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!("{} {edge}", rules.join(" "))
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}

impl FileRule for EnvVarsRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "code reads the process environment outside the declared edge",
            default_enabled: DEFAULT_ENABLED,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        supported(language).is_some()
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        let Some(entry) = supported(file.language) else {
            return;
        };
        if self.licensed(file.path) {
            return;
        }
        if !entry.markers.iter().any(|m| file.text.contains(m)) {
            return;
        }
        let Some(model) = beamte::TestModel::for_language(entry.model) else {
            return;
        };

        let pack = match crate::pack::cached(entry.pack) {
            Ok(pack) => pack,
            Err(reason) => {
                candidates.push(beamte_findings::not_read(
                    KEY,
                    file.path,
                    format!(
                        "not read: the {} grammar could not be loaded, so this file was \
                         not checked for environment reads ({reason})",
                        entry.pack
                    ),
                    Some(
                        "Packs are downloaded once and cached. Check network access, or \
                         skip `env-vars` if this environment is offline by design."
                            .to_string(),
                    ),
                ));
                return;
            }
        };

        let tree = match pack.parse(file.text) {
            Ok(tree) => tree,
            Err(error) => {
                candidates.push(beamte_findings::not_read(
                    KEY,
                    file.path,
                    format!(
                        "not read: this file did not parse as {} ({error:#})",
                        entry.model
                    ),
                    None,
                ));
                return;
            }
        };

        let unit = Unit::new(file.path, tree.source(), tree.root());
        for finding in beamte::inspect_with(&unit, &model, Selection::Only(&self.only)) {
            candidates.push(beamte_findings::candidate(
                KEY,
                Severity::Error,
                file.path,
                file.text,
                finding,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvVarsRule, KEY, SUPPORTED};

    #[test]
    fn every_supported_language_is_one_beamte_covers() {
        for entry in SUPPORTED {
            assert!(
                beamte::rules::env_read::covers(entry.model),
                "{} maps to model {}, which beamte's env-read does not cover",
                entry.id,
                entry.model
            );
        }
        assert_eq!(
            SUPPORTED.len(),
            beamte::rules::env_read::LANGUAGES.len(),
            "beamte covers a language this rule does not offer, or the reverse"
        );
    }

    #[test]
    fn every_supported_language_is_one_straitjacket_knows() {
        for entry in SUPPORTED {
            assert!(
                crate::language::language_profile(entry.id).is_some(),
                "{} is not a language straitjacket has a profile for",
                entry.id
            );
        }
    }

    /// A language with no markers is a language whose every file parses; a
    /// language with wrong markers is worse, a prefilter that skips files
    /// the rule would have flagged. Emptiness is checkable here; fidelity
    /// is what `tests/env_vars.rs` checks with real reads.
    #[test]
    fn every_marker_would_survive_its_own_surface() {
        for entry in SUPPORTED {
            assert!(
                !entry.markers.is_empty(),
                "{} has no markers, so every file of it parses",
                entry.id
            );
        }
    }

    #[test]
    fn a_licensed_file_is_licensed_however_the_walk_spells_it() {
        let rule = EnvVarsRule::new(vec!["src/config.rs".into(), "tools/".into()]);

        assert!(rule.licensed("src/config.rs"));
        assert!(rule.licensed("/repo/src/config.rs"));
        assert!(rule.licensed("tools/release.py"));
        assert!(!rule.licensed("src/main.rs"));
        assert!(!rule.licensed("src/config.rs.bak"));
    }

    #[test]
    fn the_key_is_the_one_the_registry_carries() {
        assert_eq!(KEY.as_str(), "env-vars");
    }
}
