//! Reading of the Claude Code transcripts, shared by the tools that read them (ADR 0006, 0010):
//! the projects directory, its projects and sessions, the sub-agent transcripts of a session, the parsed lines, the
//! filter of the messages the user typed, and what a session is resolved to (its repository, project, start and the
//! Cardinal revision then).
#![allow(dead_code)]

use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{CONFIG, LOCK};

/// Branches that name no issue.
pub const DEFAULT_BRANCHES: &[&str] = &["main", "master", "HEAD"];

/// The entries of `dir`, sorted; empty when it cannot be read.
pub fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.sort();
    paths
}

/// The projects directory: `arg` when given, else the `projects/` of the configuration directory (EVA-15), with a
/// warning on stderr when that default cannot be read (missing, not a directory, not readable).
pub fn projects_root(arg: Option<&String>) -> Result<PathBuf, String> {
    if let Some(dir) = arg {
        Ok(PathBuf::from(dir))
    } else {
        let root = crate::config_dir()
            .map(|dir| dir.join("projects"))
            .ok_or_else(|| {
                "the configuration directory cannot be located (set HOME or CLAUDE_CONFIG_DIR): pass the projects directory"
                    .to_string()
            })?;
        if let Err(e) = std::fs::read_dir(&root) {
            eprintln!(
                "warning: the projects directory {} cannot be read ({e}): no transcript read",
                root.display()
            );
        }
        Ok(root)
    }
}

/// The project directories of the projects directory.
pub fn projects(root: &Path) -> Vec<PathBuf> {
    sorted_entries(root)
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

fn jsonl_files(dir: &Path) -> Vec<PathBuf> {
    sorted_entries(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect()
}

/// The session transcripts of a project (`<session>.jsonl`), sub-agents excluded.
pub fn sessions(project: &Path) -> Vec<PathBuf> {
    jsonl_files(project)
}

/// The transcripts of the sub-agents of a session: `<session>/subagents/*.jsonl` next to `<session>.jsonl`.
pub fn subagents(session: &Path) -> Vec<PathBuf> {
    jsonl_files(&session.with_extension("").join("subagents"))
}

/// Calls `f` on each parsed line of a transcript, one at a time; returns the count of non-empty lines that are not
/// valid JSON.
pub fn for_each_line(path: &Path, mut f: impl FnMut(&Value)) -> std::io::Result<usize> {
    for_each_numbered_line(path, |_, value| f(value))
}

/// `for_each_line`, giving `f` the 1-based number of the line in the file too.
pub fn for_each_numbered_line(
    path: &Path,
    mut f: impl FnMut(usize, &Value),
) -> std::io::Result<usize> {
    use std::io::BufRead;
    let mut unreadable = 0;
    for (n, line) in std::io::BufReader::new(std::fs::File::open(path)?)
        .split(b'\n')
        .enumerate()
    {
        let line = line?;
        if line.trim_ascii().is_empty() {
            continue;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(value) => f(n + 1, &value),
            Err(_) => unreadable += 1,
        }
    }
    Ok(unreadable)
}

/// The text a user typed in a transcript line; `None` for tool results, meta and sidechain lines, injected text and other types.
pub fn typed_text(line: &Value) -> Option<String> {
    if line["type"] != "user"
        || line["isMeta"].as_bool() == Some(true)
        || line["isSidechain"].as_bool() == Some(true)
    {
        return None;
    }
    let content = &line["message"]["content"];
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|p| p["type"] == "text")
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let t = text.trim();
    // Reminders, command echoes and the compaction summary are injected as user lines; a typed `<div> broken` is not.
    let injected = (t.starts_with('<') && t.ends_with('>'))
        || t.starts_with("This session is being continued");
    (!t.is_empty() && !injected).then_some(text)
}

/// A command the user typed: a slash command or a `!` shell command, which `typed_text` reads as injected text.
pub fn user_command(line: &Value) -> bool {
    let own = line["type"] == "user"
        && line["isMeta"].as_bool() != Some(true)
        && line["isSidechain"].as_bool() != Some(true);
    own && line["message"]["content"].as_str().is_some_and(|t| {
        t.contains("<command-name>/") || t.trim_start().starts_with("<bash-input>")
    })
}

/// Whether a line opens a turn of the user: a message or a command they typed.
pub fn user_turn(line: &Value) -> bool {
    typed_text(line).is_some() || user_command(line)
}

/// The text of a tool result: its string content, or its text parts joined.
pub fn result_text(block: &Value) -> String {
    match &block["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The typed messages that only validate a stop, compared trimmed, lower-cased, trailing `.` and `!` removed.
const VALIDATIONS: &[&str] = &[
    "ok",
    "okay",
    "yes",
    "y",
    "oui",
    "go",
    "continue",
    "lgtm",
    "vas-y",
    "c'est bon",
];

/// A plain validation, or the answers to the questions of a stop (`1 a, 2 c`; one answer needs its space, so a
/// reply such as `4k` is not one).
pub fn is_validation(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    let t = t.trim_end_matches(['.', '!']).trim();
    let answer = |item: &str| {
        let item = item.trim();
        let Some(letter) = item.chars().last().filter(|c| ('a'..='h').contains(c)) else {
            return false;
        };
        let number = item[..item.len() - letter.len_utf8()].trim_end();
        !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
    };
    let items: Vec<&str> = t.split(',').collect();
    let answers = items.iter().all(|i| answer(i)) && (items.len() > 1 || t.contains(' '));
    VALIDATIONS.contains(&t) || answers
}

/// Appends `value` to `list` unless it is empty or already there.
pub fn push_distinct(list: &mut Vec<String>, value: Option<&str>) {
    if let Some(v) = value.filter(|v| !v.is_empty() && !list.iter().any(|k| k == v)) {
        list.push(v.to_string());
    }
}

/// What places a session: its first `cwd` (own transcript), the branches other than the default ones, and its
/// earliest and latest timestamps, sub-agents included.
#[derive(Default)]
pub struct Place {
    pub cwd: Option<String>,
    pub issues: Vec<String>,
    pub start: Option<(DateTime<FixedOffset>, String)>,
    pub end: Option<DateTime<FixedOffset>>,
}

impl Place {
    /// Takes in one line; returns false when it carries no valid `timestamp`.
    pub fn observe(&mut self, line: &Value, main: bool) -> bool {
        if main && self.cwd.is_none() {
            self.cwd = line["cwd"].as_str().map(str::to_string);
        }
        push_distinct(
            &mut self.issues,
            line["gitBranch"]
                .as_str()
                .filter(|b| !DEFAULT_BRANCHES.contains(b)),
        );
        let Some((at, text)) = line["timestamp"]
            .as_str()
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok().map(|d| (d, t)))
        else {
            return false;
        };
        if self.start.as_ref().is_none_or(|(s, _)| at < *s) {
            self.start = Some((at, text.to_string()));
        }
        if self.end.is_none_or(|e| at > e) {
            self.end = Some(at);
        }
        true
    }

    /// The text of the earliest timestamp, empty when none.
    pub fn start_text(&self) -> &str {
        self.start
            .as_ref()
            .map(|(_, text)| text.as_str())
            .unwrap_or_default()
    }
}

pub fn git(dir: &Path, args: &[&str]) -> Option<String> {
    git_bytes(dir, args).map(|out| String::from_utf8_lossy(&out).trim().to_string())
}

/// The standard output of a successful git command in `dir`, as it is.
pub fn git_bytes(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(out.stdout)
}

/// The main worktree of the repository holding `cwd`: a linked worktree counts with its main repository, a submodule
/// or a separate git directory is its own work tree. A removed worktree under `<repo>/.claude/worktrees/` (ADR 0008),
/// or a removed `<repo>-<x>` (ADR 0003, ADR 0023), counts with `<repo>` when that one still exists; `None` for an
/// empty `cwd`.
pub fn repository(cwd: &str, branches: &[String]) -> Option<PathBuf> {
    if cwd.is_empty() {
        return None;
    }
    let dir = Path::new(cwd);
    if !dir.exists() {
        let nested = dir
            .ancestors()
            .find(|a| a.ends_with(".claude/worktrees"))
            .and_then(|a| a.parent()?.parent());
        if let Some(found) = nested
            .filter(|r| r.exists())
            .and_then(|r| repository(r.to_str()?, &[]))
        {
            return Some(found);
        }
        return dir
            .ancestors()
            .take_while(|a| !a.exists())
            .find_map(|gone| sibling(gone, branches));
    }
    let common = PathBuf::from(git(dir, &["rev-parse", "--git-common-dir"])?);
    let common = if common.is_absolute() {
        common
    } else {
        dir.join(common)
    };
    if common.file_name().is_some_and(|n| n == ".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        git(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
    }
}

/// The repository of a removed `<repo>-<x>`: `<repo>` when it is the top level of a git work tree, `<x>` a branch of
/// the session first, else the longest such `<repo>`.
fn sibling(gone: &Path, branches: &[String]) -> Option<PathBuf> {
    let name = gone.file_name()?.to_str()?;
    let by_branch = branches
        .iter()
        .filter_map(|b| name.strip_suffix(&format!("-{b}")));
    let by_prefix = name.rmatch_indices('-').map(|(i, _)| &name[..i]);
    by_branch
        .chain(by_prefix)
        .filter(|r| !r.is_empty())
        .map(|r| gone.with_file_name(r))
        .filter(|r| {
            git(r, &["rev-parse", "--show-toplevel"])
                .is_some_and(|top| r.canonicalize().is_ok_and(|r| Path::new(&top) == r))
        })
        .find_map(|r| repository(r.to_str()?, &[]))
}

/// The project of a session: the name of its repository, else the last component of `cwd`.
pub fn project_name(cwd: &str, repo: Option<&Path>) -> String {
    repo.and_then(Path::file_name)
        .or_else(|| Path::new(cwd).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The default branch of `repo`: `origin/HEAD`, else `main`, else `master`.
pub fn default_branch(repo: &Path) -> Option<&'static str> {
    ["origin/HEAD", "main", "master"].into_iter().find(|b| {
        git(
            repo,
            &["rev-parse", "--verify", "-q", &format!("{b}^{{commit}}")],
        )
        .is_some()
    })
}

/// The configuration under its name before the format 3, which past commits hold (ADR 0015).
pub const OLD_CONFIG: &str = ".claude/harness.toml";
/// The lock under its name before the format 3.
pub const OLD_LOCK: &str = ".claude/harness.lock";
/// The configuration of a project under its two names, the new one first: a commit holds one or the other.
pub const CONFIGS: [&str; 2] = [CONFIG, OLD_CONFIG];
/// The lock under its two names, as `CONFIGS`.
pub const LOCKS: [&str; 2] = [LOCK, OLD_LOCK];

/// The last commit of the first-parent history of the default branch of `repo` touching one of `paths` at or before
/// `at`.
fn last_commit(repo: &Path, at: &str, paths: &[&str]) -> Option<String> {
    let branch = default_branch(repo)?;
    let before = format!("--before={at}");
    let mut args = vec![
        "log",
        "-1",
        "--first-parent",
        "--format=%H",
        &before,
        branch,
        "--",
    ];
    args.extend(paths);
    git(repo, &args).filter(|c| !c.is_empty())
}

/// The text of the first of `paths` that `commit` of `repo` holds.
pub fn shown(repo: &Path, commit: &str, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| git(repo, &["show", &format!("{commit}:{path}")]))
}

/// The configuration of `commit` in `repo`, the one `just check-commit` read; `None` when it has none or it is invalid.
pub fn commit_config(repo: &Path, commit: &str) -> Option<toml::Table> {
    shown(repo, commit, &CONFIGS).and_then(|t| t.parse().ok())
}

/// The text of the first of `paths` as committed on the default branch of `repo` at `at`, in the last commit touching
/// one of them (the new and the old name of a file); `None` when it cannot be read.
pub fn committed_file(repo: &Path, at: &str, paths: &[&str]) -> Option<String> {
    shown(repo, &last_commit(repo, at, paths)?, paths)
}

/// The configuration committed on the default branch of `repo` at `at`, parsed.
pub fn committed_config(repo: &Path, at: &str) -> Option<toml::Table> {
    committed_file(repo, at, &CONFIGS)?.parse().ok()
}

/// The revision a parsed configuration pins (ADR 0002): its `rev`, `unpinned` with no `rev`; `None` for the
/// Cardinal repository itself (`source = "."`, no `rev`), whose revision is a commit of its `files/`.
pub fn pinned_revision(config: &toml::Table) -> Option<String> {
    let harness = config.get("cardinal").or_else(|| config.get("harness"));
    let field = |key: &str| {
        harness
            .and_then(|h| h.get(key))
            .and_then(toml::Value::as_str)
    };
    match (field("rev"), field("source")) {
        (Some(rev), _) => Some(rev.to_string()),
        (None, Some(".")) => None,
        (None, _) => Some("unpinned".to_string()),
    }
}

/// The Cardinal revision in force in `repo` at `start`: the `pinned_revision` of the configuration committed
/// on the default branch then; `self:<commit>` for the Cardinal repository itself, the last commit touching `files/`;
/// empty when it cannot be read.
pub fn revision(repo: &Path, start: &str) -> String {
    let Some(config) = committed_config(repo, start) else {
        return String::new();
    };
    pinned_revision(&config).unwrap_or_else(|| {
        last_commit(repo, start, &["files"])
            .map(|c| format!("self:{c}"))
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(repo: &Path, args: &[&str], date: &str) {
        let out = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    }

    /// A repository whose old configuration and lock pin `old` at 10:00, moved to the new names pinning `new` at 12:00.
    fn migrated() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        sh(repo, &["init", "-q", "-b", "main"], "2026-01-01T09:00:00Z");
        std::fs::create_dir_all(repo.join(".claude")).unwrap();
        let config = |table: &str, rev: &str| {
            format!("[{table}]\nsource = \"x\"\nrev = \"{rev}\"\n[project]\ncommit_scopes = [\"{rev}\"]\n")
        };
        std::fs::write(repo.join(OLD_CONFIG), config("harness", "old")).unwrap();
        std::fs::write(repo.join(OLD_LOCK), "h  old.txt\n").unwrap();
        sh(repo, &["add", "-A"], "2026-01-01T10:00:00Z");
        sh(repo, &["commit", "-qm", "old"], "2026-01-01T10:00:00Z");
        std::fs::remove_file(repo.join(OLD_CONFIG)).unwrap();
        std::fs::remove_file(repo.join(OLD_LOCK)).unwrap();
        std::fs::write(repo.join(CONFIG), config("cardinal", "new")).unwrap();
        std::fs::write(repo.join(LOCK), "h  new.txt\n").unwrap();
        sh(repo, &["add", "-A"], "2026-01-01T12:00:00Z");
        sh(repo, &["commit", "-qm", "new"], "2026-01-01T12:00:00Z");
        dir
    }

    // every reader of the history goes through these helpers: a fixture per tool would repeat the same history
    // spec: EVA-8, BRE-7, BRE-9, BRE-10, REP-2 (internal: the readers share these helpers)
    #[test]
    fn the_history_is_read_under_the_new_names_else_the_old_ones() {
        let dir = migrated();
        let repo = dir.path();
        let rev = |at: &str| committed_config(repo, at).and_then(|c| pinned_revision(&c));
        assert_eq!(rev("2026-01-01T11:00:00Z").as_deref(), Some("old"));
        assert_eq!(rev("2026-01-01T13:00:00Z").as_deref(), Some("new"));
        let lock = |at: &str| committed_file(repo, at, &LOCKS);
        assert_eq!(lock("2026-01-01T11:00:00Z").as_deref(), Some("h  old.txt"));
        assert_eq!(lock("2026-01-01T13:00:00Z").as_deref(), Some("h  new.txt"));
        let head = git(repo, &["rev-parse", "HEAD"]).unwrap();
        let first = git(repo, &["rev-parse", "HEAD~1"]).unwrap();
        assert!(shown(repo, &head, &CONFIGS).unwrap().contains("[cardinal]"));
        assert!(shown(repo, &first, &CONFIGS).unwrap().contains("[harness]"));
    }
}
