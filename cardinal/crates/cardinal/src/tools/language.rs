//! Whether a reply is in the conversation language (`user_language` of `.claude/cardinal.local.toml`, else of
//! `.claude/cardinal.toml`): the test of rule `user-language` (BRE-10, `specs/rule-breaches.md`), shared by
//! `rule-breaches` and the `language-hint` hook (`specs/language-hint.md`) so the measure and the hook agree on what a breach is.

use serde_json::Value;
use std::path::Path;

/// Frequent words of one language only (`a`, `an`, `as`, `on`, `or`, `plus` are both French and English).
const FRENCH_WORDS: &[&str] = &[
    "le", "la", "les", "un", "une", "des", "du", "de", "et", "est", "que", "qui", "dans", "pour",
    "pas", "sur", "avec", "ce", "cette", "il", "elle", "je", "tu", "nous", "vous", "au", "aux",
    "ou", "mais", "par", "sont", "fait", "sans", "comme", "se", "ne", "en", "donc", "puis",
];
const ENGLISH_WORDS: &[&str] = &[
    "the", "and", "is", "are", "that", "which", "in", "for", "not", "with", "this", "it", "i",
    "you", "we", "of", "to", "be", "by", "was", "but", "at", "from", "have", "has", "so", "then",
    "will", "can",
];

/// The words every hint of the `language-hint` hook holds, by which `rule-breaches` finds it in a transcript.
pub(super) const HINT_MARK: &str = "the user_language of .claude/cardinal.toml";

/// Whether a transcript line is a hint of the `language-hint` hook: the context its withdrawn `PostToolUse` mode added
/// after a tool call, or the feedback of its Stop hook.
pub(super) fn is_hint(line: &Value) -> bool {
    let attachment = &line["attachment"];
    let added = line["type"] == "attachment"
        && attachment["type"] == "hook_additional_context"
        && attachment["content"].to_string().contains(HINT_MARK);
    let feedback = line["type"] == "user"
        && line["isMeta"].as_bool() == Some(true)
        && line["message"]["content"]
            .as_str()
            .is_some_and(|t| t.starts_with("Stop hook feedback:") && t.contains(HINT_MARK));
    added || feedback
}

/// The fewest words a text needs for its language to be judged.
const MIN_WORDS: usize = 20;

/// A conversation language: its name in `user_language`, its stop words, then those of the other language.
#[derive(Clone, Copy)]
pub(super) struct Language {
    pub(super) name: &'static str,
    own: &'static [&'static str],
    other: &'static [&'static str],
}

impl Language {
    /// The language named by `user_language`, in any case and blanks around it ignored; `None` for a language with no list.
    pub(super) fn named(name: &str) -> Option<Language> {
        match name.trim().to_lowercase().as_str() {
            "french" => Some(Language {
                name: "French",
                own: FRENCH_WORDS,
                other: ENGLISH_WORDS,
            }),
            "english" => Some(Language {
                name: "English",
                own: ENGLISH_WORDS,
                other: FRENCH_WORDS,
            }),
            _ => None,
        }
    }

    /// The language named by the personal configuration of the project at `root` when it is valid TOML and names one,
    /// else by the shared configuration `shared`; `None` when the one that names it has no stop words (LNG-12, BRE-14).
    pub(super) fn of(root: &Path, shared: Option<&toml::Table>) -> Option<Language> {
        let personal = std::fs::read_to_string(root.join(crate::PERSONAL_CONFIG))
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok());
        let name = personal
            .as_ref()
            .and_then(name_in)
            .or_else(|| shared.and_then(name_in))?;
        Language::named(name)
    }

    /// Whether `text` holds more stop words of the other language than of this one; `None` when it holds fewer
    /// than `MIN_WORDS` words of prose, too few to judge.
    pub(super) fn breached_by(self, text: &str) -> Option<bool> {
        let words = prose_words(text);
        if words.len() < MIN_WORDS {
            return None;
        }
        let count = |list: &[&str]| words.iter().filter(|w| list.contains(&w.as_str())).count();
        Some(count(self.other) > count(self.own))
    }
}

/// The `user_language` of the `[project]` table of a configuration.
fn name_in(config: &toml::Table) -> Option<&str> {
    config.get("project")?.get("user_language")?.as_str()
}

/// The lower-cased words of a text once fenced code blocks (the fence rule of QUA-12) and inline code are removed.
fn prose_words(text: &str) -> Vec<String> {
    let mut prose = String::new();
    for (line, _) in super::fenced(text.lines()).filter(|&(_, fenced)| !fenced) {
        prose.extend(line.split('`').step_by(2).flat_map(|part| [part, " "]));
        prose.push('\n');
    }
    prose
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}
