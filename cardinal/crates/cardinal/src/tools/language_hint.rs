//! `language-hint`, a Stop hook: when the last reply of the agent is not in the `user_language` of
//! `.claude/cardinal.local.toml`, else of `.claude/cardinal.toml`, as the test of rule `user-language` judges it
//! (`language.rs`), it exits 2 with a one-line hint so the agent gives it again, letting go once the agent retried
//! (`stop_hook_active`). It lets go whenever the text cannot be judged, so it never blocks for its own failure. Spec: `specs/language-hint.md`.

use super::language::{Language, HINT_MARK};
use super::transcripts::{git, typed_text};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The root of the work tree holding `dir`, when it has a configuration (`crate::config_of`).
fn root_at(dir: &Path) -> Option<PathBuf> {
    let root =
        PathBuf::from(git(dir, &["rev-parse", "--show-toplevel"]).filter(|r| !r.is_empty())?);
    Some(root).filter(|r| crate::config_of(r).exists())
}

/// The language of the configuration of the work tree of the event's `cwd`, else of `CLAUDE_PROJECT_DIR` (LNG-3),
/// its personal value first (LNG-12).
fn user_language(event: &Value) -> Option<Language> {
    let project = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());
    let root = event["cwd"]
        .as_str()
        .and_then(|cwd| root_at(Path::new(cwd)))
        .or_else(|| root_at(Path::new(&project)))?;
    let shared = std::fs::read_to_string(crate::config_of(&root))
        .ok()
        .and_then(|text| text.parse().ok());
    Language::of(&root, shared.as_ref())
}

/// The text blocks of the last assistant line holding text in the transcript, joined by a blank line; `None` when a
/// message the user typed comes after it (LNG-2).
fn last_text(transcript: &str) -> Option<String> {
    for line in transcript.lines().rev() {
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if typed_text(&line).is_some() {
            return None;
        }
        if line["type"] != "assistant" {
            continue;
        }
        let texts: Vec<&str> = line["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        if !texts.is_empty() {
            return Some(texts.join("\n\n"));
        }
    }
    None
}

/// The reply to judge: the event's `last_assistant_message`, else the last text of its transcript.
fn reply(event: &Value) -> Option<String> {
    if let Some(message) = event["last_assistant_message"].as_str() {
        return Some(message.to_string());
    }
    last_text(&std::fs::read_to_string(event["transcript_path"].as_str()?).ok()?)
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let Ok(event) = serde_json::from_str::<Value>(&input) else {
        return 0;
    };
    // any other event, such as the `PostToolUse` declaration an older Cardinal left in the settings (LNG-11)
    if event["hook_event_name"] != "Stop" {
        return 0;
    }
    if event["stop_hook_active"].as_bool().unwrap_or(false) {
        return 0;
    }
    let Some(language) = user_language(&event) else {
        return 0;
    };
    if reply(&event).and_then(|text| language.breached_by(&text)) != Some(true) {
        return 0;
    }
    let name = language.name;
    eprintln!(
        "Your last reply is not in {name}, {HINT_MARK} (or of {}, which overrides it): give it again in {name}.",
        crate::PERSONAL_CONFIG
    );
    2
}
