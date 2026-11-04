//! The warnings of `check` and `sync` on the project's `.claude/settings.json` (`specs/sync.md`, SYN-24 to SYN-27):
//! what Cardinal expects of the file beyond the entries `sync` manages (`entries.rs`). Also the
//! retention of the transcripts in the user settings, which Cardinal never writes (SYN-28).

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

use crate::entries::SETTINGS;

/// The `permissions.deny` rules that keep the review waiver from the agent (REV-19).
pub const DENY_RULES: [&str; 3] = [
    "Bash(just waive-review)",
    "Bash(just waive-review *)",
    "Edit(//**/claude-review-waived)",
];
/// The settings of the project kept out of git, relative to its root.
const LOCAL_SETTINGS: &str = ".claude/settings.local.json";
/// The days Claude Code keeps the transcripts when `cleanupPeriodDays` is unset.
const DEFAULT_RETENTION: f64 = 30.0;
/// The days of transcripts the evaluation of Cardinal needs to span several revisions.
const MIN_RETENTION: f64 = 365.0;
/// Where the hooks the settings run live, relative to the root of the project.
const HOOKS: &str = ".claude/hooks/";
/// What may come before `HOOKS` in a word of a hook command, quotes removed, for the path to be the project's.
const PROJECT_PREFIXES: [&str; 4] = ["", "./", "$CLAUDE_PROJECT_DIR/", "${CLAUDE_PROJECT_DIR}/"];

/// The warnings on the project's settings, none when it has none: the deny rules they lack when `gated` (the project
/// has the review hook), the hooks they run that the project lacks (`present` says whether it has a path relative to
/// its root), and an attribution left on when `attribution` is banned (FEA-10). One warning saying why when they cannot be
/// read or parsed.
pub fn warnings(
    root: &Path,
    gated: bool,
    present: &dyn Fn(&str) -> bool,
    attribution: bool,
) -> Vec<String> {
    match fs::read_to_string(root.join(SETTINGS)) {
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => vec![format!("warning: {SETTINGS} cannot be read: {e}")],
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Err(e) => vec![format!("warning: {SETTINGS} is not valid JSON: {e}")],
            Ok(settings) => warnings_of(&settings, gated, present, attribution),
        },
    }
}

/// The warnings of `warnings` on the parsed `settings`, the settings as a sync leaves them (SYN-27).
pub fn warnings_of(
    settings: &Value,
    gated: bool,
    present: &dyn Fn(&str) -> bool,
    attribution: bool,
) -> Vec<String> {
    let mut warnings = if gated {
        missing_deny_rules(settings)
    } else {
        Vec::new()
    };
    warnings.extend(missing_hooks(settings, present));
    if attribution && !attribution_off(settings) {
        warnings.push(format!(
            "warning: {SETTINGS} does not turn off the attribution"
        ));
    }
    warnings
}

/// One warning per rule of `DENY_RULES` missing from `permissions.deny` (SYN-24).
fn missing_deny_rules(settings: &Value) -> Vec<String> {
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    DENY_RULES
        .iter()
        .filter(|rule| !deny.contains(rule))
        .map(|rule| format!("warning: {SETTINGS} lacks the deny rule {rule}"))
        .collect()
}

/// One warning per file under the project's `HOOKS` that a hook command names and the project lacks, in the order of
/// the settings (SYN-25). A path under another directory, a directory or a glob is not looked at.
fn missing_hooks(settings: &Value, present: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for word in hook_commands(settings).flat_map(str::split_whitespace) {
        let word: String = word.chars().filter(|c| !matches!(c, '"' | '\'')).collect();
        let Some(at) = word.find(HOOKS) else { continue };
        if !PROJECT_PREFIXES.contains(&&word[..at]) {
            continue;
        }
        let name: String = word[at + HOOKS.len()..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
            .collect();
        let path = format!("{HOOKS}{name}");
        if !name.is_empty() && !present(&path) && !missing.contains(&path) {
            missing.push(path);
        }
    }
    missing
        .iter()
        .map(|path| format!("warning: {SETTINGS} runs the missing hook {path}"))
        .collect()
}

/// The commands of the hooks of `settings`, in the order of the settings.
pub fn hook_commands(settings: &Value) -> impl Iterator<Item = &str> {
    settings["hooks"]
        .as_object()
        .into_iter()
        .flat_map(|events| events.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|matcher| matcher["hooks"].as_array())
        .flatten()
        .filter_map(|hook| hook["command"].as_str())
}

/// Whether `attribution` is `false`, or holds an empty `commit` and an empty `pr` (SYN-26).
fn attribution_off(settings: &Value) -> bool {
    let attribution = &settings["attribution"];
    attribution == &Value::Bool(false)
        || ["commit", "pr"]
            .iter()
            .all(|key| attribution[*key].as_str() == Some(""))
}

/// The retention warnings (SYN-28): the user settings at `user` keep the transcripts less than `MIN_RETENTION` days
/// (unset: the default), or cannot be read or located, and each settings file of the project that sets a lower number.
/// Claude Code sweeps the transcripts of every project with the value of the session that runs the sweep.
pub fn retention(root: &Path, user: Option<&Path>) -> Vec<String> {
    let mut warnings = user_retention(user);
    warnings.extend(project_retention(root));
    warnings
}

/// The retention warning of `retention` on the user settings at `user`, `None` when they cannot be located.
pub fn user_retention(user: Option<&Path>) -> Vec<String> {
    let mut warnings = Vec::new();
    match user.map(|user| (user, read_json(user))) {
        None => warnings.push(
            "warning: the user settings cannot be located (set HOME or CLAUDE_CONFIG_DIR); the retention of the transcripts is not checked"
                .to_string(),
        ),
        Some((user, Err(()))) => warnings.push(format!(
            "warning: {} cannot be read or is not valid JSON; Claude Code pauses the cleanup of the transcripts until it is fixed",
            user.display()
        )),
        Some((user, Ok(settings))) => {
            let days = settings.as_ref().and_then(retention_days).unwrap_or(DEFAULT_RETENTION);
            if days < MIN_RETENTION {
                warnings.push(format!(
                    "warning: the transcripts are deleted after {days} days (cleanupPeriodDays); set it to at least {MIN_RETENTION} in {}",
                    user.display()
                ));
            }
        }
    }
    warnings
}

/// The retention warnings of `retention` on each settings file of the project at `root`.
pub fn project_retention(root: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    for rel in [SETTINGS, LOCAL_SETTINGS] {
        let settings = read_json(&root.join(rel)).ok().flatten();
        if let Some(days) = settings.as_ref().and_then(retention_days) {
            if days < MIN_RETENTION {
                warnings.push(format!(
                    "warning: {rel} sets cleanupPeriodDays to {days} days, below {MIN_RETENTION}"
                ));
            }
        }
    }
    warnings
}

/// The file at `path` parsed as JSON: `None` when there is no such file, an error when it cannot be read or parsed.
fn read_json(path: &Path) -> Result<Option<Value>, ()> {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(_) => Err(()),
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|_| ()),
    }
}

/// `cleanupPeriodDays` when it is a number.
fn retention_days(settings: &Value) -> Option<f64> {
    settings["cleanupPeriodDays"].as_f64()
}
