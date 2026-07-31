use crate::Settings;
use crate::rule::{RuleDescriptor, RuleKey};
use crate::rules::RuleRegistration;

pub const KEY: RuleKey = RuleKey::new("unused-marker");

pub fn descriptor() -> RuleDescriptor {
    RuleDescriptor {
        id: KEY,
        summary: "suppression marker did not suppress a finding",
        default_enabled: true,
    }
}

fn instruction(_: &Settings) -> String {
    "Unused suppression markers are not allowed. Remove markers that no longer suppress a finding."
        .into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: None,
        repository_factory: None,
        instruction,
    }
}
