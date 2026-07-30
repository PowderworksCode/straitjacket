mod color;
mod comments;
mod deep_nesting;
mod emoji;
mod file_size;
mod inline_font;
mod inline_svg;
mod key;
mod motion;
mod no_comments;
mod regex_rule;
mod stray_todo;
mod unused_marker;

use anyhow::bail;

use crate::config::Settings;
use crate::rule::FileRule;

pub use key::RuleKey;

pub use deep_nesting::DeepNestingRule;
pub use emoji::EmojiRule;
pub use file_size::FileSizeRule;

pub type RuleFactory = fn(&Settings) -> Box<dyn FileRule>;
pub type RuleInstruction = fn(&Settings) -> String;

pub struct RuleRegistration {
    pub key: RuleKey,
    pub factory: Option<RuleFactory>,
    pub instruction: RuleInstruction,
}

inventory::collect!(RuleRegistration);

fn registrations() -> anyhow::Result<Vec<&'static RuleRegistration>> {
    let mut registrations: Vec<_> = inventory::iter::<RuleRegistration>.into_iter().collect();
    registrations.sort_by_key(|registration| registration.key);
    for pair in registrations.windows(2) {
        if pair[0].key == pair[1].key {
            bail!("duplicate rule inventory key `{}`", pair[0].key);
        }
    }
    for registration in &registrations {
        let name = registration.key.as_str();
        if name.is_empty()
            || name.starts_with('-')
            || name.ends_with('-')
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            bail!("invalid rule inventory key `{name}`");
        }
    }
    Ok(registrations)
}

pub fn builtins(settings: &Settings) -> anyhow::Result<Vec<Box<dyn FileRule>>> {
    let registrations = registrations()?;
    let mut rules = Vec::new();
    for registration in registrations {
        let Some(factory) = registration.factory else {
            continue;
        };
        let rule = factory(settings);
        if rule.descriptor().id != registration.key {
            bail!(
                "rule inventory registered {} but factory built {}",
                registration.key,
                rule.descriptor().id
            );
        }
        rules.push(rule);
    }
    Ok(rules)
}

pub fn instruction(key: RuleKey, settings: &Settings) -> anyhow::Result<String> {
    let registration = registrations()?
        .into_iter()
        .find(|registration| registration.key == key)
        .expect("validated inventory contains the rule key");
    Ok((registration.instruction)(settings))
}

pub fn resolve(names: &[String]) -> anyhow::Result<Vec<RuleKey>> {
    let registrations = registrations()?;
    let mut resolved = Vec::with_capacity(names.len());
    let mut unknown = Vec::new();
    for name in names {
        match registrations
            .iter()
            .find(|registration| registration.key.as_str() == name)
        {
            Some(registration) => resolved.push(registration.key),
            None => unknown.push(name.as_str()),
        }
    }
    if !unknown.is_empty() {
        bail!("unknown rule key(s): {}", unknown.join(", "));
    }
    Ok(resolved)
}

pub use no_comments::KEY as NO_COMMENTS;
pub use unused_marker::{KEY as UNUSED_MARKER, descriptor as unused_marker_descriptor};
