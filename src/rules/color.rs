use entl_codebase::STYLE_HOST;

use crate::Settings;
use crate::rule::{FileRule, RuleKey};
use crate::rules::RuleRegistration;
use crate::rules::regex_rule::{RegexRule, whole};

const KEY: RuleKey = RuleKey::new("color");
fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(
        RegexRule::new(
            KEY,
            "hardcoded color literal",
            &STYLE_HOST,
            r"#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{4}|[0-9a-fA-F]{3})\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch)\([^\)\n]*\)|\bcolor\(\s*(?:from|srgb(?:-linear)?|display-p3|a98-rgb|prophoto-rgb|rec2020|xyz(?:-d50|-d65)?)\b[^\)\n]*\)",
            whole,
            "use a theme token or CSS variable",
        )
        .excluding(settings.theme_files.clone()),
    )
}

fn instruction(settings: &Settings) -> String {
    if settings.theme_files.is_empty() {
        return "Hardcoded colors are not allowed. Designate theme or palette files with `theme-files`, define color tokens there, and reference those tokens elsewhere.".into();
    }
    let files = settings
        .theme_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Hardcoded colors are not allowed outside {files}. Define color tokens there and reference those tokens or CSS variables elsewhere."
    )
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        instruction,
    }
}
