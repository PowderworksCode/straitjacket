use std::collections::HashMap;

use entl_codebase::{LanguageProfile, STRUCTURED_CODE};

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

const KEY: RuleKey = RuleKey::new("deep-nesting");
pub struct DeepNestingRule {
    max_depth: usize,
}

impl DeepNestingRule {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

fn build(settings: &Settings) -> Box<dyn FileRule> {
    Box::new(DeepNestingRule::new(settings.max_nesting))
}

fn instruction(settings: &Settings) -> String {
    format!(
        "Nesting deeper than {} indentation levels is not allowed. Extract or flatten the logic.",
        settings.max_nesting
    )
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        repository_factory: None,
        instruction,
    }
}

impl FileRule for DeepNestingRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "code exceeds the configured indentation-depth budget",
            default_enabled: self.max_depth > 0,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        language.has_facet(&STRUCTURED_CODE)
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        if self.max_depth == 0 {
            return;
        }
        let tabs = tab_indented(file.text);
        let unit = if tabs { 1 } else { space_unit(file.text) };
        let mut in_over_budget_run = false;
        for (index, line) in file.text.lines().enumerate() {
            let Some(whitespace) = leading(line) else {
                continue;
            };
            let depth = if tabs {
                whitespace.chars().take_while(|&ch| ch == '\t').count()
            } else {
                whitespace.chars().take_while(|&ch| ch == ' ').count() / unit
            };
            if depth <= self.max_depth {
                in_over_budget_run = false;
                continue;
            }
            if in_over_budget_run {
                continue;
            }
            let mut finding = Finding::new(
                KEY,
                Severity::Error,
                Location::point(file.path, index + 1, whitespace.len() + 1),
                format!("nesting depth {depth}"),
                format!(
                    "line is nested {depth} levels deep, over the {}-level limit",
                    self.max_depth
                ),
            );
            finding.help = Some("extract or flatten the deeply nested logic".into());
            candidates.push(Candidate::line(finding));
            in_over_budget_run = true;
        }
    }
}

fn leading(line: &str) -> Option<&str> {
    let length = line.len() - line.trim_start().len();
    (length != line.len()).then(|| &line[..length])
}

fn tab_indented(text: &str) -> bool {
    let (mut tabs, mut spaces) = (0, 0);
    for whitespace in text.lines().filter_map(leading) {
        match whitespace.chars().next() {
            Some('\t') => tabs += 1,
            Some(' ') => spaces += 1,
            _ => {}
        }
    }
    tabs > spaces
}

fn space_unit(text: &str) -> usize {
    let mut counts = HashMap::<usize, usize>::new();
    let mut previous = None;
    for whitespace in text
        .lines()
        .filter_map(leading)
        .filter(|whitespace| whitespace.bytes().all(|byte| byte == b' '))
    {
        if let Some(before) = previous
            && whitespace.len() > before
        {
            *counts.entry(whitespace.len() - before).or_default() += 1;
        }
        previous = Some(whitespace.len());
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then(right.0.cmp(&left.0)))
        .map(|(step, _)| step)
        .filter(|&step| step > 0)
        .unwrap_or(4)
}
