//! Behavior test for `cardinal tool issue-claim` and the claims `cardinal tool next-issue` reads, against throwaway git
//! repositories with a linked worktree.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// A repository `repo` on `trunk` with the issues `a` (high) and `b` (low), and a linked worktree `ws` on branch `ws`.
struct Repo {
    _root: tempfile::TempDir,
    main: PathBuf,
    ws: PathBuf,
}

fn repo() -> Repo {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("issues/done")).unwrap();
    for (s, p) in [("a", "high"), ("b", "low")] {
        std::fs::write(
            repo.join(format!("issues/{s}.md")),
            format!("---\npriority: {p}\n---\n# Title {s}\n"),
        )
        .unwrap();
    }
    std::fs::write(repo.join("code.txt"), "code\n").unwrap();
    git(&repo, &["init", "-q", "-b", "trunk", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "init", "--no-verify"]);
    let ws = root.path().join("ws");
    git(
        &repo,
        &["worktree", "add", "-q", "-b", "ws", ws.to_str().unwrap()],
    );
    Repo {
        _root: root,
        main: repo,
        ws,
    }
}

/// `cardinal tool <args>` in `cwd` as the session `session` of the process `pid`: (exit code, stdout + stderr).
fn tool(cwd: &Path, session: &str, pid: &str, args: &[&str]) -> (Option<i32>, String) {
    let out = tool_cmd(cwd, session, pid, args).output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

fn tool_cmd(cwd: &Path, session: &str, pid: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.arg("tool")
        .args(args)
        .current_dir(cwd)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_PID");
    if !session.is_empty() {
        cmd.env("CLAUDE_CODE_SESSION_ID", session);
    }
    if !pid.is_empty() {
        cmd.env("CLAUDE_PID", pid);
    }
    cmd
}

fn claim(cwd: &Path, session: &str, pid: &str, stem: &str) -> (Option<i32>, String) {
    tool(
        cwd,
        session,
        pid,
        &["issue-claim", &format!("issues/{stem}.md")],
    )
}

/// The text of the claim of `stem`, empty when there is none.
fn claim_text(repo: &Path, stem: &str) -> String {
    Command::new("git")
        .args(["cat-file", "blob", &format!("refs/cardinal/claims/{stem}")])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// A process id that runs: this test's own.
fn live() -> String {
    std::process::id().to_string()
}

/// A process id that no longer runs: a child that exited.
fn dead() -> String {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id().to_string();
    child.wait().unwrap();
    pid
}

fn next(cwd: &Path) -> String {
    tool(cwd, "", "", &["next-issue", "issues"]).1
}

fn top(dir: &Path) -> String {
    git(dir, &["rev-parse", "--show-toplevel"])
}

// spec: CLM-1
#[test]
fn a_claim_names_the_session_its_process_worktree_branch_and_state() {
    let r = repo();
    let (code, out) = claim(&r.ws, "s1", &live(), "a");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("claimed a"), "{out}");
    assert_eq!(
        claim_text(&r.main, "a"),
        format!(
            "session s1\npid {}\nworktree {}\nbranch ws\nstate working\n",
            live(),
            top(&r.ws)
        )
    );
    git(&r.main, &["checkout", "-q", "--detach"]);
    let (code, out) = claim(&r.main, "s2", &live(), "b");
    assert_eq!(code, Some(0), "{out}");
    assert!(claim_text(&r.main, "b").contains("branch HEAD\n"));
}

// spec: CLM-2
#[test]
fn a_claim_is_taken_back_only_by_its_session_or_its_worktree_once_free() {
    let r = repo();
    let (pid, gone) = (live(), dead());
    claim(&r.ws, "s1", &pid, "a");
    let (code, out) = claim(&r.main, "s2", &live(), "a");
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("worktree ") && out.contains("branch ws") && out.contains("still running"),
        "{out}"
    );
    assert!(claim_text(&r.main, "a").contains("session s1\n"));
    let (code, _) = claim(&r.ws, "s1", "4242", "a");
    assert_eq!(code, Some(0), "the same session takes it back");
    assert!(claim_text(&r.main, "a").contains("pid 4242\n"));
    claim(&r.ws, "s1", &pid, "a");
    let (code, _) = claim(&r.ws, "s3", &pid, "a");
    assert_eq!(code, Some(0), "a cleared session of the same process");
    assert!(claim_text(&r.main, "a").contains("session s3\n"));

    claim(&r.ws, "s4", &gone, "b");
    let (code, out) = claim(&r.main, "s5", &live(), "b");
    assert_eq!(code, Some(1), "another worktree never takes it: {out}");
    assert!(out.contains("ended"), "{out}");
    let (code, out) = claim(&r.ws, "s5", &live(), "b");
    assert_eq!(code, Some(0), "the same worktree takes a dead claim: {out}");
    assert!(claim_text(&r.main, "b").contains("session s5\n"));
}

// spec: CLM-3
#[test]
fn a_claim_needs_an_open_issue_a_repository_and_a_session() {
    let r = repo();
    for (session, pid) in [("", "1"), ("s", "")] {
        let (code, out) = claim(&r.ws, session, pid, "a");
        assert_eq!(code, Some(1), "{out}");
        assert!(out.contains("is unset"), "{out}");
    }
    let (code, _) = claim(&r.ws, "s", &live(), "gone");
    assert_eq!(code, Some(1));
    std::fs::create_dir_all(r.ws.join("issues/done")).unwrap();
    std::fs::write(r.ws.join("issues/done/c.md"), "# C\n").unwrap();
    let (code, _) = tool(&r.ws, "s", &live(), &["issue-claim", "issues/done/c.md"]);
    assert_eq!(code, Some(1), "a closed issue");
    let (code, _) = tool(
        &r.ws,
        "s",
        &live(),
        &["issue-claim", r.main.join("issues/a.md").to_str().unwrap()],
    );
    assert_eq!(code, Some(1), "an issue of another worktree");
    std::fs::write(r.ws.join("NOTES.md"), "# Notes\n").unwrap();
    std::fs::create_dir_all(r.ws.join("issues/sub")).unwrap();
    std::fs::write(r.ws.join("issues/sub/x.md"), "# X\n").unwrap();
    for file in ["NOTES.md", "issues/sub/x.md"] {
        let (code, _) = tool(&r.ws, "s", &live(), &["issue-claim", file]);
        assert_eq!(code, Some(1), "{file} is no issue file");
    }
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(outside.path().join("issues")).unwrap();
    std::fs::write(outside.path().join("issues/a.md"), "# A\n").unwrap();
    let (code, out) = claim(outside.path(), "s", &live(), "a");
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("not in a git repository"), "{out}");
    assert_eq!(claim_text(&r.main, "a"), "");
}

// spec: CLM-4
#[test]
fn two_sessions_claiming_at_once_leave_one_holder() {
    for _ in 0..5 {
        let r = repo();
        let pid = live();
        let mut a = tool_cmd(&r.ws, "s1", &pid, &["issue-claim", "issues/a.md"])
            .spawn()
            .unwrap();
        let mut b = tool_cmd(&r.main, "s2", &pid, &["issue-claim", "issues/a.md"])
            .spawn()
            .unwrap();
        let (a, b) = (a.wait().unwrap().success(), b.wait().unwrap().success());
        assert!(a ^ b, "exactly one succeeds: {a} {b}");
        let winner = if a { "s1" } else { "s2" };
        assert!(claim_text(&r.main, "a").contains(&format!("session {winner}\n")));
    }
}

// spec: CLM-5
#[test]
fn next_issue_skips_a_working_claim_and_says_when_its_session_ended() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    let out = next(&r.main);
    assert!(out.contains("Next: issues/b.md"), "{out}");
    let ws = top(&r.ws);
    assert!(
        out.contains(&format!("Taken: a (claimed in {ws})\n")),
        "{out}"
    );
    claim(&r.ws, "s1", &dead(), "a");
    let out = next(&r.main);
    assert!(
        out.contains(&format!("Taken: a (claimed in {ws}, session ended)\n")),
        "{out}"
    );
    claim(&r.main, "s2", &live(), "b");
    let out = next(&r.ws);
    assert!(out.contains("Every open issue is taken."), "{out}");
}

// spec: CLM-6
#[test]
fn a_merge_pending_claim_is_listed_until_the_trunk_holds_the_closed_issue() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    let text = claim_text(&r.main, "a").replace("state working", "state merge pending");
    let blob = git_stdin(&r.main, &["hash-object", "-w", "--stdin"], &text);
    git(&r.main, &["update-ref", "refs/cardinal/claims/a", &blob]);
    let out = next(&r.main);
    assert!(
        out.contains("Next: issues/b.md") && out.contains("Merge pending: a (ws)\n"),
        "{out}"
    );
    assert!(
        !claim_text(&r.main, "a").is_empty(),
        "kept before the merge"
    );
    git(&r.main, &["mv", "issues/a.md", "issues/done/a.md"]);
    git(&r.main, &["commit", "-qm", "close a", "--no-verify"]);
    let out = next(&r.ws);
    assert!(!out.contains("Merge pending"), "{out}");
    assert!(claim_text(&r.main, "a").is_empty(), "dropped once merged");
}

// spec: CLM-7
#[test]
fn close_issue_frees_the_claim_on_the_trunk_and_marks_it_merge_pending_elsewhere() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    let (code, out) = tool(&r.ws, "", "", &["close-issue", "issues/a.md"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(r.ws.join("issues/done/a.md").is_file());
    assert_eq!(
        claim_text(&r.main, "a"),
        format!(
            "session s1\npid {}\nworktree {}\nbranch ws\nstate merge pending\n",
            live(),
            top(&r.ws)
        )
    );
    assert!(out.contains("merge pending"), "{out}");
    claim(&r.main, "s2", &live(), "b");
    let (code, out) = tool(&r.main, "", "", &["close-issue", "issues/b.md"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(claim_text(&r.main, "b"), "", "deleted on the trunk");
    std::fs::write(r.main.join("issues/c.md"), "---\npriority: low\n---\n# C\n").unwrap();
    let (code, out) = tool(&r.main, "", "", &["close-issue", "issues/c.md"]);
    assert_eq!(code, Some(0), "no claim still closes: {out}");
}

// spec: CLM-6
#[test]
fn a_claim_closed_off_the_trunk_is_dropped_once_merged() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    tool(&r.ws, "", "", &["close-issue", "issues/a.md"]);
    git(&r.ws, &["commit", "-qam", "close a", "--no-verify"]);
    git(&r.main, &["merge", "-q", "--ff-only", "ws"]);
    let out = next(&r.main);
    assert!(!out.contains("Merge pending"), "{out}");
    assert_eq!(
        claim_text(&r.main, "a"),
        "",
        "dropped although `a` is no longer open"
    );
}

// spec: CLM-7
#[test]
fn close_issue_leaves_the_claim_of_another_worktree() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    let (code, out) = tool(&r.main, "", "", &["close-issue", "issues/a.md"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        claim_text(&r.main, "a").contains("state working\n"),
        "{out}"
    );
    assert!(out.contains("another worktree"), "{out}");
    std::fs::write(r.main.join("issues/c.md"), "---\npriority: low\n---\n# C\n").unwrap();
    let (code, out) = tool(
        &r.main.join("issues"),
        "s2",
        &live(),
        &["issue-claim", "c.md"],
    );
    assert_eq!(code, Some(0), "a bare file name from issues/: {out}");
    claim(&r.ws, "s1", &live(), "b");
    git(
        &r.main,
        &["worktree", "remove", "--force", r.ws.to_str().unwrap()],
    );
    let (code, out) = tool(&r.main, "", "", &["close-issue", "issues/b.md"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        claim_text(&r.main, "b"),
        "",
        "the claim of a removed worktree is freed"
    );
}

// spec: CLM-8
#[test]
fn a_claim_is_no_branch_nor_tag() {
    let r = repo();
    claim(&r.ws, "s1", &live(), "a");
    let refs = git(&r.main, &["for-each-ref", "--format=%(refname)"]);
    assert!(refs.contains("refs/cardinal/claims/a"), "{refs}");
    assert!(!refs.lines().any(
        |l| (l.starts_with("refs/heads/") || l.starts_with("refs/tags/")) && l.ends_with("/a")
    ));
}

// spec: CLM-9
#[test]
fn a_new_claim_needs_a_clean_worktree_outside_the_issues() {
    let r = repo();
    std::fs::write(
        r.ws.join("issues/a.md"),
        "---\npriority: high\n---\n# Edited\n",
    )
    .unwrap();
    std::fs::write(r.ws.join("new.txt"), "untracked\n").unwrap();
    let (code, out) = claim(&r.ws, "s1", &live(), "a");
    assert_eq!(code, Some(0), "issues and untracked files pass: {out}");
    std::fs::write(r.ws.join("code.txt"), "wip\n").unwrap();
    let (code, _) = claim(&r.ws, "s1", "4242", "a");
    assert_eq!(code, Some(0), "taking a claim back checks nothing");
    let (code, out) = claim(&r.ws, "s1", &live(), "b");
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("uncommitted changes"), "{out}");
    assert_eq!(claim_text(&r.main, "b"), "");
}

fn git_stdin(dir: &Path, args: &[&str], input: &str) -> String {
    use std::io::Write;
    let mut child = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout)
        .trim()
        .to_string()
}
