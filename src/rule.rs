use crate::finding::Finding;
pub use crate::rules::RuleKey;
use entl_codebase::LanguageProfile;

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
}

impl Candidate {
    pub fn line(finding: Finding) -> Self {
        Self {
            finding,
            line_suppressible: true,
        }
    }

    pub fn file(finding: Finding) -> Self {
        Self {
            finding,
            line_suppressible: false,
        }
    }
}

pub trait FileRule: Send + Sync {
    fn descriptor(&self) -> RuleDescriptor;
    fn applies_to(&self, language: &LanguageProfile) -> bool;
    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>);
}
