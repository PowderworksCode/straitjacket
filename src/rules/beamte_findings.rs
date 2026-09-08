//! How a beamte finding reads once straitjacket owns it.
//!
//! Two rules host beamte -- `test-quality` over test files, `env-vars` over
//! every file -- and a finding must read the same way whichever door it came
//! through. One place formats them, so the two cannot drift.

use crate::finding::{EvidenceStep, Finding, Location, Severity};
use crate::rule::{Candidate, RuleKey};

/// A beamte finding as a straitjacket candidate.
///
/// The beamte rule is named in the message rather than in the key, because
/// straitjacket registers one rule for a family of them and `test-logic`
/// would be a key nothing in its manifest declares. Beamte's DESIGN.md §6.3
/// puts citing the post on the host: it turns an argument with a linter into
/// a much shorter argument with Titus Winters.
pub fn candidate(
    key: RuleKey,
    severity: Severity,
    path: &str,
    text: &str,
    finding: beamte::Finding,
) -> Candidate {
    let line = finding.span.line;
    let column = finding.span.column;
    Candidate::line(Finding {
        rule: key,
        severity,
        location: Location::point(path, line, column),
        matched: matched_text(text, line),
        message: format!("{}: {}", finding.rule, finding.message),
        help: help_of(&finding),
        related: Vec::new(),
        evidence: finding
            .evidence
            .into_iter()
            .map(|step| EvidenceStep {
                location: Location::point(path, step.span.line, step.span.column),
                message: step.message,
            })
            .collect(),
    })
}

/// A file that could not be checked, said out loud.
///
/// Beamte's DESIGN.md §7.3: a file that was not read is reported as unread,
/// never as clean. Returning nothing would mean a failed pack fetch reads
/// exactly like a file with nothing wrong in it.
pub fn not_read(key: RuleKey, path: &str, message: String, help: Option<String>) -> Candidate {
    Candidate::file(Finding {
        rule: key,
        severity: Severity::Warning,
        location: Location::point(path, 1, 1),
        matched: String::new(),
        message,
        help,
        related: Vec::new(),
        evidence: Vec::new(),
    })
}

fn help_of(finding: &beamte::Finding) -> Option<String> {
    let citation = beamte::rule(finding.rule.as_str()).map(|rule| rule.citation);
    match (&finding.help, citation) {
        (Some(help), Some(citation)) => {
            Some(format!("{help} — {} ({})", citation.title, citation.url))
        }
        (Some(help), None) => Some(help.clone()),
        (None, Some(citation)) => Some(format!("{} ({})", citation.title, citation.url)),
        (None, None) => None,
    }
}

/// The line a finding sits on, trimmed, for the `matched` field every other
/// rule fills in from its own regex.
fn matched_text(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .to_string()
}
