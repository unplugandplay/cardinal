//! `lint-gate`, the Stop hook: if the code changed, the project's lint must pass before the agent can finish. The lint
//! is the `lint_command` of `.claude/cardinal.toml` over the files named like its `lint_files`, else clippy over the
//! `.rs` files. Exit 2 sends the problems back to the agent so it fixes the code: clippy's diagnostics one line each
//! (file, line, column, message), any other lint's first lines, at most `MAX_DIAGNOSTICS`, since every line is billed
//! to the agent. A passing run stamps the lint state in the git directory, and the lint does not run again until that
//! state changes. When the agent already retried once
//! (`stop_hook_active`), let go to avoid an endless loop. It checks the work tree of the event's `cwd` (an agent that
//! entered an issue worktree is checked there), at the place `CLAUDE_PROJECT_DIR` has in its own. Spec:
//! `specs/lint-gate.md`.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
use std::process::Command;

use super::Declared;

const MAX_DIAGNOSTICS: usize = 40;
/// The first word of the lines cargo prints to show its progress.
const PROGRESS: [&str; 10] = [
    "Compiling",
    "Checking",
    "Updating",
    "Downloading",
    "Downloaded",
    "Locking",
    "Adding",
    "Blocking",
    "Fresh",
    "Finished",
];

fn git(dir: &str, args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// The directory to check: where `CLAUDE_PROJECT_DIR` sits in its work tree, taken in the work tree of the event's
/// `cwd` (an issue worktree the agent entered); `CLAUDE_PROJECT_DIR` itself when either is outside git.
fn work_tree(event: &serde_json::Value) -> String {
    let project = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());
    let Some(cwd) = event["cwd"].as_str() else {
        return project;
    };
    let root = git(cwd, &["rev-parse", "--show-toplevel"]);
    let prefix = git(&project, &["rev-parse", "--show-prefix"]);
    if root.trim().is_empty()
        || git(&project, &["rev-parse", "--show-toplevel"])
            .trim()
            .is_empty()
    {
        return project;
    }
    Path::new(root.trim())
        .join(prefix.trim())
        .to_string_lossy()
        .into_owned()
}

/// The changed and untracked files the lint checks, outside the `.claude` directories whose scripts are not project
/// code.
fn changed_files(dir: &str, lint: &Declared) -> Vec<String> {
    let status = git(
        dir,
        &["status", "--porcelain", "-z", "--untracked-files=all"],
    );
    let mut entries = status.split('\0').filter(|e| !e.is_empty());
    let mut files = Vec::new();
    while let Some(entry) = entries.next() {
        let (code, path) = entry.split_at(entry.len().min(3));
        let mut paths = vec![path];
        if code.starts_with('R') || code.starts_with('C') {
            // a rename or copy is followed by its source path
            paths.extend(entries.next());
        }
        files.extend(
            paths
                .into_iter()
                .filter(|p| lint.acts_on(p))
                .map(String::from),
        );
    }
    files
}

/// A hash of what the lint sees: its command, the commit, and the path and content of each changed file it checks
/// (the paths of `git status`, relative to the root of the work tree).
fn lint_state(dir: &str, lint: &Declared, files: &[String]) -> String {
    let mut hasher = DefaultHasher::new();
    lint.command.hash(&mut hasher);
    git(dir, &["rev-parse", "HEAD"]).hash(&mut hasher);
    for file in files {
        file.hash(&mut hasher);
        std::fs::read(Path::new(&lint.root).join(file))
            .unwrap_or_default()
            .hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// A diagnostic line of `--message-format=short`: `path:line:column: level: message`.
fn is_diagnostic(line: &str) -> bool {
    let mut parts = line.splitn(4, ':');
    let (path, line_no, column) = (parts.next(), parts.next(), parts.next());
    let number =
        |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    path.is_some_and(|p| !p.trim().is_empty())
        && number(line_no)
        && number(column)
        && parts.next().is_some()
}

fn is_progress(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or("");
    PROGRESS.contains(&first) || line.contains("could not compile")
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    if event["stop_hook_active"].as_bool().unwrap_or(false) {
        return 0;
    }
    let dir = work_tree(&event);
    let lint = Declared::of(Path::new(&dir), "lint_command", "lint_files", &["*.rs"]);
    match lint.command.as_deref() {
        Some("") => return 0,
        None if !Path::new(&dir).join("Cargo.toml").exists() => return 0,
        _ => {}
    }
    let files = changed_files(&dir, &lint);
    if files.is_empty() {
        return 0;
    }
    let stamp = git(
        &dir,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "claude-lint-pass",
        ],
    );
    let stamp = stamp.trim();
    let state = lint_state(&dir, &lint, &files);
    if !stamp.is_empty() && std::fs::read_to_string(stamp).is_ok_and(|s| s == state) {
        return 0;
    }
    let passed = match &lint.command {
        Some(command) => run_declared(&dir, command),
        None => run_clippy(&dir),
    };
    if !passed {
        return 2;
    }
    if !stamp.is_empty() {
        // the state as the lint left it: a lint that fixes files must not run again for its own fix
        let state = lint_state(&dir, &lint, &changed_files(&dir, &lint));
        let _ = std::fs::write(stamp, state);
    }
    0
}

/// Run the declared lint `command`; when it fails, report its first non-blank lines, standard output first (CLG-8).
fn run_declared(dir: &str, command: &str) -> bool {
    let Ok(out) = Command::new("sh")
        .args(["-c", command])
        .current_dir(dir)
        .output()
    else {
        eprintln!("cannot run `sh -c {command}`");
        return false;
    };
    if out.status.success() {
        return true;
    }
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let lines: Vec<&str> = stdout
        .lines()
        .chain(stderr.lines())
        .filter(|l| !l.trim().is_empty())
        .collect();
    eprintln!("`{command}` reports problems; fix them (the project must stay warning-free):");
    for line in lines.iter().take(MAX_DIAGNOSTICS) {
        eprintln!("{line}");
    }
    if lines.len() > MAX_DIAGNOSTICS {
        eprintln!(
            "... {} more lines (run `{command}` for the rest)",
            lines.len() - MAX_DIAGNOSTICS
        );
    }
    false
}

/// Run clippy; when it fails, report its diagnostics one line each (CLG-2, CLG-3).
fn run_clippy(dir: &str) -> bool {
    let clippy = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--message-format=short",
            "--",
            "-D",
            "warnings",
        ])
        .current_dir(dir)
        .output()
        .expect("cargo must be installed");
    if clippy.status.success() {
        return true;
    }

    let stderr = String::from_utf8_lossy(&clippy.stderr);
    // the located diagnostics and the errors with no location (a build script, a missing crate); the same
    // diagnostic comes once per target (lib, lib test): keep the first
    let mut seen = HashSet::new();
    let diagnostics: Vec<&str> = stderr
        .lines()
        .filter(|l| is_diagnostic(l) || (l.starts_with("error") && !is_progress(l)))
        .filter(|l| seen.insert(*l))
        .collect();
    eprintln!("cargo clippy reports problems; fix them (the project must stay warning-free):");
    if diagnostics.is_empty() {
        // nothing to point at: show the end of the output, without the build progress
        let lines: Vec<&str> = stderr.lines().filter(|l| !is_progress(l)).collect();
        eprintln!(
            "{}",
            lines[lines.len().saturating_sub(MAX_DIAGNOSTICS)..].join("\n")
        );
    } else {
        eprintln!(
            "{}",
            diagnostics
                .iter()
                .take(MAX_DIAGNOSTICS)
                .copied()
                .collect::<Vec<_>>()
                .join("\n")
        );
        if diagnostics.len() > MAX_DIAGNOSTICS {
            eprintln!(
                "... {} more (run `cargo clippy` for the rest)",
                diagnostics.len() - MAX_DIAGNOSTICS
            );
        }
    }
    false
}
