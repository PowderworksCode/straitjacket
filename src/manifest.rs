// The machine-readable description of what this binary enforces: every rule it
// carries, every rule it used to carry, and the tunable defaults. It exists so
// that documentation can be checked against the scanner instead of maintained
// alongside it, which is the drift that shipped a website describing rules the
// binary never had.

use serde::Serialize;

use crate::config::Settings;
use crate::rule::RuleDescriptor;
use crate::rules;

pub const SCHEMA: &str = "straitjacket.rules/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rule {
    pub id: String,
    pub summary: String,
    pub default_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Defaults {
    pub max_lines: usize,
    pub max_nesting: usize,
    pub no_comments: bool,
    pub include_json: bool,
    pub no_ignore: bool,
    pub no_fail: bool,
    pub fail_on_unused_markers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    pub schema: &'static str,
    pub version: &'static str,
    pub rules: Vec<Rule>,
    pub removed: &'static [&'static str],
    pub defaults: Defaults,
}

impl Manifest {
    pub fn build(descriptors: &[RuleDescriptor], settings: &Settings) -> Self {
        let mut rules: Vec<Rule> = descriptors
            .iter()
            .map(|descriptor| Rule {
                id: descriptor.id.as_str().to_string(),
                summary: descriptor.summary.to_string(),
                default_enabled: descriptor.default_enabled,
            })
            .collect();
        rules.sort_by(|left, right| left.id.cmp(&right.id));
        rules.dedup_by(|left, right| left.id == right.id);

        Self {
            schema: SCHEMA,
            version: env!("CARGO_PKG_VERSION"),
            rules,
            removed: rules::REMOVED,
            defaults: Defaults {
                max_lines: settings.max_lines,
                max_nesting: settings.max_nesting,
                no_comments: settings.no_comments,
                include_json: settings.include_json,
                no_ignore: settings.no_ignore,
                no_fail: settings.no_fail,
                fail_on_unused_markers: settings.fail_on_unused_markers,
            },
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for rule in &self.rules {
            let default = if rule.default_enabled {
                "default"
            } else {
                "opt-in"
            };
            out.push_str(&format!("{} ({default})\n    {}\n", rule.id, rule.summary));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Scanner;

    fn manifest() -> Manifest {
        let settings = Settings::default();
        let scanner = Scanner::new(&settings).expect("scanner builds from default settings");
        let descriptors = scanner.descriptors();
        Manifest::build(&descriptors, &settings)
    }

    #[test]
    fn every_rule_has_an_id_and_a_summary() {
        for rule in &manifest().rules {
            assert!(!rule.id.is_empty(), "rule id is empty");
            assert!(!rule.summary.is_empty(), "{} has no summary", rule.id);
        }
    }

    #[test]
    fn rules_are_sorted_and_unique() {
        let manifest = manifest();
        let ids: Vec<&str> = manifest.rules.iter().map(|rule| rule.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids, sorted, "manifest rules are not sorted and unique");
    }

    #[test]
    fn a_withdrawn_rule_is_never_also_a_live_one() {
        let manifest = manifest();
        for removed in manifest.removed {
            assert!(
                !manifest.rules.iter().any(|rule| rule.id == *removed),
                "{removed} is listed as both live and withdrawn"
            );
        }
    }

    #[test]
    fn defaults_match_the_documented_constants() {
        let defaults = manifest().defaults;
        assert_eq!(defaults.max_lines, crate::config::DEFAULT_MAX_LINES);
        assert_eq!(defaults.max_nesting, crate::config::DEFAULT_MAX_NESTING);
    }

    #[test]
    fn json_round_trips_to_the_same_rule_set() {
        let manifest = manifest();
        let json = manifest.to_json().expect("manifest serializes");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("manifest is valid JSON");
        let ids: Vec<&str> = parsed["rules"]
            .as_array()
            .expect("rules is an array")
            .iter()
            .map(|rule| rule["id"].as_str().expect("id is a string"))
            .collect();
        let expected: Vec<&str> = manifest.rules.iter().map(|rule| rule.id.as_str()).collect();
        assert_eq!(ids, expected);
        assert_eq!(parsed["schema"], SCHEMA);
    }
}
