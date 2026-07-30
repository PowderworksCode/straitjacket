use entl_codebase::STYLE_HOST;
use regex::Captures;

use crate::Settings;
use crate::rule::{FileRule, RuleKey};
use crate::rules::RuleRegistration;
use crate::rules::regex_rule::RegexRule;

const KEY: RuleKey = RuleKey::new("motion");
fn motion(captures: &Captures<'_>) -> Option<String> {
    Some(captures[0].trim_end_matches([' ', ':']).to_string())
}

fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(RegexRule::new(
        KEY,
        "ad-hoc transition or animation",
        &STYLE_HOST,
        r"\b(?:transition|animation)(?:-[a-z-]+)?\s*:|@keyframes\b",
        motion,
        "centralize motion so it can be tuned or disabled consistently",
    ))
}

fn instruction(_: &Settings) -> String {
    "Ad-hoc transitions and animations are not allowed. Use the repository's centralized motion system."
        .into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}
