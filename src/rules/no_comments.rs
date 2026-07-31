use std::collections::{BTreeMap, BTreeSet};

use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;
use crate::rules::comments;

const ALLOW: &str = "straitjacket-allow";
const HEADER_LINE_LIMIT: usize = 10;
pub const KEY: RuleKey = RuleKey::new("no-comments");

pub struct NoCommentsRule;

fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(NoCommentsRule)
}

fn instruction(_: &Settings) -> String {
    "Ordinary comments are allowed only in the leading 10 lines of a file, before code begins. Documentation comments are allowed where they feed language documentation tooling, including rustdoc and JSDoc. After the header, make implementation intent clear through names and structure; Straitjacket checks this before commits when the hook is installed and in CI.".into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        repository_factory: None,
        instruction,
    }
}

impl FileRule for NoCommentsRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "ordinary comment outside the leading file header",
            default_enabled: false,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        comments::supports(language)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        for finding in scan(file.text, file.path, file.language) {
            candidates.push(Candidate::line(finding));
        }
    }
}

fn finding(path: &str, line: usize, col: usize, text: &str) -> Finding {
    let mut finding = Finding::new(
        KEY,
        Severity::Error,
        Location::point(path, line, col),
        comments::snippet(text),
        "ordinary comment outside the leading 10-line file header",
    );
    finding.help = Some(
        "use a documentation comment when documenting an API, or make implementation intent structural"
            .into(),
    );
    finding
}

fn is_documentation_comment(comment: &comments::Comment, language: &LanguageProfile) -> bool {
    let text = comment.head().text.trim_start();
    language.comments.is_some_and(|syntax| {
        syntax
            .documentation
            .iter()
            .any(|prefix| text.starts_with(prefix))
    })
}

fn leading_header_comments(text: &str, comments: &[comments::Comment]) -> BTreeSet<usize> {
    let mut parts_by_line = BTreeMap::<usize, Vec<(usize, &comments::CommentPart)>>::new();
    for (comment_index, comment) in comments.iter().enumerate() {
        for part in &comment.parts {
            parts_by_line
                .entry(part.line)
                .or_default()
                .push((comment_index, part));
        }
    }

    let mut allowed = BTreeSet::new();
    for (line_index, line) in text.lines().take(HEADER_LINE_LIMIT).enumerate() {
        let line_number = line_index + 1;
        if line_number == 1 && line.starts_with("#!") {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let whole_line_comment = parts_by_line.get(&line_number).and_then(|parts| {
            parts.iter().find(|(_, part)| {
                let start = part.col.saturating_sub(1);
                let end = start.saturating_add(part.text.len());
                line.get(..start)
                    .is_some_and(|prefix| prefix.trim().is_empty())
                    && line
                        .get(end..)
                        .is_some_and(|suffix| suffix.trim().is_empty())
            })
        });
        let Some((comment_index, _)) = whole_line_comment else {
            break;
        };
        allowed.insert(*comment_index);
    }

    allowed.retain(|index| {
        comments[*index]
            .parts
            .iter()
            .all(|part| part.line <= HEADER_LINE_LIMIT)
    });
    allowed
}

fn scan(text: &str, path: &str, language: &LanguageProfile) -> Vec<Finding> {
    let comments = comments::scan(text, language);
    let header_comments = leading_header_comments(text, &comments);
    comments
        .into_iter()
        .enumerate()
        .filter(|(index, comment)| {
            !header_comments.contains(index)
                && !is_documentation_comment(comment, language)
                && !comment.contains(ALLOW)
        })
        .map(|(_, comment)| {
            finding(
                path,
                comment.line(),
                comment.col(),
                &comment.head().text.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use entl_codebase::language_profile;

    use super::scan;

    fn lines(source: &str, language: &str) -> Vec<usize> {
        scan(source, "test", language_profile(language).unwrap())
            .into_iter()
            .map(|finding| finding.location.line)
            .collect()
    }

    #[test]
    fn recognizes_line_and_block_comments() {
        assert_eq!(
            lines("let x = 1;\n// note\n/* block */\n", "typescript"),
            [2, 3]
        );
    }

    #[test]
    fn allows_only_the_leading_ten_line_header() {
        assert!(
            lines(
                "// generated for an agent\n\n/* repository constraints */\nconst x = 1;\n",
                "typescript"
            )
            .is_empty()
        );
        assert_eq!(
            lines(
                "\n\n\n\n\n\n\n\n\n\n// too late\nconst x = 1;\n",
                "typescript"
            ),
            [11]
        );
        assert_eq!(
            lines(
                "// header\nconst x = 1;\n// implementation note\n",
                "typescript"
            ),
            [3]
        );
    }

    #[test]
    fn a_header_block_cannot_exceed_ten_lines() {
        assert_eq!(
            lines(
                "/* one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven */\n",
                "rust"
            ),
            [1]
        );
    }

    #[test]
    fn allows_rustdoc_and_jsdoc() {
        assert!(
            lines(
                "pub const X: u8 = 1;\n/// Documents the next item.\npub const Y: u8 = 2;\n/*! Crate documentation. */\n",
                "rust"
            )
            .is_empty()
        );
        assert!(
            lines(
                "const x = 1;\n/** Documents the export. */\nexport const y = 2;\n",
                "typescript"
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_strings_urls_shebangs_and_markers() {
        assert!(lines("const x = \"https://x/y\";\n", "typescript").is_empty());
        assert!(lines("#!/bin/sh\necho ok\n", "shell").is_empty());
        assert!(
            lines(
                "const x = 1;\n// straitjacket-allow:no-comments reason\n",
                "typescript"
            )
            .is_empty()
        );
    }

    #[test]
    fn block_comment_reports_once() {
        assert_eq!(
            lines("fn main() {}\n/* first\nsecond\nthird */\n", "rust"),
            [2]
        );
    }

    #[test]
    fn rust_lifetime_does_not_hide_comment() {
        assert_eq!(lines("fn f<'a>(x: &'a str) {} // note\n", "rust"), [1]);
    }

    #[test]
    fn ignores_rust_raw_string_contents() {
        assert!(
            lines(
                "let script = r##\"/* not a comment */\n// still text\"##;\n",
                "rust"
            )
            .is_empty()
        );
        assert!(lines("let bytes = br#\"// text\"#;\n", "rust").is_empty());
    }
}
