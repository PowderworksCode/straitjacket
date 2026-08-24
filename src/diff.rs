//! Rules about the shape of a change, rather than the state of a file.
//!
//! Every existing rule reads a file's final text. Some slop is invisible there:
//! a comment rewritten in place leaves a file that is perfectly fine to read,
//! and only the edit itself is odd. These functions take the two sides of a
//! change and describe what moved between them.

use std::collections::BTreeMap;

use crate::language::LanguageProfile;
use crate::rules::comments;

/// What a comment-only change did to the comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentEdit {
    /// Comment text present before is gone, and different text has arrived.
    Rewritten,
    /// Comments were added and none were removed.
    Added,
    /// Comments were removed and none were added.
    Removed,
    /// The comments are the same text; only their position or spacing moved.
    Moved,
}

/// A change to a file that leaves the code identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentOnlyChange {
    pub edit: CommentEdit,
    pub removed: Vec<String>,
    pub added: Vec<String>,
}

/// Describe a file change whose code is untouched, or `None` if code moved.
///
/// The test is deliberately whole-file rather than per-hunk: what the observer
/// noticed was "a comment is edited in place and *nothing else*", and that is a
/// claim about the file, not about one hunk. Comparing code skeletons also
/// dodges the question of whether a hunk's context lines sit inside a block
/// comment, which no line-by-line reading of a unified diff can answer.
pub fn comment_only_change(
    language: &LanguageProfile,
    before: &str,
    after: &str,
) -> Option<CommentOnlyChange> {
    if language.comments.is_none() || before == after {
        return None;
    }
    if code_skeleton(language, before) != code_skeleton(language, after) {
        return None;
    }

    let before_comments = comment_texts(language, before);
    let after_comments = comment_texts(language, after);
    let removed = multiset_difference(&before_comments, &after_comments);
    let added = multiset_difference(&after_comments, &before_comments);

    let edit = match (removed.is_empty(), added.is_empty()) {
        (false, false) => CommentEdit::Rewritten,
        (true, false) => CommentEdit::Added,
        (false, true) => CommentEdit::Removed,
        (true, true) => CommentEdit::Moved,
    };
    Some(CommentOnlyChange {
        edit,
        removed,
        added,
    })
}

/// The file with every comment cut out, as the lines that still hold code.
///
/// A line that a comment leaves blank is dropped, so adding or deleting a whole
/// comment line does not shift the skeleton. A line that was already blank is
/// kept, and leading whitespace is kept, so reindenting or reflowing the code
/// is a code change and not a comment change.
fn code_skeleton(language: &LanguageProfile, text: &str) -> Vec<String> {
    let mut spans: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for comment in comments::scan(text, language) {
        for part in &comment.parts {
            let start = part.col.saturating_sub(1);
            spans
                .entry(part.line)
                .or_default()
                .push((start, start + part.text.len()));
        }
    }

    let mut skeleton = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(cuts) = spans.get(&(index + 1)) else {
            skeleton.push(line.trim_end().to_owned());
            continue;
        };
        let mut cuts = cuts.clone();
        cuts.sort_unstable();
        let mut kept = String::new();
        let mut cursor = 0;
        for (start, end) in cuts {
            let start = clamp_boundary(line, start);
            let end = clamp_boundary(line, end);
            if start > cursor {
                kept.push_str(&line[cursor..start]);
            }
            cursor = cursor.max(end);
        }
        kept.push_str(&line[cursor..]);
        let kept = kept.trim_end();
        if !kept.trim().is_empty() {
            skeleton.push(kept.to_owned());
        }
    }
    skeleton
}

fn clamp_boundary(line: &str, offset: usize) -> usize {
    let mut offset = offset.min(line.len());
    while !line.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn comment_texts(language: &LanguageProfile, text: &str) -> Vec<String> {
    let mut texts: Vec<String> = comments::scan(text, language)
        .iter()
        .map(|comment| {
            comment
                .parts
                .iter()
                .map(|part| part.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    texts.sort();
    texts
}

/// Everything in `left` that `right` does not also have, counting duplicates.
fn multiset_difference(left: &[String], right: &[String]) -> Vec<String> {
    let mut remaining: BTreeMap<&str, usize> = BTreeMap::new();
    for value in right {
        *remaining.entry(value.as_str()).or_default() += 1;
    }
    let mut difference = Vec::new();
    for value in left {
        match remaining.get_mut(value.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => difference.push(value.clone()),
        }
    }
    difference
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::language_profile;

    fn rust() -> &'static LanguageProfile {
        language_profile("rust").expect("rust profile")
    }

    #[test]
    fn comment_rewritten_in_place_is_comment_only() {
        let before = "// adds two numbers\nfn add(a: u8, b: u8) -> u8 {\n    a + b\n}\n";
        let after =
            "// Adds the two operands together.\nfn add(a: u8, b: u8) -> u8 {\n    a + b\n}\n";
        let change = comment_only_change(rust(), before, after).expect("comment-only");
        assert_eq!(change.edit, CommentEdit::Rewritten);
        assert_eq!(change.removed, vec!["// adds two numbers".to_owned()]);
    }

    #[test]
    fn code_change_beside_a_comment_is_not_comment_only() {
        let before = "// doc\nfn add(a: u8, b: u8) -> u8 {\n    a + b\n}\n";
        let after = "// docs\nfn add(a: u8, b: u8) -> u8 {\n    a - b\n}\n";
        assert!(comment_only_change(rust(), before, after).is_none());
    }

    #[test]
    fn reindenting_code_is_not_comment_only() {
        let before = "fn add() {\n    // sum\n    1 + 1\n}\n";
        let after = "fn add() {\n  // sum\n  1 + 1\n}\n";
        assert!(comment_only_change(rust(), before, after).is_none());
    }

    #[test]
    fn a_comment_inside_a_string_is_not_a_comment() {
        let before = "let url = \"https://a\";\n";
        let after = "let url = \"https://b\";\n";
        assert!(comment_only_change(rust(), before, after).is_none());
    }

    #[test]
    fn added_comment_is_reported_as_added() {
        let before = "fn add() {\n    1 + 1\n}\n";
        let after = "fn add() {\n    // one and one\n    1 + 1\n}\n";
        let change = comment_only_change(rust(), before, after).expect("comment-only");
        assert_eq!(change.edit, CommentEdit::Added);
        assert!(change.removed.is_empty());
    }

    #[test]
    fn a_moved_comment_keeps_its_text() {
        let before = "fn add() {\n    // sum\n    1 + 1\n}\n";
        let after = "fn add() {\n    1 + 1 // sum\n}\n";
        let change = comment_only_change(rust(), before, after).expect("comment-only");
        assert_eq!(change.edit, CommentEdit::Moved);
    }
}
