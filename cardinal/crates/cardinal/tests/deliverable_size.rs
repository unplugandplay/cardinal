//! Behavior test for `cardinal tool deliverable-size`: the changed lines of a deliverable or an issue against the size
//! limit of `issue-workflow`.

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

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", message, "--no-verify"]);
}

/// A repository whose first commit holds `files`.
fn repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main", "."]);
    write(dir.path(), "README.md", "readme\n");
    for (path, body) in files {
        write(dir.path(), path, body);
    }
    commit(dir.path(), "first");
    dir
}

/// The exit code and stdout of the tool run in `dir` with `args`.
fn size(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "deliverable-size"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The total of the last line of `stdout`.
fn total(stdout: &str) -> u64 {
    let last = stdout.lines().last().unwrap_or_default();
    last.split(' ').next().unwrap().parse().unwrap()
}

fn lines(n: usize) -> String {
    (1..=n)
        .flat_map(|i| ["line ".to_string(), i.to_string(), "\n".into()])
        .collect()
}

// spec: SIZ-1
#[test]
fn counts_the_working_tree_and_the_untracked_files_against_head() {
    let dir = repo(&[("a.rs", &lines(4))]);
    write(dir.path(), "a.rs", "line 1\nx\ny\nline 4\n");
    git(dir.path(), &["add", "a.rs"]);
    write(dir.path(), "new.rs", &lines(3));
    let (code, stdout) = size(dir.path(), &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(total(&stdout), 7, "{stdout}");
}

// spec: SIZ-1
#[test]
fn a_repository_with_no_commit_counts_against_the_empty_tree() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main", "."]);
    write(dir.path(), "new.rs", &lines(3));
    let (code, stdout) = size(dir.path(), &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(total(&stdout), 3, "{stdout}");
}

// spec: SIZ-2, SIZ-3
#[test]
fn a_revision_adds_the_working_tree_a_range_does_not() {
    let dir = repo(&[]);
    git(dir.path(), &["tag", "A"]);
    write(dir.path(), "b.rs", &lines(4));
    commit(dir.path(), "B");
    write(dir.path(), "new.rs", &lines(1));
    assert_eq!(total(&size(dir.path(), &["A"]).1), 5);
    assert_eq!(total(&size(dir.path(), &["A..HEAD"]).1), 4);
}

// spec: SIZ-4
#[test]
fn leaves_out_the_issues_and_the_lock_files() {
    let dir = repo(&[]);
    for path in [
        "issues/a.md",
        "Cargo.lock",
        "web/package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "go.sum",
        "src/a.rs",
    ] {
        write(dir.path(), path, "one\n");
    }
    let (_, stdout) = size(dir.path(), &[]);
    assert_eq!(stdout, "1\tsrc/a.rs\n1 changed lines, limit 1000: within\n");
}

// spec: SIZ-5
#[test]
fn a_pure_rename_counts_nothing() {
    let dir = repo(&[("old.rs", &lines(10))]);
    git(dir.path(), &["mv", "old.rs", "new.rs"]);
    assert_eq!(total(&size(dir.path(), &[]).1), 0);
}

/// The `git status` of `dir`, staged and unstaged.
fn status(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// spec: SIZ-5, SIZ-11
#[test]
fn a_plain_move_counts_nothing_and_leaves_the_index_alone() {
    let dir = repo(&[("old.rs", &lines(10))]);
    std::fs::rename(dir.path().join("old.rs"), dir.path().join("new.rs")).unwrap();
    let before = status(dir.path());
    let (_, stdout) = size(dir.path(), &[]);
    assert_eq!(total(&stdout), 0, "{stdout}");
    assert_eq!(status(dir.path()), before);
}

// spec: SIZ-6
#[test]
fn a_rename_edited_later_in_the_range_counts_twice() {
    let dir = repo(&[("old.rs", &lines(10))]);
    git(dir.path(), &["tag", "A"]);
    git(dir.path(), &["mv", "old.rs", "new.rs"]);
    commit(dir.path(), "move");
    write(
        dir.path(),
        "new.rs",
        &lines(10).replace("line 5\n", "five\n"),
    );
    commit(dir.path(), "edit");
    assert_eq!(total(&size(dir.path(), &["A..HEAD"]).1), 20);
}

// spec: SIZ-7
#[test]
fn a_binary_file_counts_nothing() {
    let dir = repo(&[("tracked.bin", "a\0b\n")]);
    write(dir.path(), "tracked.bin", "c\0d\ne\n");
    write(dir.path(), "new.bin", "x\0y\nz\n");
    assert_eq!(total(&size(dir.path(), &[]).1), 0);
}

// spec: SIZ-7
#[test]
fn a_file_git_reads_as_binary_by_its_attributes_counts_nothing() {
    let dir = repo(&[(".gitattributes", "*.gen -diff\n")]);
    write(dir.path(), "out.gen", &lines(5));
    assert_eq!(total(&size(dir.path(), &[]).1), 0);
}

// spec: SIZ-8
#[test]
fn the_limits_come_from_the_configuration() {
    let dir = repo(&[(
        ".claude/cardinal.toml",
        "[project]\ndeliverable_size_limit = 5\nissue_size_limit = 7\n",
    )]);
    write(dir.path(), "a.rs", &lines(6));
    let (code, stdout) = size(dir.path(), &[]);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.ends_with("6 changed lines, limit 5: over\n"),
        "{stdout}"
    );
    let (code, stdout) = size(dir.path(), &["HEAD"]);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.ends_with("limit 7: within\n"), "{stdout}");
}

// spec: SIZ-8
#[test]
fn the_issue_limit_defaults_to_3000() {
    let dir = repo(&[]);
    let (code, stdout) = size(dir.path(), &["HEAD"]);
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "0 changed lines, limit 3000: within\n");
}

// spec: SIZ-9
#[test]
fn lists_the_files_largest_first_then_by_path() {
    let dir = repo(&[]);
    write(dir.path(), "b.rs", &lines(3));
    write(dir.path(), "a.rs", &lines(3));
    write(dir.path(), "c.rs", &lines(5));
    let (code, stdout) = size(dir.path(), &[]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "5\tc.rs\n3\ta.rs\n3\tb.rs\n11 changed lines, limit 1000: within\n"
    );
}

// spec: SIZ-10
#[test]
fn a_git_failure_or_a_bad_limit_exits_3_with_no_count() {
    let dir = repo(&[(
        ".claude/cardinal.toml",
        "[project]\nissue_size_limit = \"big\"\n",
    )]);
    assert_eq!(size(dir.path(), &["HEAD"]), (Some(3), String::new()));
    let dir = repo(&[(
        ".claude/cardinal.toml",
        "[project]\ndeliverable_size_limit = 0\n",
    )]);
    assert_eq!(size(dir.path(), &[]), (Some(3), String::new()));
    let dir = repo(&[]);
    assert_eq!(size(dir.path(), &["nope"]), (Some(3), String::new()));
    assert_eq!(size(dir.path(), &["--output=x"]), (Some(3), String::new()));
    assert!(!dir.path().join("x").exists());
    let dir = repo(&[(".claude/cardinal.toml/x", "")]);
    assert_eq!(size(dir.path(), &[]), (Some(3), String::new()));
    let outside = tempfile::tempdir().unwrap();
    assert_eq!(size(outside.path(), &[]), (Some(3), String::new()));
}

// spec: FEA-37
#[test]
fn counts_the_issues_with_the_issue_files_off() {
    let dir = repo(&[(
        ".claude/cardinal.toml",
        "[cardinal]\ndisabled = [\"issue-files\"]\n",
    )]);
    for path in ["issues/a.md", "Cargo.lock", "src/a.rs"] {
        write(dir.path(), path, "one\n");
    }
    let (_, stdout) = size(dir.path(), &[]);
    assert_eq!(
        stdout,
        "1\tissues/a.md\n1\tsrc/a.rs\n2 changed lines, limit 1000: within\n"
    );
}
