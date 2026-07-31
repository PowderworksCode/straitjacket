use crate::finding::Finding;
pub use crate::rules::RuleKey;
use entl_codebase::LanguageProfile;
use infact_analysis::{AnalysisSelection, FactBatch};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleDescriptor {
    pub id: RuleKey,
    pub summary: &'static str,
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SourceFile<'a> {
    pub path: &'a str,
    pub language: &'a LanguageProfile,
    pub text: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub finding: Finding,
    pub line_suppressible: bool,
    pub suppression_locations: Vec<crate::finding::Location>,
}

impl Candidate {
    pub fn line(finding: Finding) -> Self {
        let suppression_locations = vec![finding.location.clone()];
        Self {
            finding,
            line_suppressible: true,
            suppression_locations,
        }
    }

    pub fn file(finding: Finding) -> Self {
        let suppression_locations = vec![finding.location.clone()];
        Self {
            finding,
            line_suppressible: false,
            suppression_locations,
        }
    }

    pub fn lines(finding: Finding, additional: Vec<crate::finding::Location>) -> Self {
        let mut candidate = Self::line(finding);
        candidate.suppression_locations.extend(additional);
        candidate
    }
}

pub trait FileRule: Send + Sync {
    fn descriptor(&self) -> RuleDescriptor;
    fn applies_to(&self, language: &LanguageProfile) -> bool;
    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>);
}

pub trait RepositoryRule: Send + Sync {
    fn descriptor(&self) -> RuleDescriptor;
    fn select_analysis(&self, selection: &mut AnalysisSelection);
    fn check(&self, facts: &FactBatch, display_root: &Path, candidates: &mut Vec<Candidate>);
}
