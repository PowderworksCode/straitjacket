use crate::language::COMPONENT_HOST;

use crate::Settings;
use crate::rule::{FileRule, RuleKey};
use crate::rules::RuleRegistration;
use crate::rules::regex_rule::{RegexRule, whole};

const KEY: RuleKey = RuleKey::new("inline-svg");
fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(RegexRule::new(
        KEY,
        "inline SVG in component source",
        &COMPONENT_HOST,
        r#"<svg[\s/>]|createElement\(\s*["']svg["']"#,
        whole,
        "extract it into a named, reusable icon component",
    ))
}

fn instruction(_: &Settings) -> String {
    "Inline SVG is not allowed in component source. Put it in a named reusable icon component."
        .into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}
