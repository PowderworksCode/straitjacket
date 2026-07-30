use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;
use crate::rules::comments;

const ALLOW: &str = "straitjacket-allow";
pub const KEY: RuleKey = RuleKey::new("no-comments");

pub struct NoCommentsRule;

fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(NoCommentsRule)
}

fn instruction(_: &Settings) -> String {
    "Source comments are not allowed. Make the code carry its intent through names and structure; put durable explanation in documentation.".into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}

impl FileRule for NoCommentsRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "comment present while no-comments mode is enabled",
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
        "comment present while no-comments mode is enabled",
    );
    finding.help = Some("make the code carry the explanation, or put history in the commit".into());
    finding
}

fn scan(text: &str, path: &str, language: &LanguageProfile) -> Vec<Finding> {
    comments::scan(text, language)
        .into_iter()
        .filter(|comment| !comment.contains(ALLOW))
        .map(|comment| {
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
            lines("let x = 1; // note\n/* block */\n", "typescript"),
            [1, 2]
        );
    }

    #[test]
    fn ignores_strings_urls_shebangs_and_markers() {
        assert!(lines("const x = \"https://x/y\";\n", "typescript").is_empty());
        assert!(lines("#!/bin/sh\necho ok\n", "shell").is_empty());
        assert!(lines("// straitjacket-allow:no-comments reason\n", "typescript").is_empty());
    }

    #[test]
    fn block_comment_reports_once() {
        assert_eq!(lines("/* first\nsecond\nthird */\n", "rust"), [1]);
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
