//! The hook commands the installed launcher must accept (spec `specs/tool-binaries.md`, BIN-15, BIN-16), and the
//! install of a newer one (BIN-20).
//!
//! The installed `cardinal` is the launcher every hook goes through: a hook
//! command `cardinal <subcommand> ...` it does not know does not run (it exits 3, BIN-17; a launcher older than BIN-17
//! exits 2, which Claude Code reads as a refusal, so the agent is locked out), and one whose launcher is not installed
//! runs nothing. `check` asks the launcher on the path, `<launcher> help <subcommand>`, before such settings take
//! effect, and `update` refuses a revision whose hooks it refuses (BIN-16).

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::run::cargo_url;
use crate::settings::hook_commands;
use crate::{is_full_hash, is_git_url};

/// The launcher.
const LAUNCHER: &str = "cardinal";

/// Whether `launcher` accepts `sub` (`<launcher> help <sub>` succeeds); `None` when it cannot be run (not on the path).
fn accepts(launcher: &str, sub: &str) -> Option<bool> {
    Command::new(launcher)
        .args(["help", sub])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map(|status| status.success())
}

/// The hook commands of `settings` whose first word is the launcher and whose subcommand the `cardinal` on the path
/// refuses, in the order of the settings; a launcher that cannot be run refuses nothing.
pub fn refused(settings: &Value) -> Vec<String> {
    let unquoted = |word: &str| word.trim_matches(|c| matches!(c, '"' | '\'')).to_string();
    let mut known: BTreeMap<String, Option<bool>> = BTreeMap::new();
    let mut refused = Vec::new();
    for command in hook_commands(settings) {
        let mut words = command.split_whitespace().map(unquoted);
        let (Some(first), Some(sub)) = (words.next(), words.next()) else {
            continue;
        };
        if first != LAUNCHER || sub.starts_with('-') {
            continue;
        }
        let accepted = *known
            .entry(sub.clone())
            .or_insert_with(|| accepts(LAUNCHER, &sub));
        if accepted == Some(false) {
            refused.push(command.to_string());
        }
    }
    refused
}

/// The command that installs the `cardinal` of `rev` of `source`, run from the root of the project (BIN-20): `cardinal
/// upgrade` at a full commit, which needs no Rust when the commit has a release; else `cargo install`, whose `--force`
/// reinstalls the same version.
pub fn install(source: &str, rev: Option<&str>) -> String {
    if !is_git_url(source) {
        return format!("cargo install --force --path {source}/crates/cardinal --locked");
    }
    if let Some(commit) = rev.filter(|rev| is_full_hash(rev)) {
        return format!("{LAUNCHER} upgrade {source} {commit}");
    }
    let rev = rev.map(|rev| format!(" --rev {rev}")).unwrap_or_default();
    format!(
        "cargo install --force --git {}{rev} --locked",
        cargo_url(source)
    )
}
