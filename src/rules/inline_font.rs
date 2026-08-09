use crate::language::STYLE_HOST;
use regex::Captures;

use crate::Settings;
use crate::rule::{FileRule, RuleKey};
use crate::rules::RuleRegistration;
use crate::rules::regex_rule::RegexRule;

const KEY: RuleKey = RuleKey::new("inline-font");
fn font(captures: &Captures<'_>) -> Option<String> {
    let raw = captures.get(1)?.as_str().trim();
    let value = match raw.split_once(',') {
        Some((head, tail)) if tail.contains(':') => head.trim(),
        _ => raw.trim_end_matches(',').trim(),
    };
    let lower = value.to_ascii_lowercase();
    let unquoted = lower
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            lower
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(&lower);
    let keyword = matches!(
        lower.as_str(),
        "inherit" | "initial" | "unset" | "revert" | ""
    );
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '-'));
    if unquoted.starts_with("var(") || keyword || bare {
        None
    } else {
        Some(value.to_string())
    }
}

fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(RegexRule::new(
        KEY,
        "inline font-family literal stack",
        &STYLE_HOST,
        r"(?i)(?:font-family|fontFamily)\s*:\s*([^;}\n]+)",
        font,
        "define the font once and reference a token or CSS variable",
    ))
}

fn instruction(_: &Settings) -> String {
    "Literal font stacks are not allowed. Define fonts centrally and reference a token or CSS variable."
        .into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}
