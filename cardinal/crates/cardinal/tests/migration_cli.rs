//! The migrations a source declares (SYN-52, SYN-53), run as the binary on temp dirs: a project whose lock records a
//! format below the one of its source applies them.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, snapshot, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

/// The rewrite of the seeded justfile line `check: check-old` into `check: check-new`.
const JUSTFILE_REWRITE: &str = r#"
[[rewrite]]
path = "justfile"
old = "check: check-old"
new = "check: check-new"
"#;

/// Make the lock of `root` record the format `format`, or none.
fn set_lock_format(root: &Path, format: Option<u32>) {
    let lock = get(root, LOCK).unwrap();
    let mut text: String = lock
        .lines()
        .filter(|l| !l.starts_with("format "))
        .flat_map(|l| [l, "\n"])
        .collect();
    if let Some(format) = format {
        text = format!("format {format}\n{text}");
    }
    put(root, LOCK, &text);
}

/// A project synced once, whose lock is then set back to the format 2, with a justfile of its own.
fn synced_at_format_2(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    set_lock_format(root.path(), Some(2));
    put(
        root.path(),
        "justfile",
        "check: check-old\nfmt: check: check-old\n",
    );
    (source, root)
}

// spec: SYN-52, SYN-53
#[test]
fn sync_rewrites_the_seeded_lines_of_a_migration_above_the_lock_once() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    put(
        root.path(),
        "justfile",
        "check: check-old\ncheck: check-old fmt\nx check: check-old\ncheck: check-old",
    );

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("justfile"), Some("rewritten"), "{run:?}");
    assert_eq!(
        get(root.path(), "justfile").unwrap(),
        "check: check-new\ncheck: check-old fmt\nx check: check-old\ncheck: check-new"
    );
    assert!(get(root.path(), LOCK)
        .unwrap()
        .contains(&format!("\nformat {FORMAT}")));

    // the lock records the format of the source now: the migration is behind the project
    put(root.path(), "justfile", "check: check-old\n");
    let again = home.ok(root.path(), &["sync"]);
    assert_eq!(again.outcome("justfile"), None, "{again:?}");
    assert_eq!(get(root.path(), "justfile").unwrap(), "check: check-old\n");
}

// spec: SYN-52, SYN-53
#[test]
fn the_migrations_apply_in_increasing_format_from_a_lock_without_a_format() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    set_lock_format(root.path(), None);
    put(root.path(), "justfile", "one\n");
    let rewrite = |old: &str, new: &str| {
        format!("[[rewrite]]\npath = \"justfile\"\nold = \"{old}\"\nnew = \"{new}\"\n")
    };
    put(source.path(), "migrations/2.toml", &rewrite("one", "two"));
    put(source.path(), "migrations/3.toml", &rewrite("two", "three"));
    // above the format of the source, or below the one of the lock: not applied
    put(
        source.path(),
        "migrations/99.toml",
        &rewrite("three", "four"),
    );
    put(source.path(), "migrations/1.toml", &rewrite("one", "zero"));

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        run.lines()
            .iter()
            .filter(|l| **l == "rewritten: justfile")
            .count(),
        1,
        "{run:?}"
    );
    assert_eq!(get(root.path(), "justfile").unwrap(), "three\n");
}

// spec: SYN-52
#[test]
fn a_project_without_a_lock_applies_no_migration() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    put(root.path(), "justfile", "check: check-old\n");

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("justfile"), None, "{run:?}");
    assert_eq!(get(root.path(), "justfile").unwrap(), "check: check-old\n");
}

// spec: SYN-53
#[test]
fn sync_dry_run_reports_the_rewrites_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    let before = snapshot(root.path());

    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(run.outcome("justfile"), Some("rewritten"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-53
#[test]
fn a_rewrite_of_a_managed_file_the_lock_a_path_outside_or_a_missing_file_is_left_alone() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    let rewrite =
        |path: &str| format!("[[rewrite]]\npath = \"{path}\"\nold = \"a1\"\nnew = \"x\"\n");
    let text: String = ["a.txt", LOCK, "../out.txt", "missing.txt"]
        .iter()
        .map(|p| rewrite(p))
        .collect();
    put(source.path(), "migrations/3.toml", &text);
    let outside = root.path().parent().unwrap().join("out.txt");
    let had_outside = outside.exists();

    let run = home.ok(root.path(), &["sync"]);
    assert!(!run.stdout.contains("rewritten"), "{run:?}");
    assert_eq!(get(root.path(), "a.txt").unwrap(), "a1");
    assert!(!root.path().join("missing.txt").exists());
    assert_eq!(outside.exists(), had_outside);
}

// spec: SYN-52
#[test]
fn a_migration_of_the_wrong_shape_fails_the_sync_naming_it_and_writes_nothing() {
    for text in [
        "names = \"old\"\n",
        "[[names]]\nold = \"x\"\n",
        "[[rewrite]]\npath = \"justfile\"\nold = \"\"\nnew = \"y\"\n",
        "[[rewrites]]\npath = \"justfile\"\nold = \"x\"\nnew = \"y\"\n",
        "not toml = [\n",
    ] {
        let home = Home::new();
        let (source, root) = synced_at_format_2(&home);
        put(source.path(), "files/a.txt", "a2");
        put(source.path(), "migrations/3.toml", text);
        let before = snapshot(root.path());
        for args in [&["sync"][..], &["sync", "--dry-run"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(3), "{text}: {run:?}");
            assert!(run.stderr.contains("migrations/3.toml"), "{text}: {run:?}");
            assert_eq!(snapshot(root.path()), before, "{text}");
        }
    }
}

/// A git source of the format 3 with `files/a.txt`, whose head declares `migration` as `migrations/3.toml`.
fn git_source(home: &Home, migration: &str) -> TempDir {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/a.txt", "a1");
    put(repo.path(), "migrations/3.toml", migration);
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    repo
}

/// A project of `repo` updated once, whose lock is then set back to the format 2.
fn updated_at_format_2(home: &Home, repo: &Path) -> TempDir {
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "# old header\n[cardinal]\nsource = \"file://{}\"\n",
            repo.display()
        ),
    );
    home.ok(root.path(), &["update"]);
    set_lock_format(root.path(), Some(2));
    root
}

// spec: SYN-53
#[test]
fn update_rewrites_the_configuration_and_keeps_the_rev_it_writes() {
    let home = Home::new();
    let header = "[[rewrite]]\npath = \".claude/cardinal.toml\"\nold = \"# old header\"\nnew = \"# new header\"\n";
    let repo = git_source(&home, header);
    let root = updated_at_format_2(&home, repo.path());

    let run = home.ok(root.path(), &["update"]);
    assert_eq!(run.outcome(CONFIG), Some("rewritten"), "{run:?}");
    let config = get(root.path(), CONFIG).unwrap();
    assert!(config.starts_with("# new header\n[cardinal]\n"), "{config}");
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    assert!(config.contains(&format!("rev = \"{head}\"")), "{config}");
}

// spec: SYN-53
#[test]
fn update_stopped_by_a_conflict_rewrites_nothing() {
    let home = Home::new();
    let repo = git_source(&home, JUSTFILE_REWRITE);
    let root = updated_at_format_2(&home, repo.path());
    put(root.path(), "justfile", "check: check-old\n");
    put(root.path(), "a.txt", "mine");
    put(repo.path(), "files/a.txt", "a2");
    home.git(repo.path(), &["commit", "--quiet", "-am", "two"]);
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

/// The name `check-old` replaced by `check-new` in the format 3.
const NAME: &str = "[[names]]\nold = \"check-old\"\nnew = \"check-new\"\n";

/// The warning of a line of `path` citing `check-old`.
fn cites(path_line: &str) -> String {
    format!("warning: {path_line} cites check-old, named check-new since the format 3")
}

/// The warnings of stderr that report a cited old name.
fn leftovers(run: &common::Run) -> Vec<&str> {
    run.stderr
        .lines()
        .filter(|l| l.contains(" cites "))
        .collect()
}

/// `synced_at_format_2`, made a git repository that tracks the project files `files` and declares `NAME` and the
/// justfile rewrite.
fn tracked(home: &Home, files: &[(&str, &[u8])]) -> (TempDir, TempDir) {
    let (source, root) = synced_at_format_2(home);
    put(
        source.path(),
        "migrations/3.toml",
        &format!("{NAME}{JUSTFILE_REWRITE}"),
    );
    for (rel, bytes) in files {
        let path = root.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.git(root.path(), &["add", "."]);
    (source, root)
}

// spec: SYN-54
#[test]
fn sync_warns_of_each_line_of_a_tracked_project_file_citing_an_old_name() {
    let home = Home::new();
    let (source, root) = tracked(
        &home,
        &[
            (
                "README.md",
                b"run check-old\nnothing\ncheck-old and check-old\n",
            ),
            ("docs/x.md", b"just check-old\n"),
            ("issues/done/old.md", b"check-old\n"),
            ("docs/adr/0001-x.md", b"check-old\n"),
            ("bin.dat", b"\xff check-old\n"),
        ],
    );
    put(root.path(), "untracked.md", "check-old\n");
    // a managed file citing it is the source's to fix
    put(source.path(), "files/a.txt", "check-old\n");
    let readme = get(root.path(), "README.md").unwrap();

    let run = home.ok(root.path(), &["sync"]);
    // the justfile was rewritten first: its first line no longer cites the name, its second still does
    assert_eq!(
        leftovers(&run),
        [
            cites("README.md:1"),
            cites("README.md:3"),
            cites("docs/x.md:1"),
            cites("justfile:2")
        ],
        "{run:?}"
    );
    assert_eq!(get(root.path(), "README.md").unwrap(), readme);
}

// spec: SYN-54
#[test]
fn sync_dry_run_warns_of_the_lines_as_the_run_would_leave_them() {
    let home = Home::new();
    let (_source, root) = tracked(&home, &[("README.md", b"check-old\n")]);
    let before = snapshot(root.path());

    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(
        leftovers(&run),
        [cites("README.md:1"), cites("justfile:2")],
        "{run:?}"
    );
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-54
#[test]
fn no_old_name_is_reported_outside_a_git_repository_or_once_the_migration_applied() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", NAME);
    put(root.path(), "README.md", "check-old\n");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.stderr, "", "{run:?}");

    let (_source, root) = tracked(&home, &[("README.md", b"check-old\n")]);
    home.ok(root.path(), &["sync"]);
    let again = home.ok(root.path(), &["sync"]);
    assert!(leftovers(&again).is_empty(), "{again:?}");
}

// spec: SYN-53
#[test]
fn a_seeded_line_ending_in_crlf_is_rewritten() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    put(root.path(), "justfile", "check: check-old\r\nfmt:\r\n");

    home.ok(root.path(), &["sync"]);
    assert_eq!(
        get(root.path(), "justfile").unwrap(),
        "check: check-new\r\nfmt:\r\n"
    );
}

// spec: SYN-54
#[test]
fn an_old_name_is_cited_as_a_word_and_named_by_the_last_migration_that_renamed_it() {
    let home = Home::new();
    let (source, root) = tracked(
        &home,
        &[(
            "README.md",
            b"x-old\nprecheck-older\ncheck-old_x\n(check-old)\n",
        )],
    );
    set_lock_format(root.path(), None);
    put(
        source.path(),
        "migrations/2.toml",
        "[[names]]\nold = \"x-old\"\nnew = \"check-old\"\n",
    );
    put(source.path(), "migrations/3.toml", NAME);

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        leftovers(&run),
        [
            "warning: README.md:1 cites x-old, named check-new since the format 3".to_string(),
            cites("README.md:4"),
            cites("justfile:1"),
            cites("justfile:2"),
        ],
        "{run:?}"
    );
}

// spec: SYN-53, SYN-54
#[test]
fn sync_dry_run_judges_neither_a_removed_managed_file_nor_the_settings_it_merges() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    let hook = |command: &str| {
        format!(
            r#"{{"hooks": {{"Stop": [{{"hooks": [{{"type": "command", "command": "{command}"}}]}}]}}}}"#
        )
    };
    put(
        source.path(),
        "entries/.claude/settings.json",
        &hook("run check-old"),
    );
    put(source.path(), "seed/.claude/settings.json", "{}");
    put(source.path(), "files/old.txt", "check-old\n");
    // synced at the format 3, then set back: the settings run the old hook, the old managed file is there
    home.ok(root.path(), &["sync"]);
    set_lock_format(root.path(), Some(2));
    std::fs::remove_file(source.path().join("files/old.txt")).unwrap();
    put(
        source.path(),
        "entries/.claude/settings.json",
        &hook("run check-new"),
    );
    let rewrite_old = "[[rewrite]]\npath = \"old.txt\"\nold = \"check-old\"\nnew = \"x\"\n";
    put(
        source.path(),
        "migrations/3.toml",
        &format!("{NAME}{rewrite_old}"),
    );
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.git(root.path(), &["add", "."]);
    let before = snapshot(root.path());

    let dry = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(snapshot(root.path()), before);
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("old.txt"), Some("removed"), "{run:?}");
    for run in [&dry, &run] {
        assert_eq!(
            leftovers(run),
            [cites("justfile:1"), cites("justfile:2")],
            "{run:?}"
        );
        assert!(!run.stdout.contains("rewritten: old.txt"), "{run:?}");
    }
}
