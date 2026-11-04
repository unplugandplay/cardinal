//! The format of the managed content: `sync` and `update` refuse a source of a format the running `cardinal` cannot
//! read (SYN-40), run as the binary on temp dirs.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, snapshot, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

/// The format the `cardinal` under test reads: the one of its own revision (SYN-43).
fn format() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cardinal-format"))
        .unwrap()
        .trim()
        .to_string()
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

// spec: SYN-40
#[test]
fn sync_refuses_a_source_of_a_newer_format_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(source.path(), "files/a.txt", "a2");
    put(source.path(), "cardinal-format", "99\n");
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        let install = format!(
            "cargo install --force --path {}/crates/cardinal --locked",
            source.path().display()
        );
        assert!(
            run.stderr.contains(&format!(
                "needs a cardinal that reads the format 99, this one reads up to {}: install it with `{install}`",
                format()
            )),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-40
#[test]
fn sync_reads_a_source_of_a_format_it_reads() {
    let home = Home::new();
    let (source, root) = project(&home);
    for declared in [FORMAT.trim().to_string(), format()] {
        put(source.path(), "cardinal-format", &format!("{declared}\n"));
        put(source.path(), "files/a.txt", &declared);
        home.ok(root.path(), &["sync"]);
        assert_eq!(get(root.path(), "a.txt"), Some(declared));
    }
}

// spec: SYN-40
#[test]
fn a_source_format_that_is_not_a_whole_number_is_an_error_naming_the_file() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "two\n");
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("cardinal-format"), "{run:?}");
    assert!(get(root.path(), LOCK).is_none());
}

// spec: SYN-40, BIN-20
#[test]
fn update_refuses_a_revision_of_a_newer_format_and_keeps_the_project_as_it_was() {
    let home = Home::new();
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/a.txt", "a1");
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    home.git(repo.path(), &["tag", "v1"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{}\"\n", url(repo.path())),
    );
    home.ok(root.path(), &["update", "--rev", "v1"]);
    put(repo.path(), "files/a.txt", "a2");
    put(repo.path(), "cardinal-format", "99\n");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "newer format"]);
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let before = snapshot(root.path());

    for args in [&["update"][..], &["update", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        let install = format!("cardinal upgrade {} {head}", url(repo.path()));
        assert!(
            run.stderr.contains(&format!(
                "needs a cardinal that reads the format 99, this one reads up to {}: install it with `{install}`",
                format()
            )),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

/// A git source with one commit declaring the format `declared`, a project on it, and the commit.
fn git_project(home: &Home, declared: &str) -> (TempDir, TempDir, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/a.txt", "a1");
    put(repo.path(), SOURCE_FORMAT, declared);
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let commit = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{}\"\n", url(repo.path())),
    );
    (repo, root, commit)
}

// spec: SYN-40, BIN-20
#[test]
fn sync_of_an_unpinned_git_source_names_the_commit_it_checked_out() {
    let home = Home::new();
    let (repo, root, commit) = git_project(&home, "99\n");
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    let install = format!("cardinal upgrade {} {commit}", url(repo.path()));
    assert!(
        run.stderr.contains(&format!("install it with `{install}`")),
        "{run:?}"
    );
    assert!(get(root.path(), LOCK).is_none());
}

/// The `format` lines of the lock of `root`.
fn format_lines(root: &Path) -> Vec<String> {
    get(root, LOCK)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("format "))
        .map(String::from)
        .collect()
}

// spec: SYN-41
#[test]
fn the_lock_records_the_format_of_its_source_before_every_other_line() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    assert_eq!(
        format_lines(root.path()),
        [format!("format {}", FORMAT.trim())]
    );
    put(source.path(), "cardinal-format", &format!("{}\n", format()));
    home.ok(root.path(), &["sync"]);
    assert_eq!(format_lines(root.path()), [format!("format {}", format())]);
    let lock = get(root.path(), LOCK).unwrap();
    let first = lock.lines().find(|l| !l.starts_with('#')).unwrap();
    assert_eq!(first, format!("format {}", format()), "{lock}");
}

// spec: SYN-41
#[test]
fn a_lock_without_a_format_is_read_and_rewritten_with_one() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK).unwrap();
    let older: String = lock
        .lines()
        .filter(|l| !l.starts_with("format "))
        .map(|l| l.to_string() + "\n")
        .collect();
    put(root.path(), LOCK, &older);
    home.ok(root.path(), &["check"]);
    let run = home.ok(root.path(), &["sync"]);
    assert!(run.lines().is_empty(), "{run:?}");
    assert_eq!(get(root.path(), LOCK), Some(lock));
}

// spec: SYN-42
#[test]
fn check_sync_and_update_refuse_a_lock_of_a_newer_format_and_write_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK).unwrap();
    // a newer format may write lines this one cannot parse, before its format line
    let newer = format!(
        "{}entry  {{\"ask\": \"Bash(x)\"}}\n../outside  h\nformat 99\n",
        lock.replace(&format!("format {FORMAT}"), "")
    );
    put(root.path(), LOCK, &newer);
    put(source.path(), "files/a.txt", "a2");
    let before = snapshot(root.path());
    let install = format!(
        "cargo install --force --path {}/crates/cardinal --locked",
        source.path().display()
    );
    let refusal = format!(
        "{LOCK} was synced from a source of the format 99, this one reads up to {}: install it with `{install}`",
        format()
    );
    for args in [&["check"][..], &["sync"], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?} {run:?}");
        assert!(run.stderr.contains(&refusal), "{args:?} {run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-42, BIN-20
#[test]
fn update_refuses_a_lock_of_a_newer_format_naming_the_pinned_revision() {
    let home = Home::new();
    let (repo, root, commit) = git_project(&home, FORMAT);
    home.ok(root.path(), &["update"]);
    let lock = get(root.path(), LOCK).unwrap();
    put(
        root.path(),
        LOCK,
        &lock.replace(&format!("format {}", FORMAT.trim()), "format 99"),
    );
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    let install = format!("cardinal upgrade {} {commit}", url(repo.path()));
    assert!(
        run.stderr.contains(&format!("install it with `{install}`")),
        "{run:?}"
    );
    assert_eq!(snapshot(root.path()), before);
}

/// The refusal of a source below the format 5 (SYN-51).
fn too_old(source: &str, format: u32) -> String {
    format!("{source} has the format {format}, this cardinal syncs the format 5 and higher")
}

// spec: SYN-51
#[test]
fn init_and_sync_refuse_a_source_below_the_format_5_and_write_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(source.path(), "files/a.txt", "a2");
    let name = source.path().to_str().unwrap();
    for (declared, format) in [(Some("4\n"), 4), (None, 1)] {
        match declared {
            Some(text) => put(source.path(), SOURCE_FORMAT, text),
            None => std::fs::remove_file(source.path().join(SOURCE_FORMAT)).unwrap(),
        }
        let before = snapshot(root.path());
        for args in [&["sync"][..], &["sync", "--dry-run"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(3), "{run:?}");
            assert!(run.stderr.contains(&too_old(name, format)), "{run:?}");
            assert_eq!(snapshot(root.path()), before);
        }
        let fresh = tempdir().unwrap();
        let run = home.harness(fresh.path(), &["init", name]);
        assert_eq!(run.code, Some(3), "{run:?}");
        assert!(run.stderr.contains(&too_old(name, format)), "{run:?}");
        assert!(snapshot(fresh.path()).is_empty(), "{run:?}");
    }
}

// spec: SYN-51
#[test]
fn update_to_a_revision_below_the_format_5_keeps_the_project_as_it_was() {
    let home = Home::new();
    let (repo, root, _) = git_project(&home, FORMAT);
    home.ok(root.path(), &["update"]);
    put(repo.path(), "files/a.txt", "a2");
    put(repo.path(), SOURCE_FORMAT, "4\n");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "older format"]);
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let before = snapshot(root.path());
    for args in [&["update"][..], &["update", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        let source = format!("{} at {head}", url(repo.path()));
        assert!(run.stderr.contains(&too_old(&source, 4)), "{run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-10
#[test]
fn a_source_path_that_holds_nothing_is_reported_as_such_not_as_an_old_format() {
    let home = Home::new();
    let (source, root) = project(&home);
    let gone = source.path().join("gone");
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{}\"\n", gone.display()),
    );
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("has no files/ directory"), "{run:?}");
}

/// Make the lock of `root` record the format 2.
fn lock_at_format_2(root: &Path) {
    let lock = get(root, LOCK).unwrap();
    put(
        root,
        LOCK,
        &lock.replace(&format!("format {FORMAT}"), "format 2\n"),
    );
}

/// The hint of a lock of the format 2 printed by `check` and `sync`.
fn hint() -> String {
    format!(
        "hint: {LOCK} is of the format 2, this cardinal reads the format {}: run `cardinal update` to move the project to a newer revision",
        format()
    )
}

// spec: SYN-55
#[test]
fn check_and_sync_hint_at_update_while_the_lock_is_below_the_format_of_the_cardinal() {
    let home = Home::new();
    let (_repo, root, _commit) = git_project(&home, FORMAT);
    home.ok(root.path(), &["update"]);
    lock_at_format_2(root.path());
    for args in [&["check"][..], &["sync", "--dry-run"]] {
        let run = home.ok(root.path(), args);
        assert_eq!(run.stderr.matches(&hint()).count(), 1, "{args:?} {run:?}");
    }
    // the sync moves the lock to the format of its source, the one this cardinal reads
    for args in [&["sync"][..], &["check"]] {
        let run = home.ok(root.path(), args);
        assert!(!run.stderr.contains("hint:"), "{args:?} {run:?}");
    }
}

// spec: SYN-55
#[test]
fn update_does_not_hint_at_itself() {
    let home = Home::new();
    let (_repo, root, _commit) = git_project(&home, FORMAT);
    home.ok(root.path(), &["update"]);
    lock_at_format_2(root.path());
    let run = home.ok(root.path(), &["update", "--dry-run"]);
    assert!(!run.stderr.contains("hint:"), "{run:?}");
    assert!(home.ok(root.path(), &["check"]).stderr.contains(&hint()));
}

// spec: SYN-55
#[test]
fn a_local_directory_source_gets_no_hint_since_it_has_no_revision_to_update() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    lock_at_format_2(root.path());
    for args in [&["check"][..], &["sync", "--dry-run"]] {
        let run = home.ok(root.path(), args);
        assert!(!run.stderr.contains("hint:"), "{args:?} {run:?}");
    }
}
