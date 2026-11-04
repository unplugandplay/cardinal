//! Behavior test for `cardinal tool next-issue`, and of the next issue `cardinal tool close-issue` offers after a close.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
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
        .success();
    assert!(ok, "git {args:?} failed");
}

/// A branch `name` with one commit of its own, cut from `trunk`; the checkout goes back to `trunk`.
fn branch_with_commit(repo: &Path, name: &str) {
    git(repo, &["checkout", "-q", "-b", name]);
    std::fs::write(repo.join(format!("{name}.txt")), "work\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", name, "--no-verify"]);
    git(repo, &["checkout", "-q", "trunk"]);
}

// spec: PAR-3, PAR-4, PAR-5
/// An issue is taken when a worktree has its branch checked out or the branch holds commits that are not on the main
/// branch; `next_issue.rs` skips it, names it, and behaves as before when nothing is taken.
fn taken_checks(script: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("issues")).unwrap();
    for (i, s) in ["a", "b", "c", "d"].into_iter().enumerate() {
        std::fs::write(
            repo.join(format!("issues/{s}.md")),
            format!(
                "---\npriority: {}\n---\n# Title {s}\n",
                ["high", "medium", "low", "low", "low"][i]
            ),
        )
        .unwrap();
    }
    git(&repo, &["init", "-q", "-b", "trunk", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "init", "--no-verify"]);
    let run = |cwd: &Path| {
        let out = tool_command(script)
            .arg("issues")
            .current_dir(cwd)
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };

    let (code, free) = run(&repo);
    let mut out = vec![(
        "nothing taken keeps the output",
        code == Some(0) && free.contains("Next: issues/a.md") && !free.contains("aken"),
    )];
    branch_with_commit(&repo, "a");
    let (_, branch) = run(&repo);
    out.push((
        "a branch with commits off the main branch takes the issue, the others keep their position",
        branch.contains("Next: issues/b.md")
            && branch.contains("Taken: a\n")
            && branch.contains("  3. c")
            && branch.contains("  4. d"),
    ));
    let linked = root.join("repo-b");
    git(
        &repo,
        &["worktree", "add", "-q", "-b", "b", linked.to_str().unwrap()],
    );
    let (_, mid) = run(&repo);
    out.push((
        "a worktree path is named",
        mid.contains("Taken: b (worktree ") && mid.contains("repo-b)"),
    ));
    branch_with_commit(&repo, "c");
    branch_with_commit(&repo, "d");
    let (code, all) = run(&repo);
    out.push((
        "all taken is said",
        code == Some(0) && all.contains("Every open issue") && !all.contains("Next:"),
    ));
    let (_, elsewhere) = {
        let o = tool_command(script)
            .arg(repo.join("issues"))
            .current_dir(&root)
            .output()
            .unwrap();
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
        )
    };
    out.push((
        "git is queried in the repository of the issues directory",
        elsewhere.contains("Every open issue"),
    ));
    git(&repo, &["branch", "-D", "d"]);
    git(&repo, &["branch", "-m", "trunk", "d"]);
    let (_, trunk) = run(&repo);
    out.push((
        "the branch of the main worktree is not an issue",
        trunk.contains("Taken: a") && !trunk.contains("Taken: d"),
    ));
    git(&repo, &["branch", "-m", "d", "trunk"]);
    branch_with_commit(&repo, "d");
    let (_, from_linked) = run(&linked);
    out.push((
        "the mark is seen from another worktree",
        from_linked.contains("repo-b") && !from_linked.contains("Next:"),
    ));
    let plain = root.join("plain");
    std::fs::create_dir_all(plain.join("issues")).unwrap();
    std::fs::copy(repo.join("issues/a.md"), plain.join("issues/a.md")).unwrap();
    let (code, outside) = run(&plain);
    out.push((
        "outside a git repository nothing is taken",
        code == Some(0) && outside.contains("Next: issues/a.md"),
    ));
    let _ = std::fs::remove_dir_all(&root);
    out
}

// spec: PAR-3, PAR-4
/// A branch with no worktree and nothing off the main branch (left over, empty, or merged by rebase or squash of any number of commits) does not
/// take its issue: it is reported as stale, with the command that removes it.
fn stale_checks(script: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("issues")).unwrap();
    for (i, s) in ["a", "b", "c", "d", "e"].into_iter().enumerate() {
        std::fs::write(
            repo.join(format!("issues/{s}.md")),
            format!(
                "---\npriority: {}\n---\n# Title {s}\n",
                ["high", "medium", "low", "low", "low"][i]
            ),
        )
        .unwrap();
    }
    git(&repo, &["init", "-q", "-b", "trunk", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "init", "--no-verify"]);
    let run = || {
        let out = tool_command(script)
            .arg("issues")
            .current_dir(&repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    git(&repo, &["branch", "a"]);
    branch_with_commit(&repo, "b");
    git(&repo, &["cherry-pick", "b"]);
    branch_with_commit(&repo, "c");
    // d: two commits squash-merged into trunk. e: the same, then trunk changes the squashed file again.
    for name in ["d", "e"] {
        branch_with_commit(&repo, name);
        git(&repo, &["checkout", "-q", name]);
        std::fs::write(repo.join(format!("{name}.txt")), "more work\n").unwrap();
        git(&repo, &["commit", "-qam", "more", "--no-verify"]);
        git(&repo, &["checkout", "-q", "trunk"]);
        git(&repo, &["merge", "-q", "--squash", name]);
        git(
            &repo,
            &["commit", "-qm", &format!("squash {name}"), "--no-verify"],
        );
    }
    std::fs::write(repo.join("e.txt"), "trunk rewrote it\n").unwrap();
    git(&repo, &["commit", "-qam", "rewrite e", "--no-verify"]);
    let out = run();
    let checks = vec![
        (
            "an empty branch does not take the issue and is reported stale",
            out.contains("Next: issues/a.md")
                && out.contains("Stale: a")
                && out.contains("close-worktree a"),
        ),
        (
            "a branch merged by rebase is reported stale",
            out.contains("Stale: b") && !out.contains("Taken: b"),
        ),
        (
            "a branch with commits off the main branch stays taken",
            out.contains("Taken: c") && !out.contains("Stale: c"),
        ),
        (
            "a branch squash-merged from several commits is reported stale",
            out.contains("Stale: d") && !out.contains("Taken: d"),
        ),
        (
            "a squashed branch whose file trunk changed afterwards stays taken",
            out.contains("Taken: e") && !out.contains("Stale: e"),
        ),
    ];
    let _ = std::fs::remove_dir_all(&root);
    checks
}

#[test]
fn next_issue() {
    let dir = std::path::PathBuf::from("/tools");
    let tmp = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    let w = |f: &str, t: &str| std::fs::write(tmp.join("issues").join(f), t).unwrap();
    w("a.md", "---\npriority: low\n---\n# Title A\n");
    w("b.md", "---\npriority: high\n---\n# Title B\n");
    let run = |script: &str, arg: &str| {
        let out = tool_command(dir.join(script))
            .arg(arg)
            .current_dir(&tmp)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr),
        )
    };
    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (_, out) = run("next_issue.rs", "issues");
    check(
        "first line is the next issue, with its title",
        out.contains("Next: issues/b.md") && out.contains("Title B"),
    );
    run("close_issue.rs", "issues/b.md");
    let (_, out) = run("next_issue.rs", "issues");
    check(
        "next moves on after closing",
        out.contains("Next: issues/a.md"),
    );

    for (name, ok) in taken_checks(&dir.join("next_issue.rs"))
        .into_iter()
        .chain(stale_checks(&dir.join("next_issue.rs")))
    {
        check(name, ok);
    }

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// Runs `next-issue issues` in `root`: (exit code, stdout, stderr).
fn next_in(root: &Path) -> (Option<i32>, String, String) {
    let out = tool_command("next_issue.rs")
        .arg("issues")
        .current_dir(root)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A directory `issues/` holding the given files (a name under `done/` goes to `issues/done/`), outside any git
/// repository.
fn issues_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("issues/done")).unwrap();
    for (name, text) in files {
        std::fs::write(root.path().join("issues").join(name), text).unwrap();
    }
    root
}

/// The roadmap `ROADMAP.md` with the milestones `names` in that order, named in `.claude/cardinal.toml` of `root`.
fn roadmap(root: &Path, names: &[&str]) {
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    std::fs::write(
        root.join(".claude/cardinal.toml"),
        "[project]\nroadmap = [\"ROADMAP.md\"]\n",
    )
    .unwrap();
    let text = names.join("\n\n- State: idea\n\n## ");
    std::fs::write(root.join("ROADMAP.md"), format!("# Roadmap\n\n## {text}\n")).unwrap();
}

// spec: NXT-1
/// The priority orders the ready issues, `high` first, and the `why` is printed with the title.
#[test]
fn the_priority_orders_the_issues() {
    let root = issues_dir(&[
        ("a.md", "---\npriority: low\nwhy: later\n---\n# Title A\n"),
        (
            "b.md",
            "---\npriority: high\nwhy: needed first\n---\n# Title B\n",
        ),
        ("c.md", "---\npriority: medium\n---\n# Title C\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("Next: issues/b.md\n  Title B\n  why: needed first\n"),
        "{out}"
    );
    assert!(out.contains("  2. c\n  3. a\n"), "{out}");
    assert!(err.is_empty(), "{err}");
}

// spec: NXT-1
/// Issues of the same milestone and priority come oldest first, by the commit that added them, the ones git does not
/// track after them; outside git, by stem.
#[test]
fn the_oldest_issue_breaks_a_tie() {
    let root = issues_dir(&[("b.md", "---\npriority: medium\n---\n# Title B\n")]);
    let (_, out, _) = {
        let repo = root.path();
        git(repo, &["init", "-q", "-b", "trunk", "."]);
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "b", "--no-verify"]);
        std::fs::write(
            repo.join("issues/a.md"),
            "---\npriority: medium\n---\n# Title A\n",
        )
        .unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "a", "--no-verify"]);
        std::fs::write(
            repo.join("issues/0.md"),
            "---\npriority: medium\n---\n# Title 0\n",
        )
        .unwrap();
        let before = next_in(repo);
        // a renamed issue is as old as its rename: still tracked, before the untracked `0`
        git(repo, &["mv", "issues/b.md", "issues/z.md"]);
        git(repo, &["commit", "-qm", "z", "--no-verify"]);
        let (_, renamed, _) = next_in(repo);
        assert!(
            renamed.contains("Next: issues/a.md") && renamed.contains("  2. z\n  3. 0\n"),
            "{renamed}"
        );
        before
    };
    assert!(
        out.contains("Next: issues/b.md") && out.contains("  2. a\n  3. 0\n"),
        "{out}"
    );

    let root = issues_dir(&[
        ("b.md", "---\npriority: medium\n---\n# Title B\n"),
        ("a.md", "---\npriority: medium\n---\n# Title A\n"),
    ]);
    let (_, out, _) = next_in(root.path());
    assert!(out.contains("Next: issues/a.md"), "{out}");
}

// spec: NXT-2
/// An issue with no priority, or an unknown one, comes after the `low` ones and is reported.
#[test]
fn an_issue_without_a_priority_comes_last_and_is_reported() {
    let root = issues_dir(&[
        ("a.md", "---\npriority: low\n---\n# Title A\n"),
        ("b.md", "---\npriority: urgent\n---\n# Title B\n"),
        ("c.md", "# Title C\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(
        out.contains("Next: issues/a.md") && out.contains("  2. b\n  3. c\n"),
        "{out}"
    );
    assert!(
        err.contains("issue without a priority: b") && err.contains("issue without a priority: c"),
        "{err}"
    );
}

// spec: NXT-1
/// The front matter ends at its closing `---`, a quoted value is read without its quotes, and the title is the first
/// heading after it.
#[test]
fn the_front_matter_is_read_as_a_block() {
    let a = "---\n# a comment\npriority: 'low'\nwhy: \"later\"\n---\n# Title A\n";
    let root = issues_dir(&[
        ("a.md", a),
        ("b.md", "---\npriority: high\n---\n# Title B\n"),
        ("c.md", "---\nnot closed\n\n# Title C\n\npriority: high\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(
        out.contains("Next: issues/b.md") && out.contains("  2. a\n  3. c\n"),
        "{out}"
    );
    assert!(
        err.contains("issue without a priority: c") && !err.contains("priority: a"),
        "{err}"
    );
    let root = issues_dir(&[("a.md", a)]);
    let (_, out, _) = next_in(root.path());
    assert!(out.contains("  Title A\n  why: later\n"), "{out}");
}

// spec: NXT-7, NXT-11
/// An issue whose dependencies are not all done is never offered: it is listed after the ready ones with what blocks
/// it, and offered once they are done.
#[test]
fn a_blocked_issue_waits_for_its_dependencies() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\ndepends_on: [b, old]\n---\n# Title A\n",
        ),
        (
            "b.md",
            "---\npriority: low\ndepends_on: []\n---\n# Title B\n",
        ),
        (
            "c.md",
            "---\npriority: medium\ndepends_on: [old]\n---\n# Title C\n",
        ),
        ("done/old.md", "# Old\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("Next: issues/c.md") && out.contains("  2. b\n"),
        "{out}"
    );
    assert!(
        out.contains("Blocked: a (by b)\n") && !out.contains(". a\n"),
        "{out}"
    );
    std::fs::rename(
        root.path().join("issues/b.md"),
        root.path().join("issues/done/b.md"),
    )
    .unwrap();
    let (_, out, _) = next_in(root.path());
    assert!(
        out.contains("Next: issues/a.md") && !out.contains("Blocked"),
        "{out}"
    );
}

// spec: NXT-12
/// When every free issue is blocked, none is offered.
#[test]
fn no_issue_is_offered_when_every_free_one_is_blocked() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\ndepends_on: [b]\n---\n# Title A\n",
        ),
        ("b.md", "---\npriority: low\n---\n# Title B\n"),
    ]);
    let repo = root.path();
    git(repo, &["init", "-q", "-b", "trunk", "."]);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "init", "--no-verify"]);
    branch_with_commit(repo, "b");
    let (code, out, err) = next_in(repo);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.starts_with("No ready issue: every free one waits on another.\n")
            && out.contains("Blocked: a (by b)")
            && out.contains("Taken: b"),
        "{out}"
    );
}

// spec: NXT-8
/// A dependency on no known issue, or a `depends_on` that is not a list, is reported and blocks the issue.
#[test]
fn an_unknown_or_invalid_dependency_is_reported() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\ndepends_on: [gone, gone]\n---\n# Title A\n",
        ),
        (
            "b.md",
            "---\npriority: high\ndepends_on: c\n---\n# Title B\n",
        ),
        ("c.md", "---\npriority: low\n---\n# Title C\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(
        err.matches("unknown dependency of a: gone").count(),
        1,
        "{err}"
    );
    assert!(err.contains("invalid depends_on: b"), "{err}");
    assert!(
        out.contains("Next: issues/c.md")
            && out.contains("Blocked: a (by gone)")
            && out.contains("Blocked: b (by an invalid depends_on)"),
        "{out}"
    );
}

// spec: NXT-9
/// Issues that depend on each other are reported once as a cycle, and stay blocked.
#[test]
fn a_dependency_cycle_is_reported_once() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\ndepends_on: [b]\n---\n# Title A\n",
        ),
        (
            "b.md",
            "---\npriority: high\ndepends_on: [a]\n---\n# Title B\n",
        ),
        (
            "c.md",
            "---\npriority: low\ndepends_on: [c]\n---\n# Title C\n",
        ),
        (
            "d.md",
            "---\npriority: low\ndepends_on: [a]\n---\n# Title D\n",
        ),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(err.matches("dependency cycle: a, b\n").count(), 1, "{err}");
    assert!(
        err.contains("dependency cycle: c\n") && !err.contains("cycle: a, b, d"),
        "{err}"
    );
    assert!(
        out.contains("Blocked: a (by b)")
            && out.contains("Blocked: b (by a)")
            && out.contains("Blocked: d (by a)"),
        "{out}"
    );
}

// spec: NXT-10
/// An issue of an earlier milestone of the roadmap comes first whatever its priority; an issue with no milestone after
/// those with one.
#[test]
fn the_milestone_order_comes_before_the_priority() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\n---\n# Title A\n\n# Milestone\n\nTwo\n",
        ),
        (
            "b.md",
            "---\npriority: low\n---\n# Title B\n\n# Milestone (optional; when the roadmap has milestones)\nOne\n\n# Resources\n",
        ),
        ("c.md", "---\npriority: high\n---\n# Title C\n"),
    ]);
    roadmap(root.path(), &["One", "Two"]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("Next: issues/b.md") && out.contains("  2. a\n  3. c\n"),
        "{out}"
    );
}

// spec: NXT-10
/// A heading inside an HTML comment of the roadmap gives no milestone its order.
#[test]
fn a_commented_heading_orders_no_milestone() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\n---\n# Title A\n\n# Milestone\nTwo\n",
        ),
        (
            "b.md",
            "---\npriority: low\n---\n# Title B\n\n# Milestone\nOne\n",
        ),
    ]);
    roadmap(root.path(), &["One", "Two"]);
    std::fs::write(
        root.path().join("ROADMAP.md"),
        "# Roadmap\n\n<!--\n## Two\n-->\n\n## One\n\n## Two\n",
    )
    .unwrap();
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(out.contains("Next: issues/b.md"), "{out}");
}

// spec: NXT-13
/// A milestone the roadmap does not name is reported, the issue ordered as one with no milestone.
#[test]
fn an_unknown_milestone_is_reported() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: high\n---\n# Title A\n\n# Milestone\nGone\n",
        ),
        (
            "b.md",
            "---\npriority: low\n---\n# Title B\n\n# Milestone\nOne\n",
        ),
    ]);
    roadmap(root.path(), &["One"]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(err.contains("unknown milestone of a: Gone"), "{err}");
    assert!(out.contains("Next: issues/b.md"), "{out}");
}

// spec: NXT-14
/// With no roadmap, the milestones are ignored and nothing is reported.
#[test]
fn without_a_roadmap_the_priority_orders_alone() {
    let root = issues_dir(&[
        (
            "a.md",
            "---\npriority: low\n---\n# Title A\n\n# Milestone\nOne\n",
        ),
        ("b.md", "---\npriority: high\n---\n# Title B\n"),
    ]);
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("Next: issues/b.md") && err.is_empty(),
        "{out}{err}"
    );
    std::fs::create_dir_all(root.path().join(".claude")).unwrap();
    std::fs::write(
        root.path().join(".claude/cardinal.toml"),
        "[project]\nroadmap = [\"MISSING.md\"]\n",
    )
    .unwrap();
    let (code, out, err) = next_in(root.path());
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("Next: issues/b.md") && err.is_empty(),
        "{out}{err}"
    );
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
