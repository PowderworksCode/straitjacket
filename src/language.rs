use std::path::Path;

/// A property a language has that a rule cares about.
///
/// Rules ask what a file can contain, not what it is called: `color` applies
/// wherever a stylesheet value can appear, which is CSS and also Vue, Svelte,
/// and JSX. Facets are compared by identity, so a rule holds a `&'static`
/// reference to the one it wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageFacet {
    pub id: &'static str,
}

pub static COMPONENT_HOST: LanguageFacet = LanguageFacet {
    id: "component-host",
};

pub static STRUCTURED_CODE: LanguageFacet = LanguageFacet {
    id: "structured-code",
};

pub static STYLE_HOST: LanguageFacet = LanguageFacet { id: "style-host" };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageRole {
    Programming,
    Markup,
    Stylesheet,
    Data,
    Documentation,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentSyntax {
    pub line: &'static [&'static str],
    pub block: &'static [(&'static str, &'static str)],
    /// Prefixes recognized by the language's documentation tooling.
    pub documentation: &'static [&'static str],
    pub quotes: &'static [char],
    pub multi_quotes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageProfile {
    pub id: &'static str,
    pub extensions: &'static [&'static str],
    pub filenames: &'static [&'static str],
    /// Interpreters that identify a file with no extension of its own.
    pub shebangs: &'static [&'static str],
    pub role: LanguageRole,
    pub facets: &'static [&'static LanguageFacet],
    pub comments: Option<&'static CommentSyntax>,
}

impl LanguageProfile {
    pub fn has_facet(&self, facet: &LanguageFacet) -> bool {
        self.facets
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, facet))
    }
}

mod syntax {
    use super::CommentSyntax;

    pub static JS: CommentSyntax = CommentSyntax {
        line: &["//"],
        block: &[("/*", "*/")],
        documentation: &["/**"],
        quotes: &['"', '\''],
        multi_quotes: &["`"],
    };

    pub static C_LIKE: CommentSyntax = CommentSyntax {
        line: &["//"],
        block: &[("/*", "*/")],
        documentation: &["///", "//!", "/**", "/*!"],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static RUST: CommentSyntax = CommentSyntax {
        line: &["//"],
        block: &[("/*", "*/")],
        documentation: &["///", "//!", "/**", "/*!"],
        quotes: &['"'],
        multi_quotes: &[],
    };

    pub static PHP: CommentSyntax = CommentSyntax {
        line: &["//", "#"],
        block: &[("/*", "*/")],
        documentation: &["/**"],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static CSS: CommentSyntax = CommentSyntax {
        line: &[],
        block: &[("/*", "*/")],
        documentation: &[],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static CSS_NESTED: CommentSyntax = CommentSyntax {
        line: &["//"],
        block: &[("/*", "*/")],
        documentation: &[],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static HASH: CommentSyntax = CommentSyntax {
        line: &["#"],
        block: &[],
        documentation: &[],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static PYTHON: CommentSyntax = CommentSyntax {
        line: &["#"],
        block: &[],
        documentation: &[],
        quotes: &['"', '\''],
        multi_quotes: &["\"\"\"", "'''"],
    };

    pub static SQL: CommentSyntax = CommentSyntax {
        line: &["--"],
        block: &[("/*", "*/")],
        documentation: &[],
        quotes: &['"', '\''],
        multi_quotes: &[],
    };

    pub static HTML: CommentSyntax = CommentSyntax {
        line: &[],
        block: &[("<!--", "-->")],
        documentation: &[],
        quotes: &[],
        multi_quotes: &[],
    };

    pub static SFC: CommentSyntax = CommentSyntax {
        line: &["//"],
        block: &[("/*", "*/"), ("<!--", "-->")],
        documentation: &["/**"],
        quotes: &['"', '\''],
        multi_quotes: &["`"],
    };
}

macro_rules! profiles {
    ($(
        $constant:ident {
            id: $id:literal,
            role: $role:ident,
            extensions: [$($extension:literal),* $(,)?],
            filenames: [$($filename:literal),* $(,)?],
            shebangs: [$($shebang:literal),* $(,)?],
            facets: [$($facet:ident),* $(,)?],
            comments: $comments:expr,
        }
    )*) => {
        $(
            static $constant: LanguageProfile = LanguageProfile {
                id: $id,
                extensions: &[$($extension),*],
                filenames: &[$($filename),*],
                shebangs: &[$($shebang),*],
                role: LanguageRole::$role,
                facets: &[$(&$facet),*],
                comments: $comments,
            };
        )*

        static PROFILES: &[&LanguageProfile] = &[$(&$constant),*];
    };
}

profiles! {
    C {
        id: "c", role: Programming,
        extensions: ["c", "h"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    CPP {
        id: "cpp", role: Programming,
        extensions: ["cc", "cpp", "cxx", "hh", "hpp", "hxx"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    C_SHARP {
        id: "c-sharp", role: Programming,
        extensions: ["cs"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    CSS {
        id: "css", role: Stylesheet,
        extensions: ["css"], filenames: [], shebangs: [],
        facets: [STYLE_HOST], comments: Some(&syntax::CSS),
    }
    DOCKERFILE {
        id: "dockerfile", role: Build,
        extensions: [], filenames: ["Dockerfile", "Containerfile"], shebangs: [],
        facets: [], comments: Some(&syntax::HASH),
    }
    GO {
        id: "go", role: Programming,
        extensions: ["go"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    HTML {
        id: "html", role: Markup,
        extensions: ["html", "htm"], filenames: [], shebangs: [],
        facets: [STYLE_HOST], comments: Some(&syntax::HTML),
    }
    JAVA {
        id: "java", role: Programming,
        extensions: ["java"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    JAVASCRIPT {
        id: "javascript", role: Programming,
        extensions: ["js", "jsx", "mjs", "cjs"], filenames: [], shebangs: ["node", "bun", "deno"],
        facets: [STRUCTURED_CODE, STYLE_HOST, COMPONENT_HOST], comments: Some(&syntax::JS),
    }
    JSON {
        id: "json", role: Data,
        extensions: ["json", "jsonc"], filenames: [], shebangs: [],
        facets: [], comments: None,
    }
    KOTLIN {
        id: "kotlin", role: Programming,
        extensions: ["kt", "kts"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    LESS {
        id: "less", role: Stylesheet,
        extensions: ["less"], filenames: [], shebangs: [],
        facets: [STYLE_HOST], comments: Some(&syntax::CSS_NESTED),
    }
    MAKE {
        id: "make", role: Build,
        extensions: ["mk"], filenames: ["Makefile", "GNUmakefile"], shebangs: [],
        facets: [], comments: Some(&syntax::HASH),
    }
    MARKDOWN {
        id: "markdown", role: Documentation,
        extensions: ["md", "mdx", "markdown"], filenames: [], shebangs: [],
        facets: [], comments: None,
    }
    PHP {
        id: "php", role: Programming,
        extensions: ["php"], filenames: [], shebangs: ["php"],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::PHP),
    }
    PYTHON {
        id: "python", role: Programming,
        extensions: ["py", "pyi"], filenames: [], shebangs: ["python"],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::PYTHON),
    }
    RUBY {
        id: "ruby", role: Programming,
        extensions: ["rb", "rake"], filenames: ["Gemfile", "Rakefile"], shebangs: ["ruby"],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::HASH),
    }
    RUST {
        id: "rust", role: Programming,
        extensions: ["rs"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::RUST),
    }
    SCALA {
        id: "scala", role: Programming,
        extensions: ["scala"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    SCSS {
        id: "scss", role: Stylesheet,
        extensions: ["scss", "sass"], filenames: [], shebangs: [],
        facets: [STYLE_HOST], comments: Some(&syntax::CSS_NESTED),
    }
    SHELL {
        id: "shell", role: Programming,
        extensions: ["sh", "bash", "zsh", "fish"], filenames: [],
        shebangs: ["sh", "bash", "zsh", "fish"],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::HASH),
    }
    SQL {
        id: "sql", role: Programming,
        extensions: ["sql"], filenames: [], shebangs: [],
        facets: [], comments: Some(&syntax::SQL),
    }
    SVELTE {
        id: "svelte", role: Programming,
        extensions: ["svelte"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE, STYLE_HOST, COMPONENT_HOST], comments: Some(&syntax::SFC),
    }
    SWIFT {
        id: "swift", role: Programming,
        extensions: ["swift"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
    TOML {
        id: "toml", role: Data,
        extensions: ["toml"], filenames: [], shebangs: [],
        facets: [], comments: Some(&syntax::HASH),
    }
    TYPESCRIPT {
        id: "typescript", role: Programming,
        extensions: ["ts", "tsx", "mts", "cts"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE, STYLE_HOST, COMPONENT_HOST], comments: Some(&syntax::JS),
    }
    VUE {
        id: "vue", role: Programming,
        extensions: ["vue"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE, STYLE_HOST, COMPONENT_HOST], comments: Some(&syntax::SFC),
    }
    YAML {
        id: "yaml", role: Data,
        extensions: ["yaml", "yml"], filenames: [], shebangs: [],
        facets: [], comments: Some(&syntax::HASH),
    }
    ZIG {
        id: "zig", role: Programming,
        extensions: ["zig"], filenames: [], shebangs: [],
        facets: [STRUCTURED_CODE], comments: Some(&syntax::C_LIKE),
    }
}

pub fn language_profiles() -> &'static [&'static LanguageProfile] {
    PROFILES
}

pub fn language_profile(id: &str) -> Option<&'static LanguageProfile> {
    PROFILES.iter().copied().find(|profile| profile.id == id)
}

pub fn language_profile_for_extension(extension: &str) -> Option<&'static LanguageProfile> {
    let extension = extension.to_ascii_lowercase();
    PROFILES
        .iter()
        .copied()
        .find(|profile| profile.extensions.contains(&extension.as_str()))
}

/// Identify a file from its name, falling back to its interpreter line.
///
/// Extension first, then whole filename, because `Makefile` and `Dockerfile`
/// carry no extension at all. The shebang is read last and only for a file
/// that nothing else claimed: it is the only test that costs a read.
pub fn language_profile_for_path(
    path: &Path,
    first_line: impl FnOnce() -> Option<String>,
) -> Option<&'static LanguageProfile> {
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str())
        && let Some(profile) = language_profile_for_extension(extension)
    {
        return Some(profile);
    }
    if let Some(name) = path.file_name().and_then(|name| name.to_str())
        && let Some(profile) = PROFILES
            .iter()
            .copied()
            .find(|profile| profile.filenames.contains(&name))
    {
        return Some(profile);
    }
    if path.extension().is_some() {
        return None;
    }
    let line = first_line()?;
    let interpreter = shebang_interpreter(&line)?;
    PROFILES
        .iter()
        .copied()
        .find(|profile| profile.shebangs.contains(&interpreter))
}

/// The interpreter a `#!` line selects, ignoring `env` and any version suffix.
///
/// `#!/usr/bin/env python3` and `#!/bin/bash` both have to land on the name
/// the profiles list, so the path and the trailing digits are stripped.
fn shebang_interpreter(line: &str) -> Option<&'static str> {
    let rest = line.strip_prefix("#!")?;
    let mut words = rest.split_whitespace();
    let mut command = words.next()?;
    if command.rsplit('/').next() == Some("env") {
        command = words.next()?;
    }
    let name = command.rsplit('/').next()?;
    let name =
        name.trim_end_matches(|character: char| character.is_ascii_digit() || character == '.');
    PROFILES
        .iter()
        .flat_map(|profile| profile.shebangs)
        .copied()
        .find(|shebang| *shebang == name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        COMPONENT_HOST, STRUCTURED_CODE, STYLE_HOST, language_profile, language_profile_for_path,
        language_profiles,
    };

    #[test]
    fn every_extension_names_one_language() {
        let mut seen = std::collections::BTreeMap::new();
        for profile in language_profiles() {
            for extension in profile.extensions {
                if let Some(previous) = seen.insert(*extension, profile.id) {
                    panic!(
                        "`{extension}` is claimed by both {previous} and {}",
                        profile.id
                    );
                }
            }
        }
    }

    #[test]
    fn files_are_identified_by_extension_then_name_then_shebang() {
        let rust = language_profile_for_path(Path::new("src/main.rs"), || None).unwrap();
        assert_eq!(rust.id, "rust");
        let dockerfile = language_profile_for_path(Path::new("Dockerfile"), || None).unwrap();
        assert_eq!(dockerfile.id, "dockerfile");
        let script = language_profile_for_path(Path::new("scripts/release"), || {
            Some("#!/usr/bin/env bash".to_owned())
        })
        .unwrap();
        assert_eq!(script.id, "shell");
        let python =
            language_profile_for_path(Path::new("tool"), || Some("#!/usr/bin/python3".to_owned()))
                .unwrap();
        assert_eq!(python.id, "python");
        assert!(language_profile_for_path(Path::new("notes.unknown"), || None).is_none());
    }

    #[test]
    fn facets_describe_what_a_file_can_contain() {
        let vue = language_profile("vue").unwrap();
        assert!(vue.has_facet(&STYLE_HOST));
        assert!(vue.has_facet(&COMPONENT_HOST));
        assert!(vue.has_facet(&STRUCTURED_CODE));
        assert!(!language_profile("css").unwrap().has_facet(&STRUCTURED_CODE));
    }
}
