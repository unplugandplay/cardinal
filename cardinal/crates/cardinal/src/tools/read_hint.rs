//! `read-hint`, a `PreToolUse` hook on `Read`: before a whole read whose result would be over the threshold of rule
//! `oversized-result` (BRE-19), it denies the call once per file and session with a reason naming the size and the
//! range options, so the agent reads a range; the same call repeated goes through, the file being needed whole. It
//! lets go whenever it cannot judge or record, so it never blocks for its own failure. Spec: `specs/read-hint.md`.

use super::rule_breaches::OVERSIZED;
use serde_json::{json, Value};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The size above which Claude Code refuses a whole read itself (RDH-1).
const READ_CAP: u64 = 262_144;

/// The lines a whole read returns, the characters it keeps of each, and the bytes of the number it prefixes (RDH-7).
const READ_LINES: usize = 2_000;
const LINE_CHARS: usize = 2_000;
const NUMBER_BYTES: usize = 7;

/// The extensions Claude Code renders rather than reads as text (RDH-2).
const RENDERED: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "pdf", "ipynb"];

/// The bytes a whole read of `text` returns (RDH-7).
fn estimate(text: &str) -> usize {
    text.lines()
        .take(READ_LINES)
        .map(|line| {
            let kept = line
                .char_indices()
                .nth(LINE_CHARS)
                .map_or(line.len(), |(at, _)| at);
            kept + 1 + NUMBER_BYTES
        })
        .sum()
}

/// The estimate of the result of a whole read of `input`, when it is over the threshold (RDH-1, RDH-2).
fn oversized(input: &Value) -> Option<(&str, usize)> {
    if !input["offset"].is_null() || !input["limit"].is_null() {
        return None;
    }
    let path = input["file_path"].as_str()?;
    let rendered = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| RENDERED.contains(&e.to_ascii_lowercase().as_str()));
    let mut file = File::open(path).ok()?;
    let meta = file.metadata().ok().filter(fs::Metadata::is_file)?;
    if rendered || meta.len() > READ_CAP {
        return None;
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let bytes = estimate(&String::from_utf8_lossy(&bytes));
    (bytes > OVERSIZED).then_some((path, bytes))
}

/// The directory of the records: `cardinal-read-hint` of `XDG_RUNTIME_DIR`, else `cardinal-read-hint-<USER>` of the
/// temporary directory, created with mode 0700; `None` when its mode is another, such as one that lets its group or others in (RDH-5).
fn records() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|d| !d.is_empty())
        .map_or_else(
            || {
                let user = std::env::var("USER").unwrap_or_default();
                std::env::temp_dir().join(format!("cardinal-read-hint-{user}"))
            },
            |runtime| PathBuf::from(runtime).join("cardinal-read-hint"),
        );
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .ok()?;
    let meta = fs::symlink_metadata(&dir).ok()?;
    (meta.is_dir() && meta.permissions().mode() & 0o777 == 0o700).then_some(dir)
}

/// The record of the files `session` was sent back from, its name kept to the characters of a file name.
fn record_of(dir: &Path, session: &str) -> PathBuf {
    let name: String = session
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(name)
}

/// Record `path` for the session, `false` when it was already there (RDH-3) or cannot be written (RDH-4).
fn first_time(record: &Path, path: &str) -> bool {
    if fs::read_to_string(record).is_ok_and(|text| text.lines().any(|line| line == path)) {
        return false;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(record)
        .and_then(|mut file| writeln!(file, "{path}"))
        .is_ok()
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let Ok(event) = serde_json::from_str::<Value>(&input) else {
        return 0;
    };
    if event["hook_event_name"] != "PreToolUse" || event["tool_name"] != "Read" {
        return 0;
    }
    let Some(session) = event["session_id"].as_str().filter(|s| !s.is_empty()) else {
        return 0;
    };
    let Some((path, bytes)) = oversized(&event["tool_input"]) else {
        return 0;
    };
    let Some(dir) = records() else {
        return 0;
    };
    if !first_time(&record_of(&dir, session), path) {
        return 0;
    }
    let reason = format!(
        "A whole read of this file returns about {bytes} bytes: read the range you need (`offset` and `limit`) or search it with `Grep`; \
         if you need it whole, repeat the same call, which then goes through."
    );
    let output = json!({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }});
    println!("{output}");
    0
}
