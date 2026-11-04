//! Behavior test for `cardinal tool close-issue`: moves tracked and untracked issues, refuses bad targets.

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(script: &Path, cwd: &Path, arg: &str) -> (bool, String) {
    let out = tool_command(script)
        .arg(arg)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

// spec: PAR-18
/// An issue is closed in the worktree that holds it, the main one or a linked one, never from another worktree.
fn closes_only_in_its_own_worktree(script: &Path) -> Vec<(&'static str, bool)> {
    let root = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    let main = root.join("repo");
    let linked = root.join("repo-foo");
    std::fs::create_dir_all(main.join("issues")).unwrap();
    std::fs::write(main.join("issues/foo.md"), "x").unwrap();
    let git = |args: &[&str]| {
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
            .current_dir(&main)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "foo",
        linked.to_str().unwrap(),
    ]);
    let (by_path, by_path_out) = run(script, &main, "../repo-foo/issues/foo.md");
    let untouched_by_path = linked.join("issues/foo.md").is_file();
    let (from_linked, _) = run(script, &linked, "../repo/issues/foo.md");
    let main_untouched = main.join("issues/foo.md").is_file();
    let (in_linked, out) = run(script, &linked, "issues/foo.md");
    let (relink, _) = run(script, &linked, "--relink");
    let (in_main, _) = run(script, &main, "issues/foo.md");
    let out = vec![
        (
            "a linked worktree closes its own issue",
            in_linked && linked.join("issues/done/foo.md").is_file() && out.contains("moved"),
        ),
        ("--relink runs there too", relink),
        (
            "the linked worktree's issue is refused by path from the main one",
            !by_path && by_path_out.contains("another worktree") && untouched_by_path,
        ),
        (
            "the main worktree's issue is refused by path from a linked one",
            !from_linked && main_untouched,
        ),
        (
            "the main worktree closes the issue",
            in_main && main.join("issues/done/foo.md").is_file(),
        ),
    ];
    let _ = std::fs::remove_dir_all(&root);
    out
}

/// A main worktree with issues and references, and two linked worktrees nested in it; returns (main, the one under
/// `.claude/worktrees`, the one under `docs`).
fn nested_worktrees() -> (PathBuf, PathBuf, PathBuf) {
    let main = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    let _ = std::fs::remove_dir_all(&main);
    let nested = main.join(".claude/worktrees/bar");
    let elsewhere = main.join("docs/baz");
    std::fs::create_dir_all(main.join("issues")).unwrap();
    std::fs::create_dir_all(main.join("docs/worktrees")).unwrap();
    std::fs::write(main.join("issues/foo.md"), "x").unwrap();
    std::fs::write(main.join("issues/bar.md"), "x").unwrap();
    std::fs::write(main.join("issues/baz.md"), "x").unwrap();
    std::fs::write(main.join("NOTES.md"), "see issues/foo.md\n").unwrap();
    std::fs::write(main.join("docs/worktrees/w.md"), "see issues/foo.md\n").unwrap();
    // a tracked link into a nested worktree is not followed
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../.claude/worktrees/bar/NOTES.md",
        main.join("docs/link.md"),
    )
    .unwrap();
    let git = |args: &[&str]| {
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
            .current_dir(&main)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "bar",
        nested.to_str().unwrap(),
    ]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "baz",
        elsewhere.to_str().unwrap(),
    ]);
    (main, nested, elsewhere)
}

// spec: PAR-14
/// The issue worktrees nest in the main one (ADR 0008): closing an issue there must not rewrite their files, ignored
/// by git (`info/exclude`) or listed as an embedded repository (a clone that never ran `start-issue`), wherever they are.
fn skips_the_nested_worktrees(script: &Path) -> Vec<(&'static str, bool)> {
    let (main, nested, elsewhere) = nested_worktrees();
    std::fs::write(main.join("docs/draft.md"), "see issues/foo.md\n").unwrap();
    let read = |p: &Path| std::fs::read_to_string(p).unwrap();
    let kept = |wt: &Path| {
        read(&wt.join("NOTES.md")) == "see issues/foo.md\n"
            && read(&wt.join("docs/worktrees/w.md")) == "see issues/foo.md\n"
    };
    let (closed, _) = run(script, &main, "issues/foo.md");
    let nested_kept = kept(&nested);
    let elsewhere_kept = kept(&elsewhere);
    std::fs::write(main.join(".git/info/exclude"), "/.claude/worktrees/\n").unwrap();
    std::fs::write(main.join("docs/later.md"), "see issues/foo.md\n").unwrap();
    let (relinked, _) = run(script, &main, "--relink");
    std::fs::write(main.join(".git/index"), "corrupt").unwrap();
    let (relink_failed, failed_out) = run(script, &main, "--relink");
    let out = vec![
        (
            "a failing git ls-files fails, naming --relink",
            !relink_failed && failed_out.contains("--relink"),
        ),
        (
            "closing an issue rewrites the main worktree's references",
            closed && read(&main.join("NOTES.md")) == "see issues/done/foo.md\n",
        ),
        (
            "a directory named worktrees elsewhere is still scanned",
            read(&main.join("docs/worktrees/w.md")) == "see issues/done/foo.md\n",
        ),
        (
            "an untracked document of the main worktree is rewritten",
            read(&main.join("docs/draft.md")) == "see issues/done/foo.md\n",
        ),
        (
            "the files of a worktree under .claude/worktrees git does not ignore are untouched",
            nested_kept,
        ),
        (
            "the files of a worktree nested anywhere else are untouched",
            elsewhere_kept,
        ),
        (
            "--relink rewrites an untracked document",
            relinked && read(&main.join("docs/later.md")) == "see issues/done/foo.md\n",
        ),
        (
            "--relink leaves the worktree git ignores untouched",
            kept(&nested) && kept(&elsewhere),
        ),
    ];
    let _ = std::fs::remove_dir_all(&main);
    out
}

// spec: CLS-2, CLS-3, CLS-4, CLS-5, CLS-6
/// The Markdown links to a closed issue are resolved against the directory of their file, so the relative ones
/// (`../issues/foo.md` from `docs/`) follow it to `issues/done/`, and those of the moved issue still resolve.
fn relinks_the_markdown_links(script: &Path) -> Vec<(&'static str, bool)> {
    let root = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    for dir in ["issues/done", "docs/adr"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    let write = |p: &str, text: &str| std::fs::write(root.join(p), text).unwrap();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    write(
        "issues/foo.md",
        "[bar](bar.md) [q](gone.md#top) [old](done/old.md) [d](../docs/a.md) [u](https://x.org/a.md) [o](../../out.md)\n[c](done/) [docs](../docs/) `[e](bar.md)`\n[^1]: Some text\n[y]: <bar.md>\n```text\n[f](bar.md)\n```\n",
    );
    write("issues/bar.md", "[foo](foo.md)\n[f]: ./foo.md#goal\n");
    write("issues/old.md", "x");
    write(
        "docs/a.md",
        "[a](../issues/foo.md) [g](../issues/foo.md#goal) [s](issues/sibling.md)\n[z]: <../issues/foo.md>\n`[k](../issues/foo.md)`\n",
    );
    write("docs/adr/b.md", "[b](../../issues/foo.md)\n");
    write(
        "README.md",
        "[x](../issues/foo.md) [y](docs/issues/foo.md)\n",
    );
    write("notes.txt", "(../issues/foo.md)\n");
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .unwrap()
        .success());
    let (closed, _) = run(script, &root, "issues/foo.md");
    let docs = read("docs/a.md");
    let adr = read("docs/adr/b.md");
    let bar = read("issues/bar.md");
    let moved = read("issues/done/foo.md");
    let readme = read("README.md");
    let notes = read("notes.txt");
    // a link to an issue closed with an older tool, before relative links were followed
    std::fs::rename(root.join("issues/old.md"), root.join("issues/done/old.md")).unwrap();
    write("docs/c.md", "[o](../issues/old.md)\n");
    let (relinked, _) = run(script, &root, "--relink");
    let out = vec![
        ("closing an issue with Markdown links succeeds", closed),
        (
            "a relative link from docs/ and docs/adr/ follows the issue, anchor kept",
            docs == "[a](../issues/done/foo.md) [g](../issues/done/foo.md#goal) [s](issues/sibling.md)\n[z]: <../issues/done/foo.md>\n`[k](../issues/foo.md)`\n"
                && adr == "[b](../../issues/done/foo.md)\n",
        ),
        (
            "a link between issues follows the issue, inline and reference",
            bar == "[foo](done/foo.md)\n[f]: done/foo.md#goal\n",
        ),
        (
            "a link that does not resolve to the issue is untouched",
            readme == "[x](../issues/foo.md) [y](docs/issues/foo.md)\n"
                && notes == "(../issues/foo.md)\n",
        ),
        (
            "the links of the moved issue still resolve from done/",
            moved
                == "[bar](../bar.md) [q](../gone.md#top) [old](old.md) [d](../../docs/a.md) [u](https://x.org/a.md) [o](../../../out.md)\n[c](./) [docs](../../docs/) `[e](bar.md)`\n[^1]: Some text\n[y]: <../bar.md>\n```text\n[f](bar.md)\n```\n",
        ),
        (
            "--relink repairs a relative link to an issue already in done/",
            relinked && read("docs/c.md") == "[o](../issues/done/old.md)\n",
        ),
    ];
    let _ = std::fs::remove_dir_all(&root);
    out
}

/// `--relink` in `tmp` repairs the references to the done issues only, outside the skipped directories.
fn relink_checks(script: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    let mut check = |name, ok| out.push((name, ok));
    std::fs::write(tmp.join("issues/done/README.md"), "x").unwrap();
    std::fs::write(
        tmp.join("NOTES.md"),
        "issues/untracked.md issues/README.md issues/other.md",
    )
    .unwrap();
    std::fs::write(tmp.join("issues/other.md"), "x").unwrap();
    for dir in [".claude", "vendor/docs", "node_modules/pkg"] {
        std::fs::create_dir_all(tmp.join(dir)).unwrap();
    }
    std::fs::write(
        tmp.join(".claude/cardinal.toml"),
        "[project]\nskipped_dirs = [\"vendor\"]\n",
    )
    .unwrap();
    std::fs::write(tmp.join("vendor/docs/v.md"), "issues/untracked.md").unwrap();
    std::fs::write(tmp.join("node_modules/pkg/n.md"), "issues/untracked.md").unwrap();
    let (ok, _) = run(script, tmp, "--relink");
    let notes = std::fs::read_to_string(tmp.join("NOTES.md")).unwrap();
    check(
        "--relink repairs done issues only",
        ok && notes == "issues/done/untracked.md issues/README.md issues/other.md",
    );
    let untouched =
        |p: &str| std::fs::read_to_string(tmp.join(p)).unwrap() == "issues/untracked.md";
    check(
        "skipped_dirs and node_modules are not rewritten",
        untouched("vendor/docs/v.md") && untouched("node_modules/pkg/n.md"),
    );
    out
}

// spec: CLS-7
/// After the move, the output ends on the issue `next-issue` offers once the closed one is gone; `--relink` names none.
fn prints_the_next_issue(script: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("issues")).unwrap();
    std::fs::write(
        root.join("issues/foo.md"),
        "---\npriority: high\n---\n# Foo\n",
    )
    .unwrap();
    std::fs::write(
        root.join("issues/bar.md"),
        "---\npriority: medium\n---\n# Bar\n",
    )
    .unwrap();
    std::fs::write(
        root.join("issues/baz.md"),
        "---\npriority: low\n---\n# Baz\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
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
            .current_dir(root)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    let worktree = root.join("baz-worktree");
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "baz",
        worktree.to_str().unwrap(),
    ]);
    let stdout = |arg: &str| {
        let out = tool_command(script)
            .arg(arg)
            .current_dir(root)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (first, first_out) = stdout("issues/foo.md");
    let (relink, relink_out) = stdout("--relink");
    let (taken, taken_out) = stdout("issues/bar.md");
    git(&["worktree", "remove", "--force", worktree.to_str().unwrap()]);
    git(&["branch", "-D", "baz"]);
    let (last, last_out) = stdout("issues/baz.md");
    vec![
        (
            "closing an issue ends on the next one and its title",
            first && first_out.trim_end().ends_with("Next: issues/bar.md\n  Bar"),
        ),
        (
            "--relink names no next issue",
            relink && !relink_out.contains("Next:"),
        ),
        (
            "an issue taken by a worktree is skipped: `Every open issue is taken.`",
            taken && taken_out.trim_end().ends_with("Every open issue is taken."),
        ),
        (
            "closing the last issue ends on `No open issue.`",
            last && last_out.trim_end().ends_with("No open issue."),
        ),
    ]
}

// spec: CLS-8
/// After the move, the milestones that list the closed issue and have no issue left open are set `done` and named;
/// the others stay as they are, and `--relink` sets none.
fn sets_the_milestone_done(script: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("issues/done")).unwrap();
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    std::fs::write(
        root.join(".claude/cardinal.toml"),
        "[project]\nroadmap = [\"./ROADMAP.md\"]\n",
    )
    .unwrap();
    for f in ["foo.md", "baz.md", "done/bar.md", "done/qux.md"] {
        std::fs::write(root.join("issues").join(f), "# X\n").unwrap();
    }
    let roadmap = "# Roadmap\n\n\
## One\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md)\n  - [bar](issues/done/bar.md)\n\n\
## Two\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md)\n  - [baz](issues/baz.md)\n\n\
## Three\n\n- State: shaped\n- Issues:\n  - [foo](issues/foo.md)\n\n\
## Four\n\n- State: in issues\n- Issues:\n  - [qux](issues/done/qux.md)\n\n\
## Five\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md)\n  - [gone](issues/done/gone.md)\n\n\
## Six\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md) ([PR](https://example.com/pull/3), [ADR](docs/adr/1.md))\n\n\
## Seven\n\n- State: in issues\n\n```markdown\n## Example\n```\n\n- Issues:\n  - [foo](issues/foo.md)\n\n\
<!--\n## Template\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md)\n-->\n";
    std::fs::write(root.join("ROADMAP.md"), roadmap).unwrap();
    let git = |args: &[&str]| {
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
            .current_dir(root)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    let (closed, out) = run(script, root, "issues/foo.md");
    let after = std::fs::read_to_string(root.join("ROADMAP.md")).unwrap();
    let expected = roadmap
        .replace("issues/foo.md", "issues/done/foo.md")
        .replacen("- State: in issues", "- State: done", 1)
        .replace("## Six\n\n- State: in issues", "## Six\n\n- State: done")
        .replace(
            "## Seven\n\n- State: in issues",
            "## Seven\n\n- State: done",
        );
    let (relink, relink_out) = run(script, root, "--relink");
    let unchanged = std::fs::read_to_string(root.join("ROADMAP.md")).unwrap() == expected;
    vec![
        (
            "only the milestones whose issues are all closed and that list the closed one are set done, other links aside",
            closed && after == expected,
        ),
        (
            "a fenced `## ` line cuts no milestone and a commented template milestone stays as it is",
            after.contains("## Seven\n\n- State: done")
                && after.contains("## Template\n\n- State: in issues")
                && out.contains("milestone Seven done")
                && !out.contains("milestone Template"),
        ),
        (
            "the milestone set done is printed",
            out.contains("milestone One done in ./ROADMAP.md")
                && out.contains("milestone Six done")
                && !out.contains("milestone Four"),
        ),
        (
            "--relink sets no milestone done",
            relink && unchanged && !relink_out.contains("milestone"),
        ),
    ]
}

// spec: CLS-9
/// A close of a tracked issue is staged whole, so a bare `git commit` holds it; a file the user had changed stays theirs.
fn stages_the_close(script: &Path) -> Vec<(&'static str, bool)> {
    let tmp = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&tmp)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    git(&["init", "-q"]);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    std::fs::create_dir_all(tmp.join("docs")).unwrap();
    std::fs::write(tmp.join("issues/foo.md"), "# Foo\n\n[bar](bar.md)\n").unwrap();
    std::fs::write(tmp.join("issues/bar.md"), "# Bar\n").unwrap();
    std::fs::write(tmp.join("docs/a.md"), "see issues/foo.md\n").unwrap();
    std::fs::write(tmp.join("docs/b.md"), "see issues/foo.md\n").unwrap();
    std::fs::write(tmp.join("docs/c.md"), "see issues/foo.md\n").unwrap();
    // a name git would read as a glob matching `docs/na.md`
    std::fs::write(tmp.join("docs/n[ab].md"), "see issues/foo.md\n").unwrap();
    std::fs::write(tmp.join("docs/na.md"), "x\n").unwrap();
    std::fs::create_dir_all(tmp.join(".claude")).unwrap();
    std::fs::write(
        tmp.join(".claude/cardinal.toml"),
        "[project]\nroadmap = [\"ROADMAP.md\"]\n",
    )
    .unwrap();
    std::fs::write(
        tmp.join("ROADMAP.md"),
        "# Roadmap\n\n## One\n\n- State: in issues\n- Issues:\n  - [foo](issues/foo.md)\n",
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "init"]);
    std::fs::write(tmp.join("docs/b.md"), "see issues/foo.md\nmine\n").unwrap();
    std::fs::write(tmp.join("docs/c.md"), "see issues/foo.md\nstaged\n").unwrap();
    git(&["add", "docs/c.md"]);
    std::fs::write(tmp.join("docs/na.md"), "mine\n").unwrap();
    std::fs::write(
        tmp.join("issues/foo.md"),
        "# Foo\n\n[bar](bar.md)\n\n# Tracking\n",
    )
    .unwrap();
    let (ok, out) = run(script, &tmp, "issues/foo.md");
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["commit", "-q", "-m", "close"])
        .current_dir(&tmp)
        .status()
        .unwrap()
        .success();
    let left = git(&["status", "--porcelain"]);
    let roadmap = git(&["show", "HEAD:ROADMAP.md"]);
    let shown = git(&["show", "--stat", "--format=", "HEAD"]);
    let untracked = tmp.join("issues/baz.md");
    std::fs::write(&untracked, "# Baz\n").unwrap();
    let (plain, plain_out) = run(script, &tmp, "issues/baz.md");
    let _ = std::fs::remove_dir_all(&tmp);
    vec![
        (
            "a bare git commit holds the move and every rewrite of the close",
            ok && commit
                && left == " M docs/b.md\n M docs/c.md\n M docs/na.md\n"
                && shown.contains("docs/a.md")
                && shown.contains("docs/n[ab].md")
                && roadmap.contains("- State: done")
                && roadmap.contains("issues/done/foo.md")
                && shown.contains("issues/done/foo.md")
                && shown.contains("issues/foo.md"),
        ),
        (
            "the close says it is staged and names the file of the user it left out",
            out.contains(
                "staged: commit the close with `git commit` (no `git add` of issues/foo.md: git mv staged its removal)"
            ) && out.contains("not staged: docs/b.md (it held changes of yours)")
                && out.contains("not staged: docs/c.md (it held changes of yours)")
                && out.contains(
                    "staged before the close: docs/c.md (a bare `git commit` takes it too)"
                )
                && !out.contains("docs/na.md"),
        ),
        (
            "an untracked issue stages nothing and says nothing of it",
            plain && !plain_out.contains("staged"),
        ),
    ]
}

// spec: CLS-1
#[test]
fn close_issue() {
    let script = std::path::PathBuf::from("/tools/close_issue.rs");
    let tmp = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&tmp)
            .status()
            .unwrap()
            .success());
    };
    git(&["init", "-q"]);
    for f in ["tracked.md", "untracked.md"] {
        std::fs::write(tmp.join("issues").join(f), "x").unwrap();
    }
    git(&["add", "issues/tracked.md"]);
    std::fs::write(tmp.join("PLAN.md"), "see issues/tracked.md and my_issues/tracked.md, docs/issues/tracked.md, issues/tracked.md.bak2 (`issues/tracked.md`) issues/tracked_two.md issues/done/untracked.md").unwrap();

    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (ok, _) = run(&script, &tmp, "issues/tracked.md");
    let index = |path: &str| {
        let out = Command::new("git")
            .args(["ls-files", "--", path])
            .current_dir(&tmp)
            .output()
            .unwrap();
        !out.stdout.is_empty()
    };
    let staged = index("issues/done/tracked.md") && !index("issues/tracked.md");
    check(
        "tracked issue is moved with git mv",
        ok && staged
            && tmp.join("issues/done/tracked.md").is_file()
            && !tmp.join("issues/tracked.md").exists(),
    );
    let plan = std::fs::read_to_string(tmp.join("PLAN.md")).unwrap();
    check("references are pointed at done/", plan == "see issues/done/tracked.md and my_issues/tracked.md, docs/issues/tracked.md, issues/done/tracked.md.bak2 (`issues/done/tracked.md`) issues/tracked_two.md issues/done/untracked.md");
    let (ok, _) = run(&script, &tmp, "issues/untracked.md");
    let outside_index = !index("issues/done/untracked.md");
    check(
        "untracked issue is moved and left out of the index",
        ok && outside_index && tmp.join("issues/done/untracked.md").is_file(),
    );
    for (name, ok) in relink_checks(&script, &tmp) {
        check(name, ok);
    }
    std::fs::remove_dir_all(tmp.join("node_modules")).unwrap();
    std::fs::create_dir_all(tmp.join("elsewhere")).unwrap();
    std::fs::write(tmp.join("elsewhere/x.md"), "x").unwrap();
    let (ok, out) = run(&script, &tmp, "elsewhere/x.md");
    check(
        "file outside issues/ is refused",
        !ok && out.contains("not an issue"),
    );
    let (ok, _) = run(&script, &tmp, "issues/missing.md");
    check("missing file is refused", !ok);
    std::fs::write(tmp.join("issues/tracked.md"), "y").unwrap();
    let (ok, out) = run(&script, &tmp, "issues/tracked.md");
    check(
        "existing target is not overwritten",
        !ok && out.contains("already exists"),
    );
    std::fs::write(tmp.join("issues/absolute.md"), "x").unwrap();
    git(&["add", "issues/absolute.md"]);
    let outside = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let (ok, out) = run(
        &script,
        &outside,
        tmp.join("issues/absolute.md").to_str().unwrap(),
    );
    let staged = index("issues/done/absolute.md") && !index("issues/absolute.md");
    check(
        "an absolute path from outside the repo is moved with git mv",
        ok && staged && out.contains("git mv"),
    );

    for (name, ok) in closes_only_in_its_own_worktree(&script)
        .into_iter()
        .chain(skips_the_nested_worktrees(&script))
        .chain(relinks_the_markdown_links(&script))
        .chain(prints_the_next_issue(&script))
        .chain(sets_the_milestone_done(&script))
        .chain(stages_the_close(&script))
    {
        check(name, ok);
    }

    let _ = std::fs::remove_dir_all(&tmp);
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
