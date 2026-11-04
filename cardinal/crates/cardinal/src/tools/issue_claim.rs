//! Claim an issue for the Claude Code session that works it (ADR 0022): the ref `refs/cardinal/claims/<stem>`,
//! created by compare-and-swap, points at a blob naming the session (`CLAUDE_CODE_SESSION_ID`), its process
//! (`CLAUDE_PID`), the worktree, the branch and the state of the claim. A ref is seen from every worktree of the clone
//! and never pushed by the default refspecs. `next-issue` reads the claims, `close-issue` frees them.
//! Usage: `cardinal run issue-claim issues/<stem>.md` (from a Claude Code session).

use std::path::Path;
use std::process::{Command, Stdio};

/// The namespace of the claims, under no ref the default refspecs push (CLM-8).
const PREFIX: &str = "refs/cardinal/claims/";

/// What a claim records.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Claim {
    pub session: String,
    pub pid: String,
    pub worktree: String,
    pub branch: String,
    pub state: String,
}

impl Claim {
    fn parse(text: &str) -> Self {
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key)?.strip_prefix(' '))
                .unwrap_or_default()
                .to_string()
        };
        Claim {
            session: field("session"),
            pid: field("pid"),
            worktree: field("worktree"),
            branch: field("branch"),
            state: field("state"),
        }
    }

    fn text(&self) -> String {
        format!(
            "session {}\npid {}\nworktree {}\nbranch {}\nstate {}\n",
            self.session, self.pid, self.worktree, self.branch, self.state
        )
    }

    /// Whether the process of the claim still runs (`kill -0`).
    pub fn alive(&self) -> bool {
        alive(&self.pid)
    }
}

/// Whether the process `pid` runs; false for an empty or invalid id.
pub(super) fn alive(pid: &str) -> bool {
    pid.parse::<u32>().is_ok_and(|p| p > 0)
        && Command::new("kill")
            .args(["-0", pid])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
}

fn git(dir: &Path, args: &[&str], input: Option<&str>) -> Option<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(text) = input {
        use std::io::Write;
        child.stdin.take()?.write_all(text.as_bytes()).ok()?;
    }
    let out = child.wait_with_output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// The claim of `stem` and the object its ref points at, if any.
pub(super) fn read(dir: &Path, stem: &str) -> Option<(String, Claim)> {
    let oid = git(
        dir,
        &["rev-parse", "--verify", "-q", &format!("{PREFIX}{stem}")],
        None,
    )?;
    let text = git(dir, &["cat-file", "blob", &oid], None)?;
    Some((oid, Claim::parse(&text)))
}

/// Every claim of the repository of `dir`: `(stem, claim)`, by stem.
pub(super) fn all(dir: &Path) -> Vec<(String, String, Claim)> {
    let refs = git(
        dir,
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=3) %(objectname)",
            PREFIX,
        ],
        None,
    )
    .unwrap_or_default();
    refs.lines()
        .filter_map(|l| {
            let (stem, oid) = l.split_once(' ')?;
            let text = git(dir, &["cat-file", "blob", oid], None)?;
            Some((stem.to_string(), oid.to_string(), Claim::parse(&text)))
        })
        .collect()
}

/// Point the claim of `stem` at `claim`, only if its ref still points at `old` (`None`: only if it does not exist).
pub(super) fn write(dir: &Path, stem: &str, claim: &Claim, old: Option<&str>) -> bool {
    let Some(new) = git(dir, &["hash-object", "-w", "--stdin"], Some(&claim.text())) else {
        return false;
    };
    git(
        dir,
        &[
            "update-ref",
            &format!("{PREFIX}{stem}"),
            &new,
            old.unwrap_or(""),
        ],
        None,
    )
    .is_some()
}

/// Delete the claim of `stem`, only if its ref still points at `old`.
pub(super) fn delete(dir: &Path, stem: &str, old: &str) -> bool {
    git(
        dir,
        &["update-ref", "-d", &format!("{PREFIX}{stem}"), old],
        None,
    )
    .is_some()
}

/// The branch checked out in the worktree of `dir`, `HEAD` when detached.
pub(super) fn current_branch(dir: &Path) -> String {
    git(dir, &["symbolic-ref", "--short", "-q", "HEAD"], None).unwrap_or_else(|| "HEAD".into())
}

/// The trunk: the branch checked out by the main worktree, if any.
pub(super) fn trunk(dir: &Path) -> Option<String> {
    let list = git(dir, &["worktree", "list", "--porcelain"], None)?;
    list.split("\n\n")
        .next()?
        .lines()
        .find_map(|l| l.strip_prefix("branch refs/heads/"))
        .map(str::to_string)
}

/// Claim the issue file `issue` for the session of the environment (CLM-1 to CLM-4, CLM-9).
fn claim(issue: &str) -> Result<String, String> {
    let path = Path::new(issue);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|_| path.extension().is_some_and(|x| x == "md"))
        .ok_or_else(|| format!("{issue} is not an issue file (issues/<stem>.md)"))?;
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let named_issues =
        std::fs::canonicalize(dir).is_ok_and(|d| d.file_name().is_some_and(|n| n == "issues"));
    if !path.is_file() || !named_issues {
        return Err(format!("{issue} is not an open issue file"));
    }
    let env = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                format!("{key} is unset: an issue is claimed from a Claude Code session")
            })
    };
    let (session, pid) = (env("CLAUDE_CODE_SESSION_ID")?, env("CLAUDE_PID")?);
    let here = Path::new(".");
    let top = git(here, &["rev-parse", "--show-toplevel"], None)
        .ok_or("not in a git repository: there is nothing to claim the issue in")?;
    let inside = std::fs::canonicalize(dir)
        .ok()
        .zip(std::fs::canonicalize(&top).ok())
        .is_some_and(|(d, t)| d.starts_with(t));
    if !inside {
        return Err(format!("{issue} is not an issue of the worktree {top}"));
    }
    let mine = Claim {
        session,
        pid,
        worktree: top.clone(),
        branch: current_branch(here),
        state: "working".into(),
    };
    let done = |c: &Claim| {
        format!(
            "claimed {stem} for session {} in {} (branch {})",
            c.session, c.worktree, c.branch
        )
    };
    match read(here, &stem) {
        None => {
            let exclude = format!(":(exclude){}", dir.display());
            let dirty = git(
                here,
                &["status", "--porcelain", "-uno", "--", ":/", &exclude],
                None,
            )
            .ok_or("git status failed")?;
            if !dirty.is_empty() {
                return Err(format!("the worktree {top} has uncommitted changes outside {}: commit or stash them before claiming {stem}\n{dirty}", dir.display()));
            }
            if write(here, &stem, &mine, None) {
                Ok(done(&mine))
            } else {
                Err(refused(&stem, read(here, &stem).map(|(_, c)| c)))
            }
        }
        Some((old, theirs)) => {
            let ours = theirs.session == mine.session
                || (theirs.worktree == mine.worktree
                    && (theirs.pid == mine.pid || !theirs.alive()));
            if !ours {
                return Err(refused(&stem, Some(theirs)));
            }
            if write(here, &stem, &mine, Some(&old)) {
                Ok(done(&mine))
            } else {
                Err(refused(&stem, read(here, &stem).map(|(_, c)| c)))
            }
        }
    }
}

/// The refusal of a claim held by another session (CLM-2).
fn refused(stem: &str, theirs: Option<Claim>) -> String {
    match theirs {
        None => format!("git could not write {PREFIX}{stem} (a stem git refuses as a ref name, or a claim deleted meanwhile)"),
        Some(c) => format!(
            "{stem} is claimed by another session: worktree {}, branch {}, state {}, session {}; stop and ask the user",
            c.worktree,
            c.branch,
            c.state,
            if c.alive() { "still running" } else { "ended" }
        ),
    }
}

pub(super) fn main(args: &[String]) -> u8 {
    let [issue] = args else {
        eprintln!("usage: issue-claim issues/<stem>.md");
        return 2;
    };
    match claim(issue) {
        Ok(line) => {
            println!("{line}");
            0
        }
        Err(e) => {
            eprintln!("issue-claim: {e}");
            1
        }
    }
}
