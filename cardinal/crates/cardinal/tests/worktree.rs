//! Behavior test for `cardinal tool worktree` (`open` and `close`) against a throwaway git repository.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
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
        .unwrap()
        .status
        .success()
}

fn git_ok(dir: &Path, args: &[&str]) {
    assert!(git(dir, args), "git {args:?} failed");
}

fn branch_exists(repo: &Path, name: &str) -> bool {
    git(
        repo,
        &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")],
    )
}

struct Ctx {
    root: PathBuf,
    repo: PathBuf,
    script: PathBuf,
}

type Checks = Vec<(&'static str, bool)>;

impl Ctx {
    fn run(&self, cwd: &Path, args: &[&str]) -> (bool, String) {
        if !cwd.is_dir() {
            return (false, format!("{} does not exist", cwd.display()));
        }
        let out = tool_command(&self.script)
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr),
        )
    }

    /// Where `open` puts a worktree by default: next to the main one.
    fn wt(&self, name: &str) -> PathBuf {
        self.root.join(format!("repo-{name}"))
    }

    /// Where the former `start-issue` put the worktrees (ADR 0008).
    fn nested(&self, stem: &str) -> PathBuf {
        self.repo.join(".claude/worktrees").join(stem)
    }

    fn entries(&self) -> usize {
        std::fs::read_dir(&self.root).unwrap().count()
    }
}

// spec: PAR-1
fn open_creates_the_worktree(c: &Ctx) -> Checks {
    let (ok, out) = c.run(&c.repo, &["open", "x"]);
    let created = ok && c.wt("x").join("issues/x.md").is_file() && branch_exists(&c.repo, "x");
    let (linked_ok, _) = c.run(&c.wt("x"), &["open", "y"]);
    let (named_ok, named_out) = c.run(
        &c.repo,
        &[
            "open",
            "n",
            "--path",
            "../elsewhere/n",
            "--branch",
            "feat/n",
        ],
    );
    let named = named_ok
        && c.root.join("elsewhere/n/issues/x.md").is_file()
        && branch_exists(&c.repo, "feat/n")
        && !branch_exists(&c.repo, "n");
    let (no_issue, _) = c.run(&c.repo, &["open", "no-issue-file"]);
    // a branch checked out nowhere, here a merged one, is checked out as it is
    git_ok(&c.repo, &["branch", "kept"]);
    let (reopened, _) = c.run(&c.repo, &["open", "kept"]);
    let exclude = std::fs::read_to_string(c.repo.join(".git/info/exclude")).unwrap_or_default();
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&c.repo)
        .output()
        .unwrap();
    let removed = git(
        &c.repo,
        &[
            "worktree",
            "remove",
            c.wt("no-issue-file").to_str().unwrap(),
        ],
    ) && git(&c.repo, &["branch", "-D", "no-issue-file"])
        && git(
            &c.repo,
            &["worktree", "remove", c.wt("kept").to_str().unwrap()],
        )
        && git(&c.repo, &["branch", "-D", "kept"])
        && git(
            &c.repo,
            &[
                "worktree",
                "remove",
                c.root.join("elsewhere/n").to_str().unwrap(),
            ],
        )
        && git(&c.repo, &["branch", "-D", "feat/n"]);
    let _ = std::fs::remove_dir_all(c.root.join("elsewhere"));
    vec![
        (
            "open creates <repo>-<name> next to the main worktree on a branch <name>",
            created,
        ),
        (
            "open prints the worktree path",
            out.trim() == c.wt("x").to_str().unwrap(),
        ),
        (
            "open works from a linked worktree and puts the new one next to the main one",
            linked_ok && c.wt("y").is_dir(),
        ),
        (
            "open takes --path relative to the current directory and --branch",
            named && named_out.trim() == c.root.join("elsewhere/n").to_str().unwrap(),
        ),
        ("open needs no issue file", no_issue),
        (
            "open checks out an existing branch checked out nowhere",
            reopened,
        ),
        (
            "open writes nothing to info/exclude and leaves the main worktree clean",
            !exclude.contains(".claude/worktrees") && status.stdout.is_empty(),
        ),
        ("the checks clean up after themselves", removed),
    ]
}

const PERSONAL: &str = ".claude/cardinal.local.toml";
const PERSONAL_TEXT: &str = "[project]\nuser_language = \"English\"\n";

// spec: PAR-17
fn open_copies_the_personal_configuration(c: &Ctx) -> Checks {
    let copied = std::fs::read_to_string(c.wt("x").join(PERSONAL)).ok();
    vec![(
        "open copies the personal configuration of the main worktree into the new one",
        copied.as_deref() == Some(PERSONAL_TEXT),
    )]
}

// spec: PAR-2
fn open_refuses_and_changes_nothing(c: &Ctx) -> Checks {
    let before = c.entries();
    let (checked_out, _) = c.run(&c.repo, &["open", "x"]);
    let (elsewhere, _) = c.run(&c.repo, &["open", "other", "--branch", "x"]);
    let (bad_name, _) = c.run(&c.repo, &["open", "../up"]);
    let (dash, _) = c.run(&c.repo, &["open", "-x"]);
    let (dot, _) = c.run(&c.repo, &["open", ".x"]);
    std::fs::create_dir_all(c.wt("z")).unwrap();
    let (occupied, _) = c.run(&c.repo, &["open", "z"]);
    let (nested, _) = c.run(&c.repo, &["open", "q", "--path", ".claude/worktrees/q"]);
    std::fs::remove_dir_all(c.wt("z")).unwrap();
    git_ok(&c.repo, &["checkout", "-q", "--detach"]);
    let (detached, _) = c.run(&c.repo, &["open", "z"]);
    let detached_clean = !branch_exists(&c.repo, "z") && !c.wt("z").exists();
    git_ok(&c.repo, &["checkout", "-q", "trunk"]);
    vec![
        (
            "open refuses to cut a branch from a detached main worktree",
            !detached && detached_clean,
        ),
        (
            "open refuses a branch checked out in a worktree",
            !checked_out && !elsewhere && !branch_exists(&c.repo, "other"),
        ),
        (
            "open refuses a name outside its characters",
            !bad_name && !dash && !dot,
        ),
        (
            "open refuses a path inside the main worktree",
            !nested && !branch_exists(&c.repo, "q") && !c.repo.join(".claude/worktrees/q").exists(),
        ),
        (
            "open refuses an existing path",
            !occupied && !branch_exists(&c.repo, "z"),
        ),
        ("a refusal changes nothing", c.entries() == before),
    ]
}

// spec: PAR-6
fn close_needs_a_merged_branch(c: &Ctx) -> Checks {
    std::fs::write(c.wt("x").join("work.txt"), "work\n").unwrap();
    git_ok(&c.wt("x"), &["add", "-A"]);
    git_ok(&c.wt("x"), &["commit", "-qm", "work", "--no-verify"]);
    let (unmerged, _) = c.run(&c.repo, &["close", "x"]);
    let kept = c.wt("x").is_dir() && branch_exists(&c.repo, "x");
    git_ok(&c.repo, &["merge", "-q", "--ff-only", "x"]);
    let (merged, _) = c.run(&c.repo, &["close", "x"]);
    vec![
        (
            "close refuses an unmerged branch and keeps both",
            !unmerged && kept,
        ),
        (
            "close removes the worktree and the branch once merged",
            merged && !c.wt("x").exists() && !branch_exists(&c.repo, "x"),
        ),
    ]
}

// spec: PAR-7
fn close_removes_only_a_clean_worktree(c: &Ctx) -> Checks {
    std::fs::write(c.wt("y").join("dirty.txt"), "wip\n").unwrap();
    let (dirty, _) = c.run(&c.repo, &["close", "y"]);
    let (by_branch, _) = c.run(&c.repo, &["close", "trunk", "--force"]);
    let (by_path, _) = c.run(&c.wt("y"), &["close", c.repo.to_str().unwrap(), "--force"]);
    // an agent worktree nested in the issue worktree is ignored there, so `git status` alone does not see it
    let (ok, _) = c.run(&c.repo, &["open", "z"]);
    let agent = c.wt("z").join(".claude/worktrees/agent");
    git_ok(
        &c.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "agent",
            agent.to_str().unwrap(),
        ],
    );
    std::fs::write(agent.join("wip.txt"), "wip\n").unwrap();
    let (holding, _) = c.run(&c.repo, &["close", "z", "--force"]);
    let held = agent.join("wip.txt").is_file() && c.wt("z").is_dir();
    git_ok(
        &c.repo,
        &["worktree", "remove", "--force", agent.to_str().unwrap()],
    );
    let (freed, _) = c.run(&c.repo, &["close", "z"]);
    vec![
        (
            "close refuses a worktree with uncommitted changes",
            !dirty && c.wt("y").is_dir() && branch_exists(&c.repo, "y"),
        ),
        (
            "close refuses, even with --force, a worktree holding another worktree",
            ok && !holding && held,
        ),
        (
            "close removes it once the inner worktree is gone",
            freed && !c.wt("z").exists() && !branch_exists(&c.repo, "z"),
        ),
        (
            "close refuses the main worktree, named by its branch or its path",
            !by_branch && !by_path && c.repo.join(".git").is_dir(),
        ),
    ]
}

// spec: PAR-15
fn close_removes_a_legacy_nested_worktree(c: &Ctx) -> Checks {
    git_ok(
        &c.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "l",
            c.nested("l").to_str().unwrap(),
        ],
    );
    let (merged, _) = c.run(&c.repo, &["close", "l"]);
    git_ok(
        &c.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "k",
            c.nested("k").to_str().unwrap(),
        ],
    );
    commit_file(&c.nested("k"), "k.txt");
    let (unmerged, _) = c.run(&c.repo, &["close", "k"]);
    let (inside, _) = c.run(&c.nested("k"), &["close", "k", "--force"]);
    let kept = c.nested("k").is_dir() && branch_exists(&c.repo, "k");
    let (forced, _) = c.run(&c.repo, &["close", "k", "--force"]);
    vec![
        (
            "close removes a merged worktree left under .claude/worktrees/",
            merged && !c.nested("l").exists() && !branch_exists(&c.repo, "l"),
        ),
        (
            "close refuses an unmerged one, and from inside it, keeping both",
            !unmerged && !inside && kept,
        ),
        (
            "close --force removes it",
            forced && !c.nested("k").exists() && !branch_exists(&c.repo, "k"),
        ),
    ]
}

// spec: PAR-19
fn close_designates_by_path_name_or_branch(c: &Ctx) -> Checks {
    // a worktree reused on another branch than its name
    let (opened, _) = c.run(&c.repo, &["open", "a", "--branch", "b"]);
    let (by_name, _) = c.run(&c.repo, &["close", "a"]);
    let named = opened && !c.wt("a").exists() && !branch_exists(&c.repo, "b");
    // a worktree at a path of the user's, named by its branch, then by its path
    let elsewhere = c.root.join("elsewhere");
    let (opened_w, _) = c.run(&c.repo, &["open", "w", "--path", "../elsewhere"]);
    let (by_branch, _) = c.run(&c.repo, &["close", "w"]);
    let branched = opened_w && !elsewhere.exists() && !branch_exists(&c.repo, "w");
    let (opened_p, _) = c.run(&c.repo, &["open", "p", "--path", "../elsewhere"]);
    let (by_path, _) = c.run(&c.repo, &["close", "../elsewhere"]);
    let pathed = opened_p && !elsewhere.exists() && !branch_exists(&c.repo, "p");
    // a merged branch checked out nowhere, such as a stale branch of the former `start-issue`
    git_ok(&c.repo, &["branch", "stale"]);
    let (bare, _) = c.run(&c.repo, &["close", "stale"]);
    // a detached worktree: removed, no branch to delete
    let detached = c.root.join("detached");
    git_ok(
        &c.repo,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            detached.to_str().unwrap(),
        ],
    );
    commit_file(&detached, "detached.txt");
    let (unmerged, _) = c.run(&c.repo, &["close", "../detached"]);
    let detached_kept = detached.join("detached.txt").is_file();
    let (gone, _) = c.run(&c.repo, &["close", "../detached", "--force"]);
    // a detached worktree whose directory is gone still keeps its commits off trunk
    let lost = c.root.join("lost");
    git_ok(
        &c.repo,
        &["worktree", "add", "-q", "--detach", lost.to_str().unwrap()],
    );
    commit_file(&lost, "lost.txt");
    std::fs::remove_dir_all(&lost).unwrap();
    let (lost_plain, _) = c.run(&c.repo, &["close", "../lost"]);
    let (lost_forced, _) = c.run(&c.repo, &["close", "../lost", "--force"]);
    // a worktree reused on another branch while a branch of its name is left: ambiguous
    let (opened_amb, _) = c.run(&c.repo, &["open", "amb", "--branch", "next"]);
    git_ok(&c.repo, &["branch", "amb"]);
    let (ambiguous, amb_msg) = c.run(&c.repo, &["close", "amb"]);
    let amb_kept =
        c.wt("amb").is_dir() && branch_exists(&c.repo, "next") && branch_exists(&c.repo, "amb");
    let (amb_path, _) = c.run(&c.repo, &["close", "../repo-amb"]);
    git_ok(&c.repo, &["branch", "-D", "amb"]);
    let before = c.entries();
    let (nothing, msg) = c.run(&c.repo, &["close", "nothing-here"]);
    vec![
        (
            "close <name> removes <repo>-<name> and the branch it has checked out",
            by_name && named,
        ),
        (
            "close <branch> removes the worktree that has it checked out",
            by_branch && branched,
        ),
        (
            "close <path> removes the worktree at that path",
            by_path && pathed,
        ),
        (
            "close deletes a merged branch checked out nowhere",
            bare && !branch_exists(&c.repo, "stale"),
        ),
        (
            "close refuses a detached worktree with commits off trunk, and --force removes it",
            !unmerged && detached_kept && gone && !detached.exists(),
        ),
        (
            "close refuses a detached worktree with a missing directory and commits off trunk, --force prunes it",
            !lost_plain && lost_forced,
        ),
        (
            "close refuses a name that designates a reused worktree and a branch, the path removes the worktree",
            opened_amb && !ambiguous && amb_msg.contains("git branch -D amb") && amb_kept && amb_path && !c.wt("amb").exists(),
        ),
        (
            "close fails, removing nothing, when nothing is designated",
            !nothing && msg.contains("nothing-here") && c.entries() == before,
        ),
    ]
}

fn commit_file(wt: &Path, name: &str) {
    std::fs::write(wt.join(name), format!("{name}\n")).unwrap();
    git_ok(wt, &["add", "-A"]);
    git_ok(wt, &["commit", "-qm", name, "--no-verify"]);
}

// spec: PAR-6
fn close_accepts_a_patch_equivalent_branch(c: &Ctx) -> Checks {
    for stem in ["r", "m"] {
        let (ok, _) = c.run(&c.repo, &["open", stem]);
        assert!(ok);
        commit_file(&c.wt(stem), &format!("{stem}.txt"));
    }
    commit_file(&c.repo, "trunk-moved.txt");
    git_ok(&c.repo, &["cherry-pick", "r"]);
    git_ok(&c.repo, &["merge", "-q", "--squash", "m"]);
    git_ok(&c.repo, &["commit", "-qm", "squash m", "--no-verify"]);
    let (rebased, _) = c.run(&c.repo, &["close", "r"]);
    let (squashed, _) = c.run(&c.repo, &["close", "m"]);
    vec![
        (
            "close accepts a rebased branch",
            rebased && !c.wt("r").exists() && !branch_exists(&c.repo, "r"),
        ),
        (
            "close accepts a branch squashed from one commit",
            squashed && !c.wt("m").exists() && !branch_exists(&c.repo, "m"),
        ),
    ]
}

// spec: PAR-6
fn close_accepts_a_squash_of_several_commits(c: &Ctx) -> Checks {
    for stem in ["s", "t"] {
        let (ok, _) = c.run(&c.repo, &["open", stem]);
        assert!(ok);
        commit_file(&c.wt(stem), &format!("{stem}1.txt"));
        commit_file(&c.wt(stem), &format!("{stem}2.txt"));
        git_ok(&c.repo, &["merge", "-q", "--squash", stem]);
        git_ok(
            &c.repo,
            &["commit", "-qm", &format!("squash {stem}"), "--no-verify"],
        );
    }
    let (squashed, _) = c.run(&c.repo, &["close", "s"]);
    // trunk rewrites a squashed file: the content of `t` is no longer all on trunk, so it counts as unmerged.
    std::fs::write(c.repo.join("t1.txt"), "rewritten\n").unwrap();
    git_ok(&c.repo, &["commit", "-qam", "rewrite t1", "--no-verify"]);
    let (rewritten, _) = c.run(&c.repo, &["close", "t"]);
    vec![
        (
            "close accepts a branch squashed from several commits",
            squashed && !c.wt("s").exists() && !branch_exists(&c.repo, "s"),
        ),
        (
            "close keeps a squashed branch whose content trunk changed afterwards",
            !rewritten && c.wt("t").is_dir() && branch_exists(&c.repo, "t"),
        ),
    ]
}

// spec: PAR-10
fn close_forces_work_that_is_not_on_trunk(c: &Ctx) -> Checks {
    let (ok, _) = c.run(&c.repo, &["open", "u"]);
    assert!(ok);
    commit_file(&c.wt("u"), "u1.txt");
    // a tag named like the branch, on trunk, must not stand for the branch
    git_ok(&c.repo, &["tag", "u", "trunk"]);
    let (plain, msg) = c.run(&c.repo, &["close", "u"]);
    git_ok(&c.repo, &["tag", "-d", "u"]);
    let kept = c.wt("u").is_dir() && branch_exists(&c.repo, "u");
    std::fs::write(c.wt("u").join("dirty.txt"), "wip\n").unwrap();
    let (dirty, _) = c.run(&c.repo, &["close", "u", "--force"]);
    let dirty_kept = c.wt("u").is_dir() && branch_exists(&c.repo, "u");
    std::fs::remove_file(c.wt("u").join("dirty.txt")).unwrap();
    let (forced, _) = c.run(&c.repo, &["close", "u", "--force"]);
    vec![
        (
            "close refuses work that is not on trunk without --force, names --force and keeps both",
            !plain && kept && msg.contains("--force"),
        ),
        (
            "close --force still refuses a worktree with uncommitted changes",
            !dirty && dirty_kept,
        ),
        (
            "close --force removes the worktree and the branch",
            forced && !c.wt("u").exists() && !branch_exists(&c.repo, "u"),
        ),
    ]
}

// spec: PAR-11
fn close_refuses_from_inside_the_worktree(c: &Ctx) -> Checks {
    let (ok, _) = c.run(&c.repo, &["open", "c"]);
    assert!(ok);
    let sub = c.wt("c").join("issues");
    let (inside, msg) = c.run(&c.wt("c"), &["close", "c"]);
    let (below, _) = c.run(&sub, &["close", "c"]);
    let (forced, _) = c.run(&sub, &["close", "c", "--force"]);
    let kept = c.wt("c").is_dir() && branch_exists(&c.repo, "c");
    commit_file(&c.wt("c"), "c.txt");
    let (unmerged, unmerged_msg) = c.run(&c.wt("c"), &["close", "c"]);
    let (outside_unmerged, _) = c.run(&c.repo, &["close", "c"]);
    git_ok(&c.repo, &["merge", "-q", "--ff-only", "c"]);
    let (outside, _) = c.run(&c.repo, &["close", "c"]);
    vec![
        (
            "close refuses from the worktree to remove and names the main worktree",
            !inside && msg.contains(c.repo.to_str().unwrap()),
        ),
        (
            "close refuses from a subdirectory of it, with or without --force",
            !below && !forced,
        ),
        ("a refusal keeps the worktree and the branch", kept),
        (
            "an unmerged branch gets the same refusal from inside",
            !unmerged && unmerged_msg.contains("inside") && !outside_unmerged,
        ),
        (
            "close from the main worktree still removes it",
            outside && !c.wt("c").exists() && !branch_exists(&c.repo, "c"),
        ),
    ]
}

#[test]
fn worktree() {
    let script = std::path::PathBuf::from("/tools").join("worktree.rs");
    let root = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("issues")).unwrap();
    for s in ["x", "y", "z", "w", "r", "m", "s", "t", "u", "c"] {
        std::fs::write(repo.join(format!("issues/{s}.md")), format!("# {s}\n")).unwrap();
    }
    // the managed `.claude/.gitignore` and a personal configuration it ignores
    std::fs::create_dir_all(repo.join(".claude")).unwrap();
    std::fs::write(repo.join(".claude/.gitignore"), "cardinal.local.toml\n").unwrap();
    std::fs::write(repo.join(PERSONAL), PERSONAL_TEXT).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "trunk", "."]);
    git_ok(&repo, &["add", "-A"]);
    git_ok(&repo, &["commit", "-qm", "init", "--no-verify"]);

    let c = Ctx {
        root: root.clone(),
        repo,
        script,
    };
    let mut failed = false;
    for checks in [
        open_creates_the_worktree(&c),
        open_copies_the_personal_configuration(&c),
        open_refuses_and_changes_nothing(&c),
        close_needs_a_merged_branch(&c),
        close_removes_only_a_clean_worktree(&c),
        close_removes_a_legacy_nested_worktree(&c),
        close_designates_by_path_name_or_branch(&c),
        close_accepts_a_patch_equivalent_branch(&c),
        close_accepts_a_squash_of_several_commits(&c),
        close_forces_work_that_is_not_on_trunk(&c),
        close_refuses_from_inside_the_worktree(&c),
    ] {
        for (name, ok) in checks {
            println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
            failed |= !ok;
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// `cardinal tool <name>` for the script path the checks were written against (`next_issue.rs` is `next-issue`).
fn tool_command(script: impl AsRef<std::path::Path>) -> std::process::Command {
    let stem = script
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('_', "-");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.arg("tool").arg(stem);
    cmd
}
