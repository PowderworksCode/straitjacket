use serde::Serialize;

use crate::rule::RuleKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    pub path: String,
    pub line: usize,
    pub col: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<usize>,
}

impl Location {
    pub fn point(path: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            path: path.into(),
            line,
            col,
            end_line: None,
            end_col: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelatedLocation {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceStep {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: RuleKey,
    pub severity: Severity,
    pub location: Location,
    pub matched: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<RelatedLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceStep>,
}

impl Finding {
    pub fn new(
        rule: RuleKey,
        severity: Severity,
        location: Location,
        matched: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            severity,
            location,
            matched: matched.into(),
            message: message.into(),
            help: None,
            related: Vec::new(),
            evidence: Vec::new(),
        }
    }
}
