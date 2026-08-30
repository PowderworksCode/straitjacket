mod beamte_findings;
mod color;
mod comments;
mod deep_nesting;
mod emoji;
mod env_vars;
mod file_size;
mod inline_font;
mod inline_svg;
mod key;
mod motion;
mod no_comments;
mod regex_rule;
mod stray_todo;
mod test_quality;
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

/// Rules Straitjacket used to have.
///
/// Every one of them read facts from a pack that was never published, so none
/// of them could run outside the repository that built them. They are listed
/// so that a configuration naming one fails saying the rule was withdrawn,
/// rather than saying the key is unknown, which reads like a typo.
pub const REMOVED: &[&str] = &[
    "analysis-incomplete",
    "effect-barrier",
    "effect-capability",
    "error-discard",
    "exact-clone",
    "library-opportunity",
    "near-clone",
    "unknown-barrier",
];

pub fn resolve(names: &[String]) -> anyhow::Result<Vec<RuleKey>> {
    let registrations = registrations()?;
    let mut resolved = Vec::with_capacity(names.len());
    let mut removed = Vec::new();
    let mut unknown = Vec::new();
    for name in names {
        match registrations
            .iter()
            .find(|registration| registration.key.as_str() == name)
        {
            Some(registration) => resolved.push(registration.key),
            None if REMOVED.contains(&name.as_str()) => removed.push(name.as_str()),
            None => unknown.push(name.as_str()),
        }
    }
    if !removed.is_empty() {
        bail!(
            "rule(s) Straitjacket no longer has: {}. Remove them from the configuration.",
            removed.join(", ")
        );
    }
    if !unknown.is_empty() {
        bail!("unknown rule key(s): {}", unknown.join(", "));
    }
    Ok(resolved)
}

/// The beamte rules a configuration named, rejecting anything beamte does not
/// have.
///
/// Straitjacket carries one rule key for all of them, so these names never
/// reach [`resolve`] and would otherwise be accepted silently -- a typo in
/// `test-rules` would quietly turn a rule off rather than say so. A rule
/// whose beamte scope is `File` is rejected by name too: it runs under
/// `env-vars`, over every file, and listing it here would run it over test
/// files alone while looking like it ran.
pub fn resolve_test_rules(names: &[String]) -> anyhow::Result<()> {
    let mut unknown = Vec::new();
    let mut misfiled = Vec::new();
    for name in names {
        match beamte::rule(name) {
            None => unknown.push(name.as_str()),
            Some(rule) if rule.scope == beamte::Scope::File => misfiled.push(name.as_str()),
            Some(_) => {}
        }
    }
    if !misfiled.is_empty() {
        bail!(
            "{} runs over every file, not only tests: enable `env-vars` instead \
             of naming it in `test-rules`.",
            misfiled.join(", ")
        );
    }
    if !unknown.is_empty() {
        let known: Vec<&str> = beamte::catalogue()
            .iter()
            .filter(|rule| rule.scope == beamte::Scope::Tests)
            .map(|rule| rule.id.as_str())
            .collect();
        bail!(
            "unknown test rule(s): {}. Straitjacket has: {}.",
            unknown.join(", "),
            known.join(", ")
        );
    }
    Ok(())
}

pub use env_vars::KEY as ENV_VARS;
pub use no_comments::KEY as NO_COMMENTS;
pub use test_quality::KEY as TEST_QUALITY;
pub use unused_marker::{KEY as UNUSED_MARKER, descriptor as unused_marker_descriptor};
