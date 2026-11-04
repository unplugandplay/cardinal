//! Behavior test for the hooks `cardinal tool workspace-lock check | start | release` (`specs/workspace-lock.md`):
//! hook events on stdin, the process of the session in `CLAUDE_PID`, against a throwaway repository with a linked
//! worktree.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

/// A repository `main` and its linked worktree `ws`.
struct Repo {
    _root: tempfile::TempDir,
    main: PathBuf,
    ws: PathBuf,
}

fn repo() -> Repo {
    let root = tempfile::tempdir().unwrap();
    let main = root.path().join("main");
    std::fs::create_dir_all(&main).unwrap();
    git(&main, &["init", "-q", "-b", "trunk", "."]);
    git(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
    let ws = root.path().join("ws");
    git(
        &main,
        &["worktree", "add", "-q", "-b", "ws", ws.to_str().unwrap()],
    );
    Repo {
        _root: root,
        main,
        ws,
    }
}

/// What one hook run gave: (exit code, stdout, stderr).
type Out = (Option<i32>, String, String);

fn spawn(
    mode: &str,
    cwd: &Path,
    session: &str,
    pid: &str,
    input: Option<&str>,
) -> std::process::Child {
    let event =
        serde_json::json!({"session_id": session, "cwd": cwd, "hook_event_name": mode}).to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.args(["tool", "workspace-lock", mode])
        .current_dir(cwd)
        .env_remove("CLAUDE_PID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !pid.is_empty() {
        cmd.env("CLAUDE_PID", pid);
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.unwrap_or(&event).as_bytes())
        .unwrap();
    child
}

fn finish(child: std::process::Child) -> Out {
    let out = child.wait_with_output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn hook(mode: &str, cwd: &Path, session: &str, pid: &str) -> Out {
    finish(spawn(mode, cwd, session, pid, None))
}

/// The lock of the worktree of `dir`, empty when there is none.
fn lock(dir: &Path) -> String {
    let path = PathBuf::from(git(dir, &["rev-parse", "--git-path", "cardinal-session"]));
    let path = if path.is_absolute() {
        path
    } else {
        dir.join(path)
    };
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Leave work in progress in `dir`: an untracked file.
fn dirty(dir: &Path) {
    std::fs::write(dir.join("wip"), "wip").unwrap();
}

/// Claim the issue `stem` in the state `working` for `session` of process `pid` (CLM-1).
fn claim(dir: &Path, stem: &str, session: &str, pid: &str) {
    let blob = dir.join(".git").join("claim-blob");
    std::fs::write(
        &blob,
        format!(
            "session {session}\npid {pid}\nworktree {}\nbranch trunk\nstate working\n",
            dir.display()
        ),
    )
    .unwrap();
    let oid = git(dir, &["hash-object", "-w", blob.to_str().unwrap()]);
    git(
        dir,
        &["update-ref", &format!("refs/cardinal/claims/{stem}"), &oid],
    );
}

fn live() -> String {
    std::process::id().to_string()
}

/// A second live process, killed when dropped.
struct Other(std::process::Child);

impl Other {
    fn new() -> Self {
        Other(Command::new("sleep").arg("30").spawn().unwrap())
    }
    fn pid(&self) -> String {
        self.0.id().to_string()
    }
}

impl Drop for Other {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn dead() -> String {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id().to_string();
    child.wait().unwrap();
    pid
}

// spec: WSL-1
#[test]
fn check_takes_a_free_or_stale_lock_and_lets_the_call_through() {
    let r = repo();
    let (code, _, err) = hook("check", &r.main, "a", &dead());
    assert_eq!(code, Some(0), "{err}");
    assert!(lock(&r.main).contains("session a\n"));
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(0), "a dead holder frees the lock: {err}");
    assert_eq!(lock(&r.main), format!("session b\npid {}\n", live()));
}

// spec: WSL-2
#[test]
fn check_lets_the_holder_and_its_process_through() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    let (code, _, _) = hook("check", &r.main, "a", &live());
    assert_eq!(code, Some(0), "the same session in a new process");
    assert_eq!(lock(&r.main), format!("session a\npid {}\n", live()));
    let (code, _, _) = hook("check", &r.main, "c", &live());
    assert_eq!(code, Some(0), "a cleared session of the same process");
    assert!(lock(&r.main).contains("session c\n"));
}

// spec: WSL-3
#[test]
fn check_refuses_another_session_while_the_holder_runs() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    dirty(&r.main);
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(2), "{err}");
    assert!(
        err.contains("another live session") && err.contains("(a)") && err.contains("worktree"),
        "{err}"
    );
    assert!(lock(&r.main).contains("session a\n"));
}

// spec: WSL-4
#[test]
fn two_sessions_taking_the_lock_at_once_leave_one_holder() {
    for _ in 0..10 {
        let r = repo();
        dirty(&r.main);
        let other = Other::new();
        let a = spawn("check", &r.main, "a", &live(), None);
        let b = spawn("check", &r.main, "b", &other.pid(), None);
        let (a, b) = (finish(a).0, finish(b).0);
        assert!(
            (a == Some(0)) ^ (b == Some(0)),
            "exactly one holds it: {a:?} {b:?}"
        );
        let winner = if a == Some(0) { "a" } else { "b" };
        assert!(lock(&r.main).contains(&format!("session {winner}\n")));
    }
}

// spec: WSL-5
#[test]
fn start_takes_the_lock_or_tells_the_agent_the_worktree_is_busy() {
    let r = repo();
    let other = Other::new();
    let (code, out, _) = hook("start", &r.main, "a", &other.pid());
    assert_eq!(code, Some(0));
    assert!(out.is_empty(), "{out}");
    assert!(lock(&r.main).contains("session a\n"));
    dirty(&r.main);
    let (code, out, _) = hook("start", &r.main, "b", &live());
    assert_eq!(code, Some(0), "SessionStart cannot block");
    assert!(out.contains("busy") && out.contains("refused"), "{out}");
    assert!(lock(&r.main).contains("session a\n"));
}

// spec: WSL-10
#[test]
fn a_holder_with_nothing_in_progress_hands_the_lock_over() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(0), "a clean worktree and no claim: {err}");
    assert_eq!(lock(&r.main), format!("session b\npid {}\n", live()));
    dirty(&r.main);
    let (code, _, err) = hook("check", &r.main, "a", &other.pid());
    assert_eq!(
        code,
        Some(2),
        "the old holder is refused once b has work in progress: {err}"
    );
    assert!(lock(&r.main).contains("session b\n"));
}

// spec: WSL-10
#[test]
fn start_takes_over_the_lock_of_a_holder_with_nothing_in_progress() {
    let r = repo();
    let other = Other::new();
    hook("start", &r.main, "a", &other.pid());
    let (code, out, _) = hook("start", &r.main, "b", &live());
    assert_eq!(code, Some(0));
    assert!(out.is_empty(), "no busy warning: {out}");
    assert!(lock(&r.main).contains("session b\n"));
}

// spec: WSL-10
#[test]
fn a_holder_with_a_working_claim_keeps_the_lock_in_a_clean_worktree() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    claim(&r.main, "foo", "a", &other.pid());
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(2), "claimed by its session: {err}");
    claim(&r.main, "foo", "cleared", &other.pid());
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(2), "claimed by its process: {err}");
    claim(&r.main, "foo", "z", &dead());
    let (code, _, err) = hook("check", &r.main, "b", &live());
    assert_eq!(code, Some(0), "a claim of another session: {err}");
    assert!(lock(&r.main).contains("session b\n"));
}

// spec: WSL-6
#[test]
fn release_deletes_only_its_own_lock() {
    let r = repo();
    hook("check", &r.main, "a", &live());
    let (code, _, _) = hook("release", &r.main, "b", &live());
    assert_eq!(code, Some(0));
    assert!(lock(&r.main).contains("session a\n"));
    hook("release", &r.main, "a", &live());
    assert_eq!(lock(&r.main), "");
}

// spec: WSL-7
#[test]
fn a_broken_input_or_no_repository_lets_the_call_through() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    let (code, _, _) = finish(spawn("check", &r.main, "b", &live(), Some("not json")));
    assert_eq!(code, Some(0), "unreadable input");
    let (code, _, _) = hook("check", &r.main, "b", "");
    assert_eq!(code, Some(0), "no CLAUDE_PID");
    let outside = tempfile::tempdir().unwrap();
    let (code, _, _) = hook("check", outside.path(), "b", &live());
    assert_eq!(code, Some(0), "no repository");
    assert!(lock(&r.main).contains("session a\n"));
}

// spec: WSL-9
#[test]
fn a_linked_worktree_has_its_own_lock() {
    let r = repo();
    let other = Other::new();
    hook("check", &r.main, "a", &other.pid());
    let (code, _, err) = hook("check", &r.ws, "b", &live());
    assert_eq!(code, Some(0), "{err}");
    assert!(lock(&r.ws).contains("session b\n"));
    assert!(lock(&r.main).contains("session a\n"));
}

// spec: WSL-8
#[test]
fn the_shipped_settings_entries_declare_the_three_hooks() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../entries/.claude/settings.json"),
    )
    .unwrap();
    let entries: serde_json::Value = serde_json::from_str(&text).unwrap();
    let declared = |event: &str, matcher: Option<&str>, command: &str| {
        entries["hooks"][event].as_array().is_some_and(|groups| {
            groups.iter().any(|g| {
                matcher.is_none_or(|m| g["matcher"] == m)
                    && g["hooks"]
                        .as_array()
                        .is_some_and(|h| h.iter().any(|h| h["command"] == command))
            })
        })
    };
    assert!(declared(
        "PreToolUse",
        Some("Bash|Edit|Write|NotebookEdit"),
        "cardinal run workspace-lock check"
    ));
    assert!(declared(
        "SessionStart",
        None,
        "cardinal run workspace-lock start"
    ));
    assert!(declared(
        "SessionEnd",
        None,
        "cardinal run workspace-lock release"
    ));
}
