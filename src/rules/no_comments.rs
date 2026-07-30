use entl_codebase::LanguageProfile;

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

const ALLOW: &str = "straitjacket-allow";
pub const KEY: RuleKey = RuleKey::new("no-comments");

pub fn supports_language(language: &LanguageProfile) -> bool {
    language.comments.is_some()
}

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
        supports_language(language)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        for finding in scan(file.text, file.path, file.language) {
            candidates.push(Candidate::line(finding));
        }
    }
}

fn boundary_ok(marker: &str, previous: Option<char>) -> bool {
    match marker {
        "//" => previous != Some(':'),
        "#" | "--" => previous.is_none_or(char::is_whitespace),
        _ => true,
    }
}

fn snippet(value: &str) -> String {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let head: String = chars.by_ref().take(60).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn rust_raw_string(rest: &str) -> Option<(usize, String)> {
    let prefix_len = if rest.starts_with("br") || rest.starts_with("cr") {
        2
    } else if rest.starts_with('r') {
        1
    } else {
        return None;
    };
    let mut cursor = prefix_len;
    while rest.as_bytes().get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    if rest.as_bytes().get(cursor) != Some(&b'"') {
        return None;
    }
    let hashes = cursor - prefix_len;
    Some((cursor + 1, format!("\"{}", "#".repeat(hashes))))
}

fn finding(path: &str, line: usize, col: usize, text: &str) -> Finding {
    let mut finding = Finding::new(
        KEY,
        Severity::Error,
        Location::point(path, line, col),
        snippet(text),
        "comment present while no-comments mode is enabled",
    );
    finding.help = Some("make the code carry the explanation, or put history in the commit".into());
    finding
}

struct OpenBlock {
    end: &'static str,
    has_marker: bool,
    finding: Finding,
}

fn scan(text: &str, path: &str, language: &LanguageProfile) -> Vec<Finding> {
    let Some(syntax) = language.comments else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    let mut open_block: Option<OpenBlock> = None;
    let mut open_string: Option<String> = None;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line_number == 1 && line.starts_with("#!") && syntax.line.contains(&"#") {
            continue;
        }

        let mut cursor = 0;
        if let Some(block) = open_block.as_mut() {
            match line.find(block.end) {
                Some(position) => {
                    block.has_marker |= line[..position].contains(ALLOW);
                    let block = open_block.take().expect("open block exists");
                    if !block.has_marker {
                        findings.push(block.finding);
                    }
                    cursor = position + block.end.len();
                }
                None => {
                    block.has_marker |= line.contains(ALLOW);
                    continue;
                }
            }
        } else if let Some(delimiter) = open_string.as_deref() {
            match line.find(delimiter) {
                Some(position) => {
                    cursor = position + delimiter.len();
                    open_string = None;
                }
                None => continue,
            }
        }

        let mut previous = line[..cursor].chars().next_back();
        let mut quote = None;
        'line: while cursor < line.len() {
            let rest = &line[cursor..];
            let character = rest.chars().next().expect("cursor is a char boundary");

            if let Some(delimiter) = quote {
                if character == '\\' {
                    cursor += character.len_utf8();
                    if let Some(escaped) = line[cursor..].chars().next() {
                        cursor += escaped.len_utf8();
                    }
                    continue;
                }
                if character == delimiter {
                    quote = None;
                }
                cursor += character.len_utf8();
                continue;
            }

            if language.id == "rust"
                && let Some((opening_len, delimiter)) = rust_raw_string(rest)
            {
                let after = cursor + opening_len;
                match line[after..].find(&delimiter) {
                    Some(position) => cursor = after + position + delimiter.len(),
                    None => {
                        open_string = Some(delimiter);
                        break 'line;
                    }
                }
                previous = line[..cursor].chars().next_back();
                continue;
            }

            if let Some(delimiter) = syntax
                .multi_quotes
                .iter()
                .find(|delimiter| rest.starts_with(**delimiter))
            {
                let after = cursor + delimiter.len();
                match line[after..].find(delimiter) {
                    Some(position) => cursor = after + position + delimiter.len(),
                    None => {
                        open_string = Some((*delimiter).to_owned());
                        break 'line;
                    }
                }
                previous = delimiter.chars().next_back();
                continue;
            }

            if syntax.quotes.contains(&character) {
                quote = Some(character);
                cursor += character.len_utf8();
                continue;
            }

            if let Some((open, close)) =
                syntax.block.iter().find(|(open, _)| rest.starts_with(open))
            {
                let after = cursor + open.len();
                match line[after..].find(close) {
                    Some(position) => {
                        let end = after + position + close.len();
                        if !line[after..after + position].contains(ALLOW) {
                            findings.push(finding(
                                path,
                                line_number,
                                cursor + 1,
                                &line[cursor..end],
                            ));
                        }
                        cursor = end;
                        previous = close.chars().next_back();
                        continue;
                    }
                    None => {
                        open_block = Some(OpenBlock {
                            end: close,
                            has_marker: line[after..].contains(ALLOW),
                            finding: finding(path, line_number, cursor + 1, rest),
                        });
                        break 'line;
                    }
                }
            }

            if let Some(marker) = syntax.line.iter().find(|marker| rest.starts_with(**marker))
                && boundary_ok(marker, previous)
            {
                if !rest.contains(ALLOW) {
                    findings.push(finding(path, line_number, cursor + 1, rest));
                }
                break 'line;
            }

            previous = Some(character);
            cursor += character.len_utf8();
        }
    }

    if let Some(block) = open_block
        && !block.has_marker
    {
        findings.push(block.finding);
    }
    findings
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
