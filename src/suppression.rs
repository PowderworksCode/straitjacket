use std::collections::HashSet;

use entl_codebase::LanguageProfile;

use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, RuleKey};
use crate::rules;

const LINE_MARKER: &str = "straitjacket-allow";
const FILE_MARKER: &str = "straitjacket-allow-file";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Marker {
    line: usize,
    file_level: bool,
    rule: Option<String>,
    used: bool,
}

#[derive(Debug, Default)]
pub struct SuppressionResult {
    pub findings: Vec<Finding>,
    pub suppressed: usize,
}

pub fn apply(
    text: &str,
    path: &str,
    language: &LanguageProfile,
    candidates: Vec<Candidate>,
    enabled_rules: &HashSet<RuleKey>,
    applicable_rules: &HashSet<RuleKey>,
    fail_on_unused: bool,
) -> SuppressionResult {
    let mut markers = collect_markers(text, language);
    let mut result = SuppressionResult::default();

    if enabled_rules.contains(&rules::NO_COMMENTS) && applicable_rules.contains(&rules::NO_COMMENTS)
    {
        for marker in &mut markers {
            if marker
                .rule
                .as_deref()
                .is_none_or(|rule| rule == rules::NO_COMMENTS.as_str())
            {
                marker.used = true;
            }
        }
    }

    for candidate in candidates {
        let rule = candidate.finding.rule;
        let line = candidate.finding.location.line;
        let chosen = markers
            .iter()
            .enumerate()
            .filter(|(_, marker)| marker_covers(marker, rule, line, candidate.line_suppressible))
            .min_by_key(|(_, marker)| marker_priority(marker));
        if let Some((index, _)) = chosen {
            markers[index].used = true;
            result.suppressed += 1;
        } else {
            result.findings.push(candidate.finding);
        }
    }

    if fail_on_unused && language.id != "markdown" {
        for marker in markers {
            if marker.used || !eligible(&marker, enabled_rules, applicable_rules) {
                continue;
            }
            let keyword = if marker.file_level {
                FILE_MARKER
            } else {
                LINE_MARKER
            };
            let matched = marker
                .rule
                .as_deref()
                .map(|rule| format!("{keyword}:{rule}"))
                .unwrap_or_else(|| keyword.to_string());
            result.findings.push(Finding::new(
                rules::UNUSED_MARKER,
                Severity::Error,
                Location::point(path, marker.line, 1),
                matched,
                "suppression marker did not suppress a finding",
            ));
        }
    }
    result
}

fn eligible(
    marker: &Marker,
    enabled_rules: &HashSet<RuleKey>,
    applicable_rules: &HashSet<RuleKey>,
) -> bool {
    match marker.rule.as_deref() {
        Some(rule) => enabled_rules
            .iter()
            .any(|key| key.as_str() == rule && applicable_rules.contains(key)),
        None => enabled_rules
            .iter()
            .any(|rule| applicable_rules.contains(rule)),
    }
}

fn marker_priority(marker: &Marker) -> (bool, bool) {
    (marker.file_level, marker.rule.is_none())
}

fn marker_covers(marker: &Marker, rule: RuleKey, line: usize, line_suppressible: bool) -> bool {
    if !marker.file_level && (!line_suppressible || marker.line != line) {
        return false;
    }
    marker
        .rule
        .as_deref()
        .is_none_or(|only| only == rule.as_str())
}

fn collect_markers(text: &str, language: &LanguageProfile) -> Vec<Marker> {
    let mut markers = Vec::new();
    for (index, line) in text.lines().enumerate() {
        collect_from_line(line, index + 1, language, &mut markers);
    }
    markers
}

fn collect_from_line(
    line: &str,
    line_number: usize,
    language: &LanguageProfile,
    out: &mut Vec<Marker>,
) {
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(LINE_MARKER) {
        let position = cursor + relative;
        if !looks_like_comment_directive(line, position, language) {
            cursor = position + LINE_MARKER.len();
            continue;
        }
        let rest = &line[position + LINE_MARKER.len()..];
        let file_level = rest.starts_with("-file");
        let suffix = if file_level {
            &rest["-file".len()..]
        } else {
            rest
        };
        let rule = suffix.strip_prefix(':').and_then(|suffix| {
            let id: String = suffix
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
                .collect();
            (!id.is_empty()).then_some(id)
        });
        out.push(Marker {
            line: line_number,
            file_level,
            rule,
            used: false,
        });
        cursor = position + LINE_MARKER.len();
    }
}

fn looks_like_comment_directive(line: &str, position: usize, language: &LanguageProfile) -> bool {
    let before = &line[..position];
    let has_comment_opener = ["//", "/*", "#", "--", "<!--"]
        .iter()
        .any(|opener| before.rfind(opener).is_some());
    if !has_comment_opener {
        return false;
    }
    let track_single = language.id != "rust";
    !inside_quote(before, '"')
        && !inside_quote(before, '`')
        && (!track_single || !inside_quote(before, '\''))
}

fn inside_quote(value: &str, quote: char) -> bool {
    let mut open = false;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            open = !open;
        }
    }
    open
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use entl_codebase::language_profile;

    use crate::finding::{Finding, Location, Severity};
    use crate::rule::{Candidate, RuleKey};

    use super::apply;

    fn candidate(rule: RuleKey, line: usize) -> Candidate {
        Candidate::line(Finding::new(
            rule,
            Severity::Error,
            Location::point("test.ts", line, 1),
            "x",
            "message",
        ))
    }

    #[test]
    fn scoped_marker_suppresses_only_its_rule() {
        let enabled = HashSet::from([RuleKey::new("color"), RuleKey::new("emoji")]);
        let result = apply(
            "const x = '#fff'; // straitjacket-allow:color\n",
            "test.ts",
            language_profile("typescript").unwrap(),
            vec![
                candidate(RuleKey::new("color"), 1),
                candidate(RuleKey::new("emoji"), 1),
            ],
            &enabled,
            &enabled,
            true,
        );
        assert_eq!(result.suppressed, 1);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].rule, RuleKey::new("emoji"));
    }

    #[test]
    fn dead_marker_becomes_a_finding_without_rescanning() {
        let enabled = HashSet::from([RuleKey::new("color")]);
        let result = apply(
            "const x = token; // straitjacket-allow:color\n",
            "test.ts",
            language_profile("typescript").unwrap(),
            vec![],
            &enabled,
            &enabled,
            true,
        );
        assert_eq!(result.findings[0].rule, RuleKey::new("unused-marker"));
    }

    #[test]
    fn marker_text_inside_a_string_is_data() {
        let enabled = HashSet::from([RuleKey::new("color")]);
        let result = apply(
            "const x = \"// straitjacket-allow:color\";\n",
            "test.ts",
            language_profile("typescript").unwrap(),
            vec![],
            &enabled,
            &enabled,
            true,
        );
        assert!(result.findings.is_empty());
    }
}
