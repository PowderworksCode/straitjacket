use entl_codebase::{LanguageProfile, LanguageRole};
use unic_emoji_char::{is_emoji, is_emoji_presentation};

use crate::Settings;
use crate::finding::{Finding, Location, Severity};
use crate::rule::{Candidate, FileRule, RuleDescriptor, RuleKey, SourceFile};
use crate::rules::RuleRegistration;

const VS16: char = '\u{FE0F}';
const KEY: RuleKey = RuleKey::new("emoji");
pub struct EmojiRule;

fn build(_: &Settings) -> Box<dyn FileRule> {
    Box::new(EmojiRule)
}

fn instruction(_: &Settings) -> String {
    "Emoji glyphs are not allowed. Use text labels or named icons.".into()
}

inventory::submit! {
    RuleRegistration {
        key: KEY,
        factory: Some(build),
        repository_factory: None,
        instruction,
    }
}

impl FileRule for EmojiRule {
    fn descriptor(&self) -> RuleDescriptor {
        RuleDescriptor {
            id: KEY,
            summary: "emoji glyph in source; use a text label or named icon",
            default_enabled: true,
        }
    }

    fn applies_to(&self, language: &LanguageProfile) -> bool {
        language.role != LanguageRole::Data
    }

    fn check(&self, file: SourceFile<'_>, candidates: &mut Vec<Candidate>) {
        for (line_index, line) in file.text.lines().enumerate() {
            let chars: Vec<_> = line.char_indices().collect();
            for (index, &(byte, ch)) in chars.iter().enumerate() {
                if ch == VS16 {
                    continue;
                }
                let followed_by_vs16 = chars.get(index + 1).is_some_and(|&(_, next)| next == VS16);
                let regional = ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch);
                if !(regional || is_emoji_presentation(ch) || followed_by_vs16 && is_emoji(ch)) {
                    continue;
                }
                let mut matched = ch.to_string();
                if followed_by_vs16 {
                    matched.push(VS16);
                }
                candidates.push(Candidate::line(Finding::new(
                    KEY,
                    Severity::Error,
                    Location::point(file.path, line_index + 1, byte + 1),
                    matched,
                    "emoji glyph in source renders inconsistently and obscures intent",
                )));
            }
        }
    }
}
