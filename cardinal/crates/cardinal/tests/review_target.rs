//! Behavior test for `cardinal tool review-target`: the target of a code review, without the issue files and the
//! managed copies.

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

fn write(dir: &Path, path: &str, body: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, body).unwrap();
}

/// A repository with one commit, holding `files`.
fn repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main", "."]);
    write(dir.path(), "README.md", "readme\n");
    for (path, body) in files {
        write(dir.path(), path, body);
    }
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-qm", "first", "--no-verify"]);
    dir
}

const LOCK: &str = "# Written by `cardinal sync`. Do not edit.
format 8
aaaa  AGENTS.md
bbbb+x  .claude/rules/a.md
entry  {\"allow\":\"Bash(cardinal run *)\"}
";

/// The exit code, stdout and stderr of the tool run in `dir` with `args`.
fn target(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "review-target"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The excluding pathspec of `path`, quoted.
fn ex(path: &str) -> String {
    format!("':(top,exclude,literal){path}'")
}

/// The paths `git diff --name-only <line>` names, run by a shell in `dir`, as the reviewer runs it.
fn diffed(dir: &Path, line: &str) -> String {
    let out = Command::new("sh")
        .args(["-c", &format!("git diff --name-only {line}")])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// spec: RVT-1, RVT-3
#[test]
fn prints_the_base_without_the_issues_the_lock_and_the_managed_files() {
    let dir = repo(&[(".claude/cardinal.lock", LOCK)]);
    let (code, stdout, _) = target(dir.path(), &["HEAD"]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        format!(
            "'HEAD' -- ':/' {} {} {} {}\n",
            ex("issues"),
            ex(".claude/cardinal.lock"),
            ex(".claude/rules/a.md"),
            ex("AGENTS.md")
        )
    );
}

// spec: RVT-2
#[test]
fn prints_the_working_tree_against_head_with_no_argument() {
    let dir = repo(&[]);
    let (code, stdout, _) = target(dir.path(), &[]);
    assert_eq!(code, Some(0));
    assert_eq!(stdout, format!("'HEAD' -- ':/' {}\n", ex("issues")));
}

// spec: RVT-1, RVT-3
#[test]
fn the_line_of_a_base_diffs_the_commits_and_the_working_tree_without_the_excluded_paths() {
    let dir = repo(&[(".claude/cardinal.lock", LOCK)]);
    let base = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    write(dir.path(), "AGENTS.md", "managed\n");
    write(
        dir.path(),
        ".claude/cardinal.lock",
        &format!("{LOCK}cccc  b.md\n"),
    );
    write(dir.path(), "src/a.rs", "committed\n");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-qm", "second", "--no-verify"]);
    write(dir.path(), "issues/x.md", "tracking\n");
    write(dir.path(), "src/b.rs", "staged\n");
    git(dir.path(), &["add", "-A"]);
    let sub = dir.path().join("src");
    let (code, stdout, _) = target(&sub, &[base.trim()]);
    assert_eq!(code, Some(0));
    // from a subdirectory too
    assert_eq!(diffed(&sub, stdout.trim_end()), "src/a.rs\nsrc/b.rs\n");
}

// spec: RVT-3
#[test]
fn a_managed_path_excludes_itself_alone() {
    let lock = "aaaa  docs/[d].md\nbbbb  *.sh\n";
    let dir = repo(&[(".claude/cardinal.lock", lock)]);
    write(dir.path(), "docs/d.md", "own\n");
    write(dir.path(), "docs/[d].md", "managed\n");
    write(dir.path(), "run.sh", "own\n");
    git(dir.path(), &["add", "-A"]);
    let (_, stdout, _) = target(dir.path(), &[]);
    assert_eq!(diffed(dir.path(), stdout.trim_end()), "docs/d.md\nrun.sh\n");
}

// spec: RVT-5
#[test]
fn quotes_a_path_holding_a_quote() {
    let dir = repo(&[(".claude/cardinal.lock", "aaaa  it's.md\n")]);
    let (_, stdout, _) = target(dir.path(), &[]);
    assert!(
        stdout.ends_with(" ':(top,exclude,literal)it'\\''s.md'\n"),
        "{stdout}"
    );
}

// spec: RVT-5
#[test]
fn quotes_a_base_a_shell_would_expand() {
    let dir = repo(&[]);
    for base in ["b@{0}", "HEAD^{commit}", "HEAD~0"] {
        git(dir.path(), &["branch", "-f", "b"]);
        let (_, stdout, _) = target(dir.path(), &[base]);
        assert!(stdout.starts_with(&format!("'{base}' -- ")), "{stdout}");
    }
}

// spec: RVT-6
#[test]
fn refuses_an_untracked_file_outside_the_exclusions() {
    let dir = repo(&[(".claude/cardinal.lock", "aaaa  AGENTS.md\n")]);
    write(dir.path(), "src/new.rs", "new\n");
    write(dir.path(), "issues/x.md", "tracking\n");
    write(dir.path(), "AGENTS.md", "managed\n");
    write(dir.path(), ".gitignore", "target/\n");
    write(dir.path(), "target/out", "built\n");
    let (code, stdout, stderr) = target(dir.path(), &[]);
    assert_eq!((code, stdout.as_str()), (Some(3), ""));
    assert!(stderr.contains("git add -N"), "{stderr}");
    assert!(stderr.contains("src/new.rs"), "{stderr}");
    assert!(stderr.contains(".gitignore"), "{stderr}");
    for left in ["issues/x.md", "AGENTS.md", "target/out"] {
        assert!(!stderr.contains(left), "{stderr}");
    }
    git(dir.path(), &["add", "-N", "src/new.rs", ".gitignore"]);
    let (code, stdout, _) = target(dir.path(), &[]);
    assert_eq!(code, Some(0));
    assert_eq!(
        diffed(dir.path(), stdout.trim_end()),
        ".gitignore\nsrc/new.rs\n"
    );
}

// spec: FEA-39
#[test]
fn keeps_the_issues_when_the_issue_files_are_off() {
    let dir = repo(&[
        (".claude/cardinal.lock", LOCK),
        (
            ".claude/cardinal.toml",
            "[cardinal]\ndisabled = [\"issue-files\"]\n",
        ),
    ]);
    let (code, stdout, _) = target(dir.path(), &[]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        format!(
            "'HEAD' -- ':/' {} {} {}\n",
            ex(".claude/cardinal.lock"),
            ex(".claude/rules/a.md"),
            ex("AGENTS.md")
        )
    );
}

// spec: RVT-4
#[test]
fn refuses_an_option_an_unknown_revision_two_arguments_and_no_repository() {
    let dir = repo(&[]);
    for args in [&["--stat"][..], &["nope"], &["HEAD", "HEAD"]] {
        let (code, stdout, stderr) = target(dir.path(), args);
        assert_eq!(code, Some(3), "{args:?}");
        assert_eq!(stdout, "", "{args:?}");
        assert!(!stderr.is_empty(), "{args:?}");
    }
    let bare = tempfile::tempdir().unwrap();
    let (code, stdout, _) = target(bare.path(), &[]);
    assert_eq!((code, stdout.as_str()), (Some(3), ""));
}

// spec: RVT-4
#[test]
fn refuses_a_head_with_no_commit() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main", "."]);
    let (code, stdout, _) = target(dir.path(), &[]);
    assert_eq!((code, stdout.as_str()), (Some(3), ""));
}

// spec: RVT-4
#[test]
fn refuses_a_lock_it_cannot_read() {
    let dir = repo(&[(".claude/cardinal.lock", "format 999\n")]);
    let (code, stdout, _) = target(dir.path(), &[]);
    assert_eq!((code, stdout.as_str()), (Some(3), ""));
}
