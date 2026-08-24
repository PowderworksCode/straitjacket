//! Measure the comment-only detector over real git history.
//!
//! Not part of the shipped binary. It walks every non-merge commit in the given
//! repositories, reconstructs both sides of every modified file, and reports
//! which file changes leave the code identical.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use straitjacket::diff::{CommentEdit, CommentOnlyChange, comment_only_change};
use straitjacket::language::{LanguageProfile, language_profile_for_path};

const MAX_BLOB: usize = 512 * 1024;

struct BlobReader {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl BlobReader {
    fn open(repo: &Path) -> anyhow::Result<Self> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    fn read(&mut self, oid: &str) -> anyhow::Result<Option<String>> {
        writeln!(self.stdin, "{oid}")?;
        self.stdin.flush()?;
        let mut header = String::new();
        self.stdout.read_line(&mut header)?;
        let fields: Vec<&str> = header.split_whitespace().collect();
        if fields.len() != 3 {
            return Ok(None);
        }
        let size: usize = fields[2].parse()?;
        let mut body = vec![0u8; size + 1];
        self.stdout.read_exact(&mut body)?;
        body.truncate(size);
        if size > MAX_BLOB {
            return Ok(None);
        }
        Ok(String::from_utf8(body).ok())
    }
}

impl Drop for BlobReader {
    fn drop(&mut self) {
        if self.child.kill().is_ok() {
            drop(self.child.wait());
        }
    }
}

fn git(repo: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

struct Modified {
    before: String,
    after: String,
    path: String,
}

struct Tree {
    modified: Vec<Modified>,
    touched: usize,
    created_or_deleted: usize,
}

fn modified_files(repo: &Path, commit: &str) -> anyhow::Result<Tree> {
    let raw = git(
        repo,
        &["diff-tree", "-r", "-M", "--root", "--no-commit-id", commit],
    )?;
    let mut modified = Vec::new();
    let mut touched = 0;
    let mut created_or_deleted = 0;
    for line in raw.lines() {
        let Some(rest) = line.strip_prefix(':') else {
            continue;
        };
        let Some((meta, path)) = rest.split_once('\t') else {
            continue;
        };
        touched += 1;
        let fields: Vec<&str> = meta.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }
        if !fields[4].starts_with('M') {
            created_or_deleted += 1;
            continue;
        }
        modified.push(Modified {
            before: fields[2].to_owned(),
            after: fields[3].to_owned(),
            path: path.split('\t').next_back().unwrap_or(path).to_owned(),
        });
    }
    Ok(Tree {
        modified,
        touched,
        created_or_deleted,
    })
}

/// Ask difftastic whether the change survives its parser with comments ignored.
///
/// `Some(true)` means difftastic parsed both sides and saw no syntactic change
/// once comments are set aside, which is its version of the same verdict.
fn difftastic_says_comment_only(path: &str, before: &str, after: &str) -> Option<bool> {
    let name = Path::new(path).file_name()?.to_str()?;
    let dir = tempfile::tempdir().ok()?;
    let old = dir.path().join("old").join(name);
    let new = dir.path().join("new").join(name);
    std::fs::create_dir_all(old.parent()?).ok()?;
    std::fs::create_dir_all(new.parent()?).ok()?;
    std::fs::write(&old, before).ok()?;
    std::fs::write(&new, after).ok()?;
    let status = Command::new("difft")
        .args(["--ignore-comments", "--check-only", "--exit-code"])
        .arg(&old)
        .arg(&new)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// The lowercase words of a text, minus the ones that carry no information.
fn words(text: &str) -> std::collections::BTreeSet<String> {
    const NOISE: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "can", "for", "from", "has",
        "have", "in", "is", "it", "its", "not", "of", "on", "one", "or", "so", "that", "the",
        "then", "there", "this", "to", "was", "what", "when", "which", "with", "would",
    ];
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| word.len() > 2)
        .map(|word| word.to_ascii_lowercase())
        .filter(|word| !NOISE.contains(&word.as_str()))
        .collect()
}

/// How much of the two comment texts is the same vocabulary.
///
/// A typo fix or a link migration keeps nearly every word. A comment rewritten
/// for its own sake does not.
fn similarity(removed: &str, added: &str) -> f64 {
    let before = words(removed);
    let after = words(added);
    let union = before.union(&after).count();
    if union == 0 {
        return 1.0;
    }
    before.intersection(&after).count() as f64 / union as f64
}

/// Every line the commit adds, outside the given file.
fn added_elsewhere(repo: &Path, commit: &str, skip: &str) -> anyhow::Result<String> {
    let raw = git(repo, &["show", "--format=", "--unified=0", commit])?;
    let mut collected = String::new();
    let mut current = String::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            current = rest.to_owned();
            continue;
        }
        if current != skip
            && let Some(rest) = line.strip_prefix('+')
            && !rest.starts_with("++")
        {
            collected.push_str(rest);
            collected.push('\n');
        }
    }
    Ok(collected)
}

fn language_of(path: &str) -> Option<&'static LanguageProfile> {
    language_profile_for_path(Path::new(path), || None)
}

fn kind_name(edit: CommentEdit) -> &'static str {
    match edit {
        CommentEdit::Rewritten => "rewritten",
        CommentEdit::Added => "added",
        CommentEdit::Removed => "removed",
        CommentEdit::Moved => "moved",
    }
}

fn main() -> anyhow::Result<()> {
    let mut repos: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let cross_check = repos.iter().any(|arg| arg.as_os_str() == "--cross-check");
    repos.retain(|arg| arg.as_os_str() != "--cross-check");
    let mut confusion: HashMap<(bool, bool), usize> = HashMap::new();
    let mut unparsed = 0usize;
    let mut totals: HashMap<String, usize> = HashMap::new();
    let mut commits_total = 0usize;
    let mut commits_hit = 0usize;
    let mut commits_pure = 0usize;
    let mut drifted = 0usize;
    let mut reworded = 0usize;
    let mut files_total = 0usize;
    let mut stdout = std::io::stdout().lock();

    for repo in &repos {
        let name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        let log = git(
            repo,
            &["log", "--no-merges", "--format=%H%x1f%an%x1f%aI%x1f%s"],
        )?;
        let mut blobs = BlobReader::open(repo)?;

        for entry in log.lines() {
            let fields: Vec<&str> = entry.split('\u{1f}').collect();
            if fields.len() < 4 {
                continue;
            }
            let (commit, author, date, subject) = (fields[0], fields[1], fields[2], fields[3]);
            commits_total += 1;
            let tree = modified_files(repo, commit)?;
            let mut hits: Vec<(String, &'static LanguageProfile, CommentOnlyChange)> = Vec::new();
            let mut examined = 0usize;
            let mut code_changed = 0usize;
            for file in &tree.modified {
                let language = language_of(&file.path).filter(|l| l.comments.is_some());
                let Some(language) = language else {
                    code_changed += 1;
                    continue;
                };
                let (Some(before), Some(after)) =
                    (blobs.read(&file.before)?, blobs.read(&file.after)?)
                else {
                    code_changed += 1;
                    continue;
                };
                examined += 1;
                if cross_check {
                    let cheap = comment_only_change(language, &before, &after).is_some();
                    match difftastic_says_comment_only(&file.path, &before, &after) {
                        Some(parsed) => {
                            *confusion.entry((cheap, parsed)).or_default() += 1;
                            if cheap != parsed {
                                let record = serde_json::json!({
                                    "disagreement": true,
                                    "repo": name,
                                    "commit": commit,
                                    "path": file.path,
                                    "straitjacket": cheap,
                                    "difftastic": parsed,
                                });
                                println!("{record}");
                            }
                        }
                        None => unparsed += 1,
                    }
                }
                match comment_only_change(language, &before, &after) {
                    Some(change) => hits.push((file.path.clone(), language, change)),
                    None => code_changed += 1,
                }
            }
            files_total += examined;
            if hits.is_empty() {
                continue;
            }
            commits_hit += 1;
            let pure = code_changed == 0 && tree.created_or_deleted == 0;
            if pure {
                commits_pure += 1;
            }
            for (path, language, change) in hits {
                let new_words: Vec<String> = words(&change.added.join(" "))
                    .difference(&words(&change.removed.join(" ")))
                    .cloned()
                    .collect();
                let overlap = similarity(&change.removed.join(" "), &change.added.join(" "));
                let elsewhere = words(&added_elsewhere(repo, commit, &path)?);
                let unexplained: Vec<&String> = new_words
                    .iter()
                    .filter(|word| !elsewhere.contains(*word))
                    .collect();
                let tracks_nothing = !new_words.is_empty() && unexplained.len() == new_words.len();
                if tracks_nothing {
                    drifted += 1;
                    if change.edit == CommentEdit::Rewritten && overlap < 0.6 {
                        reworded += 1;
                    }
                }
                *totals.entry(kind_name(change.edit).to_owned()).or_default() += 1;
                let record = serde_json::json!({
                    "repo": name,
                    "commit": commit,
                    "author": author,
                    "date": date,
                    "subject": subject,
                    "path": path,
                    "language": language.id,
                    "edit": kind_name(change.edit),
                    "files_touched": tree.touched,
                    "solo_file": tree.touched == 1,
                    "code_changed_files": code_changed,
                    "pure_comment_commit": pure,
                    "similarity": overlap,
                    "new_words": new_words,
                    "tracks_nothing": tracks_nothing,
                    "removed": change.removed,
                    "added": change.added,
                });
                writeln!(stdout, "{record}")?;
            }
        }
    }

    if cross_check {
        eprintln!("cross-check against difftastic --ignore-comments:");
        for cheap in [true, false] {
            for parsed in [true, false] {
                let count = confusion.get(&(cheap, parsed)).copied().unwrap_or(0);
                eprintln!("  straitjacket={cheap:<5} difftastic={parsed:<5} {count}");
            }
        }
        eprintln!("  difftastic could not parse: {unparsed}");
    }
    let mut kinds: Vec<_> = totals.iter().collect();
    kinds.sort();
    eprintln!("commits scanned: {commits_total}");
    eprintln!("modified files examined: {files_total}");
    eprintln!("commits with a comment-only file: {commits_hit}");
    eprintln!("commits that changed nothing but comments: {commits_pure}");
    eprintln!("comment-only files whose new wording tracks nothing else in the commit: {drifted}");
    eprintln!("  of those, rewrites that also replaced most of the vocabulary: {reworded}");
    for (kind, count) in kinds {
        eprintln!("  {kind}: {count}");
    }
    Ok(())
}
