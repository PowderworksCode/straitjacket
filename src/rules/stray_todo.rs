use std::path::{Path, PathBuf};

use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;
use crate::rules::comments::{self, CommentPart};

const KEY: RuleKey = RuleKey::new("stray-todo");
const MARKERS: [&str; 4] = ["TODO", "TBD", "FIXME", "WIP"];

pub struct StrayTodoRule {
    exclude: Vec<PathBuf>,
}

impl StrayTodoRule {
    pub fn new(exclude: Vec<PathBuf>) -> Self {
        Self { exclude }
    }
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(StrayTodoRule::new(settings.todo_exclude.clone()))
}

fn instruction(settings: &Settings) -> String {
    let markers = MARKERS.join(", ");
    if settings.todo_exclude.is_empty() {
        return format!(
            "Deferred-work markers ({markers}) are not allowed in comments. Do the work, or track it in an issue."
        );
    }
    let excluded = settings
        .todo_exclude
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Deferred-work markers ({markers}) are not allowed in comments outside {excluded}. Do the work, or track it in an issue."
    )
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}

impl FileRule for StrayTodoRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "deferred-work marker left in a comment",
            default_enabled: true,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        comments::supports(language)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        let path = Path::new(file.path);
        if self.exclude.iter().any(|excluded| {
            path == excluded
                || path.starts_with(excluded)
                || excluded.is_relative() && path.is_absolute() && path.ends_with(excluded)
        }) {
            return;
        }
        for comment in comments::scan(file.text, file.language) {
            for part in &comment.parts {
                for (offset, marker) in markers_in(&part.text) {
                    candidates.push(Candidate::line(finding(file.path, part, offset, marker)));
                }
            }
        }
    }
}

fn finding(path: &str, part: &CommentPart, offset: usize, marker: &'static str) -> Finding {
    let mut finding = Finding::new(
        KEY,
        Severity::Error,
        Location::point(path, part.line, part.col + offset),
        comments::snippet(&part.text[offset..]),
        format!("{marker} marker left in a comment"),
    );
    finding.help = Some("do the work now, or record it in an issue the repository tracks".into());
    finding
}

fn word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let free = |character: Option<char>| {
        character.is_none_or(|value| !value.is_alphanumeric() && value != '_')
    };
    free(before) && free(after)
}

fn markers_in(text: &str) -> Vec<(usize, &'static str)> {
    let upper = text.to_uppercase();
    if upper.len() != text.len() {
        return markers_by_char(text);
    }
    let mut hits = Vec::new();
    for marker in MARKERS {
        let mut from = 0;
        while let Some(position) = upper[from..].find(marker) {
            let start = from + position;
            let end = start + marker.len();
            if word_boundary(text, start, end) {
                hits.push((start, marker));
            }
            from = end;
        }
    }
    hits.sort_by_key(|(offset, _)| *offset);
    hits
}

fn markers_by_char(text: &str) -> Vec<(usize, &'static str)> {
    let mut hits = Vec::new();
    for (start, _) in text.char_indices() {
        for marker in MARKERS {
            let end = start + marker.len();
            if text.is_char_boundary(end)
                && text[start..end].eq_ignore_ascii_case(marker)
                && word_boundary(text, start, end)
            {
                hits.push((start, marker));
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use entl_codebase::language_profile;

    use crate::rule::{Candidate, FileRule, SourceFile};

    use super::StrayTodoRule;

    fn findings(source: &str, language: &str) -> Vec<(usize, usize, String)> {
        let mut candidates: Vec<Candidate> = Vec::new();
        StrayTodoRule::new(Vec::new()).check(
            SourceFile {
                path: "test",
                language: language_profile(language).unwrap(),
                text: source,
            },
            &mut candidates,
        );
        candidates
            .into_iter()
            .map(|candidate| {
                (
                    candidate.finding.location.line,
                    candidate.finding.location.col,
                    candidate.finding.matched,
                )
            })
            .collect()
    }

    #[test]
    fn flags_markers_inside_comments_with_positions() {
        let hits = findings("let x = 1; // TODO: fix this\n", "rust");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 1);
        assert_eq!(hits[0].1, 15);
        assert_eq!(hits[0].2, "TODO: fix this");
    }

    #[test]
    fn ignores_markers_outside_comments() {
        assert!(findings("let url = \"https://example.com/page#TODO\";\n", "rust").is_empty());
        assert!(findings("let s = \"-- TODO: data, not a comment\";\n", "rust").is_empty());
        assert!(findings("let todos = load(\"todo.txt\");\n", "rust").is_empty());
        assert!(findings("print(\"# TODO inside a string\")\n", "python").is_empty());
    }

    #[test]
    fn reports_the_line_inside_a_block_comment() {
        let hits = findings("/* first\n   FIXME: later\n   third */\n", "rust");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 2);
    }

    #[test]
    fn requires_a_word_boundary_and_matches_any_case() {
        assert!(findings("// TODOS are fine as a word\n", "rust").is_empty());
        assert!(findings("// nothing deferred here\n", "rust").is_empty());
        assert_eq!(findings("// todo: lowercase counts\n", "rust").len(), 1);
        assert_eq!(findings("// WIP\n", "rust").len(), 1);
    }
}
