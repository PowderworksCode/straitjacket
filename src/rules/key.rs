use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct RuleKey(&'static str);

impl RuleKey {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for RuleKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::RuleKey;

    #[test]
    fn rule_keys_serialize_as_names() {
        let key = RuleKey::new("example-rule");
        assert_eq!(key.as_str(), "example-rule");
        assert_eq!(serde_json::to_string(&key).unwrap(), "\"example-rule\"");
    }
}
