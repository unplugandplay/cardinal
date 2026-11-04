//! The native review gate `sync` installs, run as the binary on temp dirs, and the `pre-commit` it writes, run with
//! `/bin/sh` (SYN-18 to SYN-23, SYN-60, BIN-9, BIN-14, FEA-24, FEA-25).
#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{
    executable, gated, get, output, put, script, set_mode, snapshot, Home, Run, CONFIG, RECORD,
};
use tempfile::{tempdir, TempDir};

const GATE: &str = "native review gate";
const MOVED: &str = "pre-commit -> pre-commit.user";
const SKIPPED: &str = "review gate skipped";

/// `gated`, synced.
fn synced(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = gated(home);
    home.ok(root.path(), &["sync"]);
    (source, root)
}

fn pre_commit(root: &TempDir) -> PathBuf {
    root.path().join(".git/hooks/pre-commit")
}

/// A directory holding only a fake `program` that records its arguments in `calls`, then runs `body`.
fn fake(program: &str, body: &str) -> TempDir {
    let bin = tempdir().unwrap();
    script(
        &bin.path().join(program),
        &format!("echo \"$@\" >> '{}/calls'\n{body}", bin.path().display()),
    );
    bin
}

/// A `cardinal` that knows `run` and whose `run` exits with `code`.
fn fake_cardinal(code: i32) -> TempDir {
    fake_launcher("cardinal", code)
}

/// A launcher `name` that knows `run` and whose `run` exits with `code`.
fn fake_launcher(name: &str, code: i32) -> TempDir {
    fake(name, &format!("[ \"$1\" = help ] && exit 0\nexit {code}\n"))
}

/// What the fake of `bin` was called with, one line per call; empty when it never ran.
fn calls(bin: &TempDir) -> String {
    get(bin.path(), "calls").unwrap_or_default()
}

/// Run the installed hook in `root`, as git does, with `path` as the whole `PATH` of its shell.
fn run_hook(home: &Home, root: &TempDir, path: &Path, args: &[&str]) -> Run {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(".git/hooks/pre-commit")
        .args(args)
        .current_dir(root.path())
        .env("PATH", path);
    home.isolated(&mut cmd);
    output(cmd, "")
}

/// A `pre-commit` of the user that records its arguments in `ran` and exits with `code`.
fn user_hook(root: &TempDir, code: i32) {
    script(
        &pre_commit(root),
        &format!("echo \"user $@\" >> ran\nexit {code}\n"),
    );
}

// spec: SYN-18
#[test]
fn sync_installs_an_executable_hook_that_runs_the_review_in_native_mode() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    assert!(!pre_commit(&root).exists(), "init alone installs no gate");
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    let text = get(root.path(), ".git/hooks/pre-commit").unwrap();
    assert!(
        // the mark every revision reads as its own gate, the ones before the rename included
        text.starts_with("#!/bin/sh\n# harness: native review gate"),
        "{text}"
    );
    assert!(text.contains("run review-gate native"), "{text}");
    assert!(executable(&pre_commit(&root)));
}

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_and_fails_the_commit_with_its_exit_code() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    for code in [0, 1, 2] {
        let bin = fake_cardinal(code);
        let hook = run_hook(&home, &root, bin.path(), &[]);
        assert_eq!(hook.code, Some(code), "{hook:?}");
        assert_eq!(calls(&bin), "help run\nrun review-gate native\n");
    }
}

// spec: SYN-18, BIN-14
#[test]
fn the_hook_lets_the_commit_through_when_the_launcher_cannot_run_the_review() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let bin = fake_cardinal(3);
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
}

/// The review hook script of the revisions before the tools moved into `harness`.
const LEGACY_HOOK: &str = ".claude/hooks/require_code_review.rs";

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_script_of_an_older_revision_with_rust_script() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    put(
        root.path(),
        LEGACY_HOOK,
        "// the review hook of an older revision",
    );
    let bin = fake("rust-script", "exit 2\n");
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(2), "{hook:?}");
    assert_eq!(calls(&bin), format!("{LEGACY_HOOK} native\n"));
    // its script without `rust-script`: skipped, not refused
    let empty = tempdir().unwrap();
    let hook = run_hook(&home, &root, empty.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
}

// spec: SYN-18, BIN-9
#[test]
fn the_hook_warns_and_lets_the_commit_through_without_cardinal() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    // no configuration in the work tree (a branch that predates Cardinal)
    let config = get(root.path(), CONFIG).unwrap();
    fs::remove_file(root.path().join(CONFIG)).unwrap();
    let bin = fake_cardinal(2);
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    assert_eq!(calls(&bin), "");
    // the configuration present, no cardinal on the PATH
    put(root.path(), CONFIG, &config);
    let empty = tempdir().unwrap();
    let hook = run_hook(&home, &root, empty.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    // a launcher that predates `run`: its `help run` fails, as any unknown subcommand
    let old = fake("cardinal", "exit 2\n");
    let hook = run_hook(&home, &root, old.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    assert_eq!(calls(&old), "help run\n");
}

// spec: BIN-9
#[test]
fn the_hook_runs_the_gate_through_cardinal_never_through_harness() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let cardinal = fake_cardinal(2);
    let harness = fake_launcher("harness", 2);
    let both = std::env::join_paths([cardinal.path(), harness.path()]).unwrap();

    let hook = run_hook(&home, &root, Path::new(&both), &[]);

    assert_eq!(hook.code, Some(2), "the refusal of cardinal: {hook:?}");
    assert_eq!(calls(&cardinal), "help run\nrun review-gate native\n");
    assert_eq!(calls(&harness), "");
    // only the `harness` of before the rename: skipped, as without a launcher
    let hook = run_hook(&home, &root, harness.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    assert_eq!(calls(&harness), "");
}

// spec: SYN-19
#[test]
fn sync_moves_a_hook_of_the_user_aside_and_the_gate_runs_it_after_the_review() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 0);
    let original = get(root.path(), ".git/hooks/pre-commit").unwrap();
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    let chained = root.path().join(".git/hooks/pre-commit.user");
    assert_eq!(fs::read_to_string(&chained).unwrap(), original);
    assert!(executable(&chained));

    let bin = fake_cardinal(0);
    let hook = run_hook(&home, &root, bin.path(), &["msg-file", "message"]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert_eq!(calls(&bin), "help run\nrun review-gate native\n");
    assert_eq!(
        get(root.path(), "ran").as_deref(),
        Some("user msg-file message\n")
    );
}

// spec: SYN-19
#[test]
fn a_refusal_of_the_gate_stops_the_commit_before_the_hook_of_the_user() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 0);
    home.ok(root.path(), &["sync"]);
    let bin = fake_cardinal(2);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(2));
    assert!(get(root.path(), "ran").is_none());
}

// spec: SYN-19
#[test]
fn a_failure_of_the_hook_of_the_user_fails_the_commit() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 7);
    home.ok(root.path(), &["sync"]);
    let bin = fake_cardinal(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(7));
    assert_eq!(get(root.path(), "ran").as_deref(), Some("user \n"));
}

// spec: SYN-19
#[test]
fn sync_never_replaces_a_chained_hook_and_fails_changing_nothing() {
    // a dangling link is a `pre-commit.user` too, though `exists()` says it is not
    let put_chained: [fn(&Path); 2] = [
        |root| put(root, ".git/hooks/pre-commit.user", "kept"),
        |root| symlink("missing", root.join(".git/hooks/pre-commit.user")).unwrap(),
    ];
    for put_chained in put_chained {
        let home = Home::new();
        let (_source, root) = gated(&home);
        user_hook(&root, 0);
        put_chained(root.path());
        let before = snapshot(root.path());
        let sync = home.harness(root.path(), &["sync"]);
        assert_ne!(sync.code, Some(0), "{sync:?}");
        assert!(sync.stderr.contains("pre-commit.user"), "{sync:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-19, SYN-20
#[test]
fn a_gate_installed_before_the_rename_is_ours_and_replaced() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    // the mark of the revisions before the rename, and the one two revisions after it wrote
    for mark in [
        "# harness: native review gate (installed by `harness sync`, do not edit)",
        "# cardinal: native review gate (installed by `cardinal sync`, do not edit)",
    ] {
        script(&pre_commit(&root), &format!("{mark}\nexit 0\n"));
        let sync = home.ok(root.path(), &["sync"]);
        assert_eq!(sync.outcome(GATE), Some("updated"), "{mark}: {sync:?}");
        assert_eq!(sync.outcome(MOVED), None, "{mark}: {sync:?}");
    }
    assert!(!root.path().join(".git/hooks/pre-commit.user").exists());
}

// spec: SYN-19
#[test]
fn a_hook_that_only_quotes_the_mark_is_the_users() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    script(
        &pre_commit(&root),
        "# copied from: # harness: native review gate\nexit 0\n",
    );
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
    assert!(get(root.path(), ".git/hooks/pre-commit.user")
        .unwrap()
        .contains("copied from"));
}

// spec: SYN-19
#[test]
fn a_symlinked_hook_of_the_user_still_reaches_its_target_once_moved() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    script(&root.path().join("scripts/hook"), "echo linked >> ran\n");
    fs::create_dir_all(root.path().join(".git/hooks")).unwrap();
    symlink("../../scripts/hook", pre_commit(&root)).unwrap();
    home.ok(root.path(), &["sync"]);
    assert!(fs::symlink_metadata(pre_commit(&root)).unwrap().is_file());
    let bin = fake_cardinal(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(0));
    assert_eq!(get(root.path(), "ran").as_deref(), Some("linked\n"));
}

// spec: SYN-19
#[test]
fn a_dangling_hook_of_the_user_moves_as_a_link_and_leaves_no_staged_file() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    fs::create_dir_all(root.path().join(".git/hooks")).unwrap();
    symlink("missing", pre_commit(&root)).unwrap();
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    let hooks = root.path().join(".git/hooks");
    assert_eq!(
        fs::read_link(hooks.join("pre-commit.user")).unwrap(),
        Path::new("missing")
    );
    assert!(!hooks.join("pre-commit.cardinal-new").exists());
    let bin = fake_cardinal(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(0));
}

// spec: SYN-20
#[test]
fn sync_reinstalls_a_missing_modified_or_non_executable_hook_and_is_idempotent() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let managed = get(root.path(), ".git/hooks/pre-commit").unwrap();
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), None);

    fs::remove_file(pre_commit(&root)).unwrap();
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), Some("added"));

    fs::write(pre_commit(&root), format!("{managed}# edited\n")).unwrap();
    assert_eq!(
        home.ok(root.path(), &["sync"]).outcome(GATE),
        Some("updated")
    );
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit").as_deref(),
        Some(managed.as_str())
    );

    set_mode(&pre_commit(&root), 0o644);
    assert_eq!(
        home.ok(root.path(), &["sync"]).outcome(GATE),
        Some("updated")
    );
    assert!(executable(&pre_commit(&root)));

    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), None);
    assert!(!root.path().join(".git/hooks/pre-commit.user").exists());
}

// spec: SYN-20
#[test]
fn a_dry_run_reports_the_installation_and_writes_nothing() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    let before = snapshot(root.path());
    let dry = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(dry.outcome(GATE), Some("added"), "{dry:?}");
    assert_eq!(snapshot(root.path()), before);

    user_hook(&root, 0);
    let before = snapshot(root.path());
    let dry = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(dry.outcome(MOVED), Some("moved"), "{dry:?}");
    assert_eq!(dry.outcome(GATE), Some("added"), "{dry:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-21
#[test]
fn a_linked_worktree_gets_the_gate_in_the_hooks_of_the_common_git_directory() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    home.git(root.path(), &["add", CONFIG]);
    home.git(root.path(), &["commit", "--quiet", "-m", "init"]);
    let linked = tempdir().unwrap();
    let wt = linked.path().join("wt");
    home.git(
        root.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "other",
            wt.to_str().unwrap(),
        ],
    );

    let sync = home.ok(&wt, &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    assert!(get(root.path(), ".git/hooks/pre-commit").is_some_and(|t| t.contains(GATE)));
    let private = home.git(&wt, &["rev-parse", "--absolute-git-dir"]);
    assert!(!Path::new(&private).join("hooks").exists(), "{private}");
    // once for all the worktrees
    assert_eq!(home.ok(&wt, &["sync"]).outcome(GATE), None);
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), None);
}

// spec: SYN-22
#[test]
fn sync_installs_nothing_when_core_hooks_path_is_set_and_says_why() {
    let home = Home::new();
    for hooks_path in [None, Some(".githooks")] {
        let (_source, root) = gated(&home);
        let elsewhere = tempdir().unwrap();
        let value = hooks_path.map_or(elsewhere.path().to_str().unwrap(), |p| p);
        home.git(root.path(), &["config", "core.hooksPath", value]);
        let sync = home.ok(root.path(), &["sync"]);
        assert_eq!(sync.outcome(GATE), Some("skipped"), "{sync:?}");
        let why = sync
            .lines()
            .into_iter()
            .find(|l| l.ends_with(&format!(": {GATE}")));
        assert!(
            why.is_some_and(|l| l.contains("core.hooksPath")),
            "{sync:?}"
        );
        assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
        assert!(!pre_commit(&root).exists());
        assert!(fs::read_dir(elsewhere.path()).unwrap().next().is_none());
        assert!(!root.path().join(".githooks").exists());
    }
}

// spec: SYN-22
#[test]
fn no_gate_and_no_error_outside_a_git_repository_or_without_the_review_hook() {
    let home = Home::new();
    // not a repository
    let (_source, root) = gated(&home);
    fs::remove_dir_all(root.path().join(".git")).unwrap();
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));

    // a source that manages no hook of the review gate
    let (source, root) = gated(&home);
    fs::remove_file(source.path().join("entries/.claude/settings.json")).unwrap();
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert!(!pre_commit(&root).exists());

    // a project inside another repository is not the top of one
    let (_source, root) = gated(&home);
    let outer = tempdir().unwrap();
    home.git(outer.path(), &["init", "--quiet", "-b", "main"]);
    let inner = outer.path().join("project");
    put(&inner, CONFIG, &get(root.path(), CONFIG).unwrap());
    let sync = home.ok(&inner, &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert!(!outer.path().join(".git/hooks/pre-commit").exists());
}

/// `cardinal check` on `root`, with the `CI` environment variable set when `ci`.
fn check(home: &Home, root: &TempDir, ci: bool) -> Run {
    let env: &[(&str, &str)] = if ci { &[("CI", "true")] } else { &[] };
    home.harness_env(root.path(), &["check"], env)
}

/// Assert `check` exits 0 with no problem when `problems` is empty, else exits 1 printing exactly `problems`.
fn assert_problems(run: &Run, problems: &[&str]) {
    let code = i32::from(!problems.is_empty());
    assert_eq!(run.code, Some(code), "{run:?}");
    assert_eq!(run.lines(), problems, "{run:?}");
}

const MISSING: &str = "missing: native review gate";
const MODIFIED: &str = "modified: native review gate";

// spec: SYN-23
#[test]
fn check_reports_a_missing_or_modified_gate() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    assert_problems(&check(&home, &root, false), &[]);

    let managed = get(root.path(), ".git/hooks/pre-commit").unwrap();
    fs::write(pre_commit(&root), format!("{managed}# edited\n")).unwrap();
    assert_problems(&check(&home, &root, false), &[MODIFIED]);

    fs::write(pre_commit(&root), &managed).unwrap();
    set_mode(&pre_commit(&root), 0o644);
    assert_problems(&check(&home, &root, false), &[MODIFIED]);

    fs::remove_file(pre_commit(&root)).unwrap();
    assert_problems(&check(&home, &root, false), &[MISSING]);
    // a fresh clone of a CI run has no git hooks
    assert_problems(&check(&home, &root, true), &[]);

    // a hook of the user without ours is a missing gate too
    user_hook(&root, 0);
    assert_problems(&check(&home, &root, false), &[MISSING]);
}

/// The line of `check`, and the error of `sync`, on a hook of the user and a `pre-commit.user` both in `root`.
fn conflict(root: &TempDir) -> String {
    let hooks = root.path().join(".git/hooks");
    let (hook, chained) = (hooks.join("pre-commit"), hooks.join("pre-commit.user"));
    let (hook, chained) = (hook.display(), chained.display());
    format!(
        "{hook} and {chained} both exist: merge them by hand into {chained} (replace it if it is a dangling link), remove {hook}, then run `cardinal sync`, which never overwrites a hook of yours"
    )
}

// spec: SYN-23
#[test]
fn check_names_the_chained_hook_conflict_sync_refuses() {
    let put_chained: [fn(&Path); 2] = [
        |root| put(root, ".git/hooks/pre-commit.user", "kept"),
        |root| symlink("missing", root.join(".git/hooks/pre-commit.user")).unwrap(),
    ];
    for put_chained in put_chained {
        let home = Home::new();
        let (_source, root) = synced(&home);
        user_hook(&root, 0);
        put_chained(root.path());
        let line = format!("conflict: {GATE} ({})", conflict(&root));
        assert_problems(&check(&home, &root, false), &[&line]);
        assert_problems(&check(&home, &root, true), &[]);
        let before = snapshot(root.path());
        let sync = home.harness(root.path(), &["sync"]);
        assert_ne!(sync.code, Some(0), "{sync:?}");
        assert!(sync.stderr.contains(&conflict(&root)), "{sync:?}");
        assert_eq!(snapshot(root.path()), before);
    }
    // the merge the line asks for is one `sync` installs
    let home = Home::new();
    let (_source, root) = synced(&home);
    user_hook(&root, 0);
    put(root.path(), ".git/hooks/pre-commit.user", "kept");
    fs::rename(
        pre_commit(&root),
        root.path().join(".git/hooks/pre-commit.user"),
    )
    .unwrap();
    home.ok(root.path(), &["sync"]);
    assert_problems(&check(&home, &root, false), &[]);
}

// spec: SYN-19, SYN-23
#[test]
fn an_install_killed_between_the_link_and_the_rename_is_finished() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    user_hook(&root, 0);
    // what a `sync` killed after linking the hook of the user as `pre-commit.user` leaves
    let chained = root.path().join(".git/hooks/pre-commit.user");
    fs::hard_link(pre_commit(&root), &chained).unwrap();
    put(root.path(), ".git/hooks/pre-commit.cardinal-new", "staged");
    assert_problems(&check(&home, &root, false), &[MISSING]);
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    // the move was the killed run's
    assert_eq!(sync.outcome(MOVED), None, "{sync:?}");
    assert!(get(root.path(), ".git/hooks/pre-commit")
        .unwrap()
        .contains("native review gate"));
    assert!(get(root.path(), ".git/hooks/pre-commit.user")
        .unwrap()
        .contains("user"));
    assert!(!root
        .path()
        .join(".git/hooks/pre-commit.cardinal-new")
        .exists());
    assert_problems(&check(&home, &root, false), &[]);
    let bin = fake_cardinal(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &["a"]).code, Some(0));
    assert_eq!(get(root.path(), "ran").as_deref(), Some("user a\n"));
}

// spec: SYN-22, SYN-23
#[test]
fn a_review_hook_named_before_the_rename_gets_no_gate() {
    let home = Home::new();
    let (source, root) = gated(&home);
    let old = RECORD.replacen("cardinal", "harness", 1);
    let fragment = serde_json::json!({"hooks": {"PostToolUse": [{"matcher": "Skill", "hooks": [
        {"type": "command", "command": old}]}]}});
    put(
        source.path(),
        "entries/.claude/settings.json",
        &fragment.to_string(),
    );
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert!(!pre_commit(&root).exists());
    assert_problems(&check(&home, &root, false), &[]);
}

// spec: SYN-23
#[test]
fn check_expects_the_gate_the_lock_records_even_once_out_of_the_settings() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    fs::remove_file(pre_commit(&root)).unwrap();
    put(root.path(), ".claude/settings.json", "{}");
    let entry = format!(".claude/settings.json hook PostToolUse Skill {RECORD}");
    let missing_entry = format!("missing: {entry}");
    assert_problems(&check(&home, &root, false), &[&missing_entry, MISSING]);
    // a fresh clone of a CI run has no git hooks
    assert_problems(&check(&home, &root, true), &[&missing_entry]);
}

// spec: SYN-23
#[test]
fn check_expects_no_gate_it_cannot_or_need_not_install() {
    let home = Home::new();
    // core.hooksPath set
    let (_source, root) = synced(&home);
    fs::remove_file(pre_commit(&root)).unwrap();
    home.git(root.path(), &["config", "core.hooksPath", ".githooks"]);
    assert_problems(&check(&home, &root, false), &[]);

    // a source that manages no hook of the review gate
    let (source, root) = gated(&home);
    fs::remove_file(source.path().join("entries/.claude/settings.json")).unwrap();
    home.ok(root.path(), &["sync"]);
    assert_problems(&check(&home, &root, false), &[]);

    // not a repository
    let (_source, root) = gated(&home);
    fs::remove_dir_all(root.path().join(".git")).unwrap();
    home.ok(root.path(), &["sync"]);
    assert_problems(&check(&home, &root, false), &[]);
}

// spec: SYN-22, SYN-23
#[test]
fn a_source_of_an_older_revision_still_gets_and_checks_the_gate() {
    let home = Home::new();
    let (source, root) = gated(&home);
    fs::remove_file(source.path().join("entries/.claude/settings.json")).unwrap();
    put(
        source.path(),
        &format!("files/{LEGACY_HOOK}"),
        "// the review hook",
    );
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    fs::remove_file(pre_commit(&root)).unwrap();
    assert_problems(&check(&home, &root, false), &[MISSING]);
    // even with its hook script deleted since
    fs::remove_file(root.path().join(LEGACY_HOOK)).unwrap();
    assert_problems(
        &check(&home, &root, false),
        &[&format!("missing: {LEGACY_HOOK}"), MISSING],
    );
}

const RESTORED: &str = "pre-commit.user -> pre-commit";

/// Make the source of `gated` manage no hook of the review gate any more.
fn drop_gate(source: &TempDir) {
    put(source.path(), "entries/.claude/settings.json", "{}");
}

// spec: SYN-60
#[test]
fn sync_removes_the_gate_once_the_source_no_longer_manages_it() {
    let home = Home::new();
    let (source, root) = synced(&home);
    drop_gate(&source);
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("removed"), "{sync:?}");
    assert_eq!(sync.outcome(RESTORED), None, "{sync:?}");
    assert!(fs::symlink_metadata(pre_commit(&root)).is_err());
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), None);

    // an edited gate, or one of the revisions that wrote the other mark, is ours too
    for (from, to) in [
        ("cardinal: review", "x"),
        ("# harness: native", "# cardinal: native"),
    ] {
        let (source, root) = synced(&home);
        let managed = get(root.path(), ".git/hooks/pre-commit").unwrap();
        fs::write(pre_commit(&root), managed.replace(from, to)).unwrap();
        drop_gate(&source);
        let sync = home.ok(root.path(), &["sync"]);
        assert_eq!(sync.outcome(GATE), Some("removed"), "{to}: {sync:?}");
        assert!(fs::symlink_metadata(pre_commit(&root)).is_err(), "{to}");
    }
}

// spec: SYN-60
#[test]
fn removing_the_gate_gives_the_chained_hook_of_the_user_back() {
    let home = Home::new();
    let (source, root) = gated(&home);
    user_hook(&root, 1);
    let user = get(root.path(), ".git/hooks/pre-commit").unwrap();
    home.ok(root.path(), &["sync"]);
    drop_gate(&source);
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("removed"), "{sync:?}");
    assert_eq!(sync.outcome(RESTORED), Some("moved"), "{sync:?}");
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit").as_deref(),
        Some(user.as_str())
    );
    assert!(executable(&pre_commit(&root)));
    assert!(!root.path().join(".git/hooks/pre-commit.user").exists());
    // the user's hook runs as the commit's hook again
    let hook = run_hook(&home, &root, Path::new("/usr/bin:/bin"), &["x"]);
    assert_eq!(hook.code, Some(1), "{hook:?}");
    assert_eq!(get(root.path(), "ran").as_deref(), Some("user x\n"));
}

// spec: SYN-60
#[test]
fn removing_the_gate_gives_a_symlinked_hook_of_the_user_back_as_a_link() {
    let home = Home::new();
    let (source, root) = gated(&home);
    script(&root.path().join("scripts/hook"), "echo linked >> ran\n");
    fs::create_dir_all(root.path().join(".git/hooks")).unwrap();
    symlink("../../scripts/hook", pre_commit(&root)).unwrap();
    home.ok(root.path(), &["sync"]);
    drop_gate(&source);
    home.ok(root.path(), &["sync"]);
    assert!(fs::symlink_metadata(pre_commit(&root))
        .unwrap()
        .is_symlink());
    assert_eq!(
        fs::canonicalize(pre_commit(&root)).unwrap(),
        fs::canonicalize(root.path().join("scripts/hook")).unwrap()
    );
}

// spec: SYN-60
#[test]
fn without_the_gate_sync_leaves_the_hooks_of_the_user_alone() {
    let home = Home::new();
    // a hook of the user that is not ours
    let (source, root) = gated(&home);
    drop_gate(&source);
    user_hook(&root, 0);
    let hooks = root.path().join(".git/hooks");
    put(&hooks, "pre-commit.user", "#!/bin/sh\n");
    let before = snapshot(&hooks);
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert_eq!(snapshot(&hooks), before);

    // a `pre-commit.user` alone, with no gate
    fs::remove_file(pre_commit(&root)).unwrap();
    let before = snapshot(&hooks);
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(RESTORED), None);
    assert_eq!(snapshot(&hooks), before);

    // core.hooksPath set: the gate under .git/hooks is not the one git runs, it stays
    let (source, root) = synced(&home);
    home.git(root.path(), &["config", "core.hooksPath", ".githooks"]);
    drop_gate(&source);
    let hooks = root.path().join(".git/hooks");
    let before = snapshot(&hooks);
    assert_eq!(home.ok(root.path(), &["sync"]).outcome(GATE), None);
    assert_eq!(snapshot(&hooks), before);
}

// spec: SYN-60
#[test]
fn dry_run_reports_the_removal_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = gated(&home);
    user_hook(&root, 0);
    home.ok(root.path(), &["sync"]);
    drop_gate(&source);
    let hooks = root.path().join(".git/hooks");
    let before = snapshot(&hooks);
    let dry = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(dry.outcome(GATE), Some("removed"), "{dry:?}");
    assert_eq!(dry.outcome(RESTORED), Some("moved"), "{dry:?}");
    assert_eq!(snapshot(&hooks), before);
}

/// Set `disabled` in the `[cardinal]` table of the project's configuration.
fn disable(root: &TempDir, value: &str) {
    let config = get(root.path(), CONFIG).unwrap();
    let config: String = config
        .lines()
        .filter(|l| !l.starts_with("disabled = "))
        .flat_map(|l| [l, "\n"])
        .collect();
    let config = config.replacen(
        "[cardinal]\n",
        &format!("[cardinal]\ndisabled = {value}\n"),
        1,
    );
    put(root.path(), CONFIG, &config);
}

// spec: FEA-24, FEA-25, SYN-60
#[test]
fn a_review_gate_turned_off_is_removed_and_checked_no_more() {
    let home = Home::new();
    let (source, root) = gated(&home);
    let entry = format!("hook PostToolUse Skill {RECORD}");
    put(
        source.path(),
        "features.toml",
        &format!("[review-gate]\nentries = [\"{entry}\"]\n"),
    );
    user_hook(&root, 0);
    let sync = home.ok(root.path(), &["sync"]);
    // the source manages no deny rule of the waiver: the warnings of SYN-24
    assert!(sync.stderr.contains("lacks the deny rule"), "{sync:?}");

    disable(&root, "[\"review-gate\"]");
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("removed"), "{sync:?}");
    assert_eq!(sync.outcome(RESTORED), Some("moved"), "{sync:?}");
    assert!(!sync.stderr.contains("deny rule"), "{sync:?}");
    let run = check(&home, &root, false);
    assert_problems(&run, &[]);
    assert!(!run.stderr.contains("deny rule"), "{run:?}");
    // no gate is expected any more, not even a missing one
    fs::remove_file(pre_commit(&root)).unwrap();
    assert_problems(&check(&home, &root, false), &[]);
    user_hook(&root, 0);

    // turned on again: the gate is back, the user's hook chained again
    disable(&root, "[]");
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
}

// spec: SYN-60
#[test]
fn a_gate_whose_settings_are_left_alone_stays() {
    let home = Home::new();
    let (source, root) = synced(&home);
    // settings sync cannot read keep the review hook, and the lock records it still
    put(root.path(), ".claude/settings.json", "{ not json");
    drop_gate(&source);
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), None, "{sync:?}");
    assert!(get(root.path(), ".git/hooks/pre-commit").is_some_and(|t| t.contains(GATE)));
}
