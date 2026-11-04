//! `init`, `sync` and `check` run as the binary on temp dirs (SYN-1 to SYN-10).
#![cfg(unix)]

mod common;

use std::fs;

use common::{get, project, put, snapshot, source, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

// spec: SYN-1
#[test]
fn sync_copies_every_managed_file_to_the_same_path() {
    let home = Home::new();
    let (_source, root) = project(&home);
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert_eq!(get(root.path(), "sub/b.txt").as_deref(), Some("b1"));
    assert_eq!(run.outcome("a.txt"), Some("added"));
    assert_eq!(run.outcome("sub/b.txt"), Some("added"));
}

// spec: SYN-2
#[test]
fn sync_overwrites_an_untouched_file_when_the_source_changed() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(source.path(), "files/a.txt", "a2");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert_eq!(run.lines(), ["updated: a.txt"]);
}

// spec: SYN-3
#[test]
fn sync_keeps_a_locally_modified_file_and_fails_unless_forced() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), "a.txt", "local");
    put(source.path(), "files/a.txt", "a2");
    for _ in 0..2 {
        // the conflict survives the next run: the lock still holds the last synced hash
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(1), "{run:?}");
        assert_eq!(run.outcome("a.txt"), Some("conflict"));
        assert_eq!(get(root.path(), "a.txt").as_deref(), Some("local"));
    }
    let run = home.ok(root.path(), &["sync", "--force"]);
    assert_eq!(run.outcome("a.txt"), Some("updated"));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
}

// spec: SYN-3
#[test]
fn a_differing_file_that_was_never_synced_is_a_conflict() {
    let home = Home::new();
    let (_source, root) = project(&home);
    put(root.path(), "a.txt", "mine");
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.outcome("a.txt"), Some("conflict"));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("mine"));
}

// spec: SYN-4
#[test]
fn sync_seeds_only_what_is_missing_and_never_touches_it_again() {
    let home = Home::new();
    let (source, root) = project(&home);
    assert_eq!(
        home.ok(root.path(), &["sync"]).outcome("s.txt"),
        Some("seeded")
    );
    put(root.path(), "s.txt", "edited");
    put(source.path(), "seed/s.txt", "seed2");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("s.txt"), None);
    assert_eq!(get(root.path(), "s.txt").as_deref(), Some("edited"));
    let lock = get(root.path(), LOCK).unwrap();
    assert!(!lock.contains("s.txt"), "a seed is not managed: {lock}");
    // so `check` says nothing of it either
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
}

// spec: SYN-4
#[test]
fn a_seed_on_a_managed_path_is_not_a_seed() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "seed/a.txt", "seed");
    let run = home.ok(root.path(), &["sync"]);
    let lines: Vec<&str> = run
        .lines()
        .into_iter()
        .filter(|l| l.ends_with(": a.txt"))
        .collect();
    assert_eq!(lines, ["added: a.txt"]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

// spec: SYN-5
#[test]
fn sync_removes_a_dropped_file_unless_it_was_modified() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), "sub/b.txt", "local");
    fs::remove_file(source.path().join("files/a.txt")).unwrap();
    fs::remove_file(source.path().join("files/sub/b.txt")).unwrap();
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("a.txt"), Some("removed"));
    assert!(get(root.path(), "a.txt").is_none());
    assert_eq!(run.outcome("sub/b.txt"), Some("kept"));
    assert_eq!(get(root.path(), "sub/b.txt").as_deref(), Some("local"));
    // no longer managed: the next sync and `check` say nothing of either
    assert_eq!(home.ok(root.path(), &["sync"]).stdout, "");
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
}

// spec: SYN-6
#[test]
fn a_second_sync_reports_nothing_and_changes_nothing() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let before = snapshot(root.path());
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-6
#[test]
fn an_identical_file_missing_from_the_lock_is_locked_and_not_reported() {
    let home = Home::new();
    let (_source, root) = project(&home);
    put(root.path(), "a.txt", "a1");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("a.txt"), None);
    assert!(get(root.path(), LOCK).unwrap().contains("a.txt"));
}

// spec: SYN-7
#[test]
fn sync_dry_run_reports_and_writes_nothing() {
    let home = Home::new();
    let (_source, root) = project(&home);
    let before = snapshot(root.path());
    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(run.outcome("a.txt"), Some("added"));
    assert_eq!(run.outcome("s.txt"), Some("seeded"));
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-8
#[test]
fn check_lists_the_modified_and_deleted_managed_files_only() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
    put(root.path(), "a.txt", "local");
    fs::remove_file(root.path().join("sub/b.txt")).unwrap();
    put(root.path(), "s.txt", "a seed may change freely");
    put(root.path(), "other.txt", "not managed");
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.lines(), ["modified: a.txt", "missing: sub/b.txt"]);
}

// spec: SYN-8
#[test]
fn check_fails_when_the_project_was_never_synced() {
    let home = Home::new();
    let (_source, root) = project(&home);
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("error:"), "{run:?}");
}

// spec: SYN-9
#[test]
fn init_writes_the_config_with_the_source_defaults_and_refuses_to_overwrite() {
    let home = Home::new();
    let source = source();
    put(
        source.path(),
        "seed/.claude/cardinal.toml",
        "[project]\nname = \"x\"\n",
    );
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    let text = get(root.path(), CONFIG).unwrap();
    let expected = format!("[cardinal]\nsource = \"{}\"\n", source.path().display());
    assert!(text.starts_with(&expected), "{text}");
    assert!(text.contains("[project]\nname = \"x\""), "{text}");

    let run = home.harness(root.path(), &["init", "elsewhere"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert_eq!(get(root.path(), CONFIG), Some(text));
}

// spec: SYN-9
#[test]
fn init_escapes_the_source_and_the_rev_in_the_config() {
    let home = Home::new();
    let source = tempdir().unwrap();
    let odd = source.path().join("we\"ird\\dir");
    fs::create_dir_all(odd.join("files")).unwrap();
    put(&odd, SOURCE_FORMAT, FORMAT);
    let root = tempdir().unwrap();
    home.ok(
        root.path(),
        &["init", odd.to_str().unwrap(), "--rev", "v\"1"],
    );
    let config: toml::Table = get(root.path(), CONFIG).unwrap().parse().unwrap();
    let table = config["cardinal"].as_table().unwrap();
    assert_eq!(table["source"].as_str(), odd.to_str());
    assert_eq!(table["rev"].as_str(), Some("v\"1"));
}

// spec: SYN-10
#[test]
fn a_source_without_files_fails_and_deletes_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    fs::remove_dir_all(source.path().join("files")).unwrap();
    let before = snapshot(root.path());
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("no files/ directory"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-10
#[test]
fn a_lock_path_outside_the_project_is_refused_by_sync_and_check() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let outside = tempdir().unwrap();
    put(outside.path(), "x", "precious");
    let evil = outside.path().join("x");
    // its real hash: were the path let through, `sync` would delete it as a file the source dropped
    let hash: String = Sha256::digest(b"precious")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .concat();
    let lock = get(root.path(), LOCK).unwrap();
    put(
        root.path(),
        LOCK,
        &format!("{lock}{hash}  {}\n", evil.display()),
    );
    for args in [&["sync"][..], &["check"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?}: {run:?}");
    }
    assert_eq!(get(outside.path(), "x").as_deref(), Some("precious"));
}

// spec: SYN-10
#[test]
fn an_unreadable_lock_is_an_error_not_a_first_sync() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    fs::write(root.path().join(LOCK), [0xff, 0xfe, 0x00]).unwrap();
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["check"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?}: {run:?}");
    }
    assert_eq!(snapshot(root.path()), before);
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    use super::common::{executable, project, Home};

    // spec: SYN-1
    #[test]
    fn sync_keeps_the_executable_bit_and_check_reports_a_mode_drift() {
        let home = Home::new();
        let (source, root) = project(&home);
        let script = source.path().join("files/sub/b.txt");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        home.ok(root.path(), &["sync"]);
        let path = root.path().join("sub/b.txt");
        assert!(executable(&path));
        assert!(!executable(&root.path().join("a.txt")));
        assert_eq!(home.ok(root.path(), &["check"]).stdout, "");

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let run = home.harness(root.path(), &["check"]);
        assert_eq!(run.code, Some(1), "{run:?}");
        assert_eq!(run.lines(), ["modified: sub/b.txt"]);
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.outcome("sub/b.txt"), Some("conflict"));
        let run = home.ok(root.path(), &["sync", "--force"]);
        assert_eq!(run.outcome("sub/b.txt"), Some("updated"));
        assert!(executable(&path));
    }

    // spec: SYN-4
    #[test]
    fn a_dangling_symlink_at_a_seed_path_is_left_alone() {
        let home = Home::new();
        let (_source, root) = project(&home);
        let target = root.path().join("nowhere");
        symlink(&target, root.path().join("s.txt")).unwrap();
        let run = home.ok(root.path(), &["sync"]);
        assert_eq!(run.outcome("s.txt"), None);
        assert!(!target.exists());
    }
}

// spec: SYN-59
#[test]
fn the_personal_configuration_is_ignored_by_git_and_never_managed() {
    let home = Home::new();
    let (source, root) = project(&home);
    let shipped = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../files/.claude/.gitignore"
    );
    put(
        source.path(),
        "files/.claude/.gitignore",
        &fs::read_to_string(shipped).unwrap(),
    );
    home.git(root.path(), &["init", "-q"]);
    home.ok(root.path(), &["sync"]);
    put(root.path(), ".claude/cardinal.local.toml", "[project]\n");
    home.git(
        root.path(),
        &["check-ignore", "-q", ".claude/cardinal.local.toml"],
    );
    for args in [&["sync"][..], &["check"]] {
        let run = home.ok(root.path(), args);
        assert!(
            !format!("{run:?}").contains("cardinal.local"),
            "{args:?}: {run:?}"
        );
    }
    assert!(!get(root.path(), LOCK).unwrap().contains("cardinal.local"));
}
