//! The lock of a worktree (ADR 0022): the first Claude Code session that acts in a worktree holds it while its process
//! runs, and the others are refused their edits and commands, so two sessions never share its files, index and
//! review marker. The lock is the file `cardinal-session` of the git directory of the worktree, holding the
//! `session_id` of the hook event and the process of the session (`CLAUDE_PID`); a process that no longer runs frees
//! it, and so does a holder with no work in progress (ADR 0024): a clean worktree and no issue it claims. Every change
//! of the lock happens under a mutex directory beside it, created atomically.
//! A gate for cooperative agents (ADR 0005): a broken input or environment lets the call through.
//! Usage: hooks `cardinal run workspace-lock check` (`PreToolUse`), `start` (`SessionStart`), `release` (`SessionEnd`).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use super::issue_claim::{self, alive};

/// A mutex older than this was left by a process that died holding it.
const STALE_MUTEX: Duration = Duration::from_secs(10);

/// The holder of a lock.
#[derive(PartialEq)]
struct Holder {
    session: String,
    pid: String,
}

impl Holder {
    fn parse(text: &str) -> Option<Self> {
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key)?.strip_prefix(' '))
                .map(str::to_string)
        };
        Some(Holder {
            session: field("session")?,
            pid: field("pid")?,
        })
    }

    fn text(&self) -> String {
        format!("session {}\npid {}\n", self.session, self.pid)
    }
}

/// The path of the lock of the worktree of `cwd` (WSL-9: per worktree).
fn lock_path(cwd: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", "cardinal-session"])
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Some(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

fn read(path: &Path) -> Option<Holder> {
    Holder::parse(&std::fs::read_to_string(path).ok()?)
}

/// The mutex of `lock`, removed when dropped.
struct Mutex(PathBuf);

impl Mutex {
    /// Wait up to two seconds for the mutex of `lock`, breaking one left by a dead process.
    fn take(lock: &Path) -> Option<Self> {
        let dir = lock.with_extension("mutex");
        for _ in 0..200 {
            if std::fs::create_dir(&dir).is_ok() {
                return Some(Mutex(dir));
            }
            let old = std::fs::metadata(&dir)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age > STALE_MUTEX);
            if old {
                let _ = std::fs::remove_dir(&dir);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }
}

impl Drop for Mutex {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.0);
    }
}

/// Whether `holder` keeps the lock of the worktree of `cwd` from `me`: another session of another process that runs and
/// has work in progress (WSL-3, WSL-10).
fn keeps(holder: &Holder, me: &Holder, cwd: &Path) -> bool {
    holder.session != me.session
        && holder.pid != me.pid
        && alive(&holder.pid)
        && in_progress(holder, cwd)
}

/// Whether `holder` has work in progress (WSL-10): a change or an untracked file in the worktree of `cwd`, or an issue
/// claimed `working` by its session or process; an unreadable status counts as work in progress.
fn in_progress(holder: &Holder, cwd: &Path) -> bool {
    let status = Command::new("git")
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=normal",
        ])
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success());
    let Some(status) = status else {
        return true;
    };
    !status.stdout.is_empty()
        || issue_claim::all(cwd).iter().any(|(_, _, c)| {
            c.state == "working" && (c.session == holder.session || c.pid == holder.pid)
        })
}

/// Take the lock at `path` for `me` (WSL-1, WSL-2, WSL-4, WSL-10): `Ok` when it holds it, else the holder that keeps it.
fn take(path: &Path, me: &Holder, cwd: &Path) -> Result<(), Holder> {
    let before = read(path);
    if before.as_ref() == Some(me) {
        return Ok(());
    }
    // Read git before the mutex, so a slow status never holds it; a holder that changed meanwhile is judged under it.
    let kept = before.as_ref().is_some_and(|h| keeps(h, me, cwd));
    let Some(_mutex) = Mutex::take(path) else {
        return Ok(());
    };
    let keeper = read(path).filter(|h| {
        if before.as_ref() == Some(h) {
            kept
        } else {
            keeps(h, me, cwd)
        }
    });
    if let Some(h) = keeper {
        return Err(h);
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    if std::fs::write(&tmp, me.text()).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
    Ok(())
}

/// Delete the lock at `path` when `session` holds it (WSL-6).
fn release(path: &Path, session: &str) {
    let Some(_mutex) = Mutex::take(path) else {
        return;
    };
    if read(path).is_some_and(|h| h.session == session) {
        let _ = std::fs::remove_file(path);
    }
}

pub(super) fn main(args: &[String]) -> u8 {
    let mode = args.first().map_or("", String::as_str);
    if !["check", "start", "release"].contains(&mode) {
        eprintln!("usage: workspace-lock check | start | release");
        return 2;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let Ok(event) = serde_json::from_str::<serde_json::Value>(&input) else {
        return 0;
    };
    let (Some(session), Some(cwd)) = (event["session_id"].as_str(), event["cwd"].as_str()) else {
        return 0;
    };
    let Some(pid) = std::env::var("CLAUDE_PID").ok().filter(|p| !p.is_empty()) else {
        return 0;
    };
    let Some(path) = lock_path(Path::new(cwd)) else {
        return 0;
    };
    if mode == "release" {
        release(&path, session);
        return 0;
    }
    let me = Holder {
        session: session.to_string(),
        pid,
    };
    let Err(holder) = take(&path, &me, Path::new(cwd)) else {
        return 0;
    };
    if mode == "start" {
        println!("This worktree is busy: the Claude Code session {} works in it and still runs. Your edits and commands will be refused here: tell the user to open another worktree or to end that session.", holder.session);
        return 0;
    }
    eprintln!("Refused: another live session ({}) works in this worktree; two sessions would share its files, index and review marker. Stop and let the user open another worktree or end that session.", holder.session);
    2
}
