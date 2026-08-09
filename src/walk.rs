use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use ignore::WalkBuilder;

use crate::language::{LanguageProfile, language_profile_for_path};

#[derive(Debug, Clone, Copy)]
pub struct WalkOptions {
    /// Honor `.gitignore`, `.ignore`, and `.git/info/exclude`.
    pub respect_ignore_files: bool,
    /// Include dot-files and dot-directories other than `.git`.
    pub include_hidden: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            respect_ignore_files: true,
            include_hidden: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceFileEntry {
    /// Path relative to the walked root.
    pub path: PathBuf,
    pub language: &'static LanguageProfile,
}

/// List the source files under a root, in a stable order.
///
/// A walk error is returned rather than skipped. A scan over less than the
/// repository must not be able to report clean, and an unreadable directory is
/// exactly the shape that produces one.
pub fn walk(root: &Path, options: &WalkOptions) -> anyhow::Result<Vec<SourceFileEntry>> {
    if !root.is_dir() {
        anyhow::bail!("scan root is not a directory: {}", root.display());
    }
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(!options.include_hidden)
        .parents(options.respect_ignore_files)
        .ignore(options.respect_ignore_files)
        .git_ignore(options.respect_ignore_files)
        .git_exclude(options.respect_ignore_files)
        .git_global(false)
        .require_git(false)
        .follow_links(false)
        .sort_by_file_path(Path::cmp);
    builder.filter_entry(|entry| entry.file_name() != ".git");

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.with_context(|| format!("walking {}", root.display()))?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let absolute = entry.path();
        let Some(language) = language_profile_for_path(absolute, || first_line(absolute)) else {
            continue;
        };
        let path = absolute
            .strip_prefix(root)
            .unwrap_or(absolute)
            .to_path_buf();
        files.push(SourceFileEntry { path, language });
    }
    Ok(files)
}

/// The first line of a file, when it is text and has one.
///
/// Only the head is read, because the caller is looking for a `#!` line and a
/// file with no extension may be a binary of any size. A file that will not
/// open or is not text simply goes unidentified, which is the same outcome as
/// a file whose interpreter nothing recognizes.
fn first_line(path: &Path) -> Option<String> {
    let mut head = [0u8; 128];
    let mut file = std::fs::File::open(path).ok()?;
    let read = file.read(&mut head).ok()?;
    let text = std::str::from_utf8(&head[..read]).ok()?;
    Some(text.lines().next()?.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{WalkOptions, walk};

    #[test]
    fn ignored_and_hidden_files_are_left_out() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".gitignore"), "generated.rs\n").unwrap();
        fs::write(root.path().join("kept.rs"), "fn main() {}\n").unwrap();
        fs::write(root.path().join("generated.rs"), "fn main() {}\n").unwrap();
        fs::create_dir(root.path().join(".hidden")).unwrap();
        fs::write(root.path().join(".hidden/secret.rs"), "fn main() {}\n").unwrap();

        let found = walk(root.path(), &WalkOptions::default()).unwrap();
        let paths = found
            .iter()
            .map(|entry| entry.path.display().to_string())
            .collect::<Vec<_>>();
        assert_eq!(paths, ["kept.rs"]);

        let everything = walk(
            root.path(),
            &WalkOptions {
                respect_ignore_files: false,
                include_hidden: true,
            },
        )
        .unwrap();
        assert_eq!(everything.len(), 3);
    }
}
