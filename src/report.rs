use std::collections::HashMap;

use clap::ValueEnum;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::finding::{Finding, Severity};
use crate::rule::{RuleDescriptor, RuleKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}

pub fn render(
    format: OutputFormat,
    findings: &[Finding],
    descriptors: &[RuleDescriptor],
    version: &str,
) -> String {
    match format {
        OutputFormat::Text => text(findings),
        OutputFormat::Json => {
            serde_json::to_string_pretty(findings).unwrap_or_else(|_| "[]".into())
        }
        OutputFormat::Sarif => sarif(findings, descriptors, version),
    }
}

pub fn text(findings: &[Finding]) -> String {
    let mut output = String::new();
    for finding in findings {
        let warning = if finding.severity == Severity::Warning {
            " (warn)"
        } else {
            ""
        };
        output.push_str(&format!(
            "{}:{}:{}  [{}]{}  {}\n",
            finding.location.path,
            finding.location.line,
            finding.location.col,
            finding.rule,
            warning,
            finding.matched
        ));
        output.push_str(&format!("  {}\n", finding.message));
        if let Some(help) = &finding.help {
            output.push_str(&format!("  help: {help}\n"));
        }
        for related in &finding.related {
            output.push_str(&format!(
                "  related: {}:{}:{}: {}\n",
                related.location.path, related.location.line, related.location.col, related.message
            ));
        }
        for step in &finding.evidence {
            output.push_str(&format!(
                "  via: {}:{}:{}: {}\n",
                step.location.path, step.location.line, step.location.col, step.message
            ));
        }
    }
    output
}

pub fn sarif(findings: &[Finding], descriptors: &[RuleDescriptor], version: &str) -> String {
    let mut rules = Vec::<Value>::new();
    let mut indexes = HashMap::<RuleKey, usize>::new();
    for descriptor in descriptors {
        indexes.insert(descriptor.id, rules.len());
        rules.push(json!({
            "id": descriptor.id,
            "shortDescription": { "text": descriptor.summary },
            "defaultConfiguration": {
                "level": if descriptor.default_enabled { "error" } else { "none" }
            }
        }));
    }

    let results: Vec<_> = findings
        .iter()
        .map(|finding| {
            let mut result = json!({
                "ruleId": finding.rule,
                "ruleIndex": indexes.get(&finding.rule).copied().unwrap_or(0),
                "level": match finding.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                },
                "message": { "text": finding.message },
                "locations": [{ "physicalLocation": physical_location(&finding.location) }],
                "properties": {
                    "matched": finding.matched,
                    "help": finding.help,
                }
            });
            if !finding.related.is_empty() {
                result["relatedLocations"] = Value::Array(
                    finding
                        .related
                        .iter()
                        .enumerate()
                        .map(|(id, related)| {
                            json!({
                                "id": id,
                                "physicalLocation": physical_location(&related.location),
                                "message": { "text": related.message },
                            })
                        })
                        .collect(),
                );
            }
            if !finding.evidence.is_empty() {
                result["codeFlows"] = json!([{
                    "threadFlows": [{
                        "locations": finding.evidence.iter().map(|step| json!({
                            "location": {
                                "physicalLocation": physical_location(&step.location),
                                "message": { "text": step.message },
                            }
                        })).collect::<Vec<_>>()
                    }]
                }]);
            }
            result
        })
        .collect();

    serde_json::to_string_pretty(&json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "straitjacket",
                    "semanticVersion": version,
                    "informationUri": "https://github.com/zmaril/straitjacket",
                    "rules": rules,
                }
            },
            "results": results,
        }]
    }))
    .unwrap_or_else(|_| "{}".into())
}

fn physical_location(location: &crate::finding::Location) -> Value {
    let mut region = json!({
        "startLine": location.line,
        "startColumn": location.col,
    });
    if let Some(end_line) = location.end_line {
        region["endLine"] = json!(end_line);
    }
    if let Some(end_col) = location.end_col {
        region["endColumn"] = json!(end_col);
    }
    json!({
        "artifactLocation": { "uri": location.path },
        "region": region,
    })
}

#[cfg(test)]
mod tests {
    use crate::finding::{EvidenceStep, Finding, Location, Severity};
    use crate::rule::{RuleDescriptor, RuleKey};

    use super::sarif;

    #[test]
    fn sarif_preserves_evidence_as_a_code_flow() {
        let mut finding = Finding::new(
            RuleKey::new("color"),
            Severity::Error,
            Location::point("src/a.rs", 2, 3),
            "x",
            "example finding",
        );
        finding.evidence.push(EvidenceStep {
            location: Location::point("src/b.rs", 4, 5),
            message: "called here".into(),
        });
        let output = sarif(
            &[finding],
            &[RuleDescriptor {
                id: RuleKey::new("color"),
                summary: "example",
                default_enabled: true,
            }],
            "0.1.0",
        );
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(
            value["runs"][0]["results"][0]["codeFlows"][0]["threadFlows"][0]["locations"][0]["location"]
                ["physicalLocation"]["region"]["startLine"],
            4
        );
    }
}
