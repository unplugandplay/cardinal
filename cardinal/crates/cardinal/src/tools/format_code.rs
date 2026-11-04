//! `format-code`, the `PostToolUse` hook (Edit|Write): format the edited file with the `format_command` of
//! `.claude/cardinal.toml` when its name matches the `format_files`, run from the root of the work tree holding it with
//! the file as last argument; else run `cargo fmt --all` after a Rust file was edited, from the directory of that file,
//! so cargo formats the project holding it (an issue worktree, a crate in a subdirectory), not the session's. Spec:
//! `specs/format-hook.md`.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use super::Declared;

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let Some(file) = event["tool_input"]["file_path"].as_str() else {
        return 0;
    };
    let dir = Path::new(file)
        .parent()
        .filter(|d| d.is_dir())
        .unwrap_or(Path::new("."));
    let format = Declared::of(dir, "format_command", "format_files", &["*.rs"]);
    // judged relative to the work tree, which may itself sit under a `.claude/worktrees` directory
    let name = Path::new(file).file_name().unwrap_or_default();
    let relative = dir
        .canonicalize()
        .ok()
        .and_then(|d| {
            d.join(name)
                .strip_prefix(&format.root)
                .ok()
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| name.into());
    if !format.acts_on(&relative.to_string_lossy()) {
        return 0;
    }
    let mut run = match &format.command {
        Some(command) if command.is_empty() => return 0,
        // a declared command implies a work tree: its configuration is read there
        Some(command) => {
            let mut sh = Command::new("sh");
            sh.args(["-c", &format!("{command} \"$@\""), "sh", file])
                .current_dir(&format.root);
            sh
        }
        None => {
            let mut cargo = Command::new("cargo");
            cargo.args(["fmt", "--all"]).current_dir(dir);
            cargo
        }
    };
    // Formatting is best-effort and silent: a syntax error mid-edit must not block the agent, nor its output be billed.
    let _ = run.stdout(Stdio::null()).stderr(Stdio::null()).status();
    0
}
