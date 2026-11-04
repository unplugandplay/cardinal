//! The hook commands the installed `cardinal` must accept (BIN-15, BIN-16), run as the binary with a fake installed
//! launcher first on the path.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::Path;

use common::{get, project, put, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

const SETTINGS: &str = ".claude/settings.json";

/// Make the installed `cardinal` of `home` one that knows only the subcommand `run`, logging each call to `<home>/log`.
fn install_cardinal_with_run(home: &Home) {
    home.program(
        "cardinal",
        &format!(
            "echo \"$*\" >> '{}/log'\n[ \"$1\" = help ] && [ \"$2\" = run ]\n",
            home.path().display()
        ),
    );
}

const HOOKS: &str = r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
    {"type": "command", "command": "cardinal run review-gate check"},
    {"type": "command", "command": "cardinal 'run' quoted"},
    {"type": "command", "command": "cardinal newcmd --flag"},
    {"type": "command", "command": "sh -c 'cardinal newcmd'"}
]}], "Stop": [{"hooks": [{"type": "command", "command": "cardinal newcmd again"}]}]}}"#;

/// A synced project whose settings hold `HOOKS`.
fn project_with_hooks(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), SETTINGS, HOOKS);
    (source, root)
}

// spec: BIN-15, BIN-20
#[test]
fn check_fails_on_each_hook_command_whose_subcommand_the_installed_cardinal_refuses() {
    let home = Home::new();
    let (source, root) = project_with_hooks(&home);
    install_cardinal_with_run(&home);

    let run = home.harness(root.path(), &["check"]);

    assert_eq!(run.code, Some(1), "{run:?}");
    let problems = run.lines();
    assert_eq!(problems.len(), 2, "{run:?}");
    assert!(problems[0].contains("`cardinal newcmd --flag`"), "{run:?}");
    assert!(problems[1].contains("`cardinal newcmd again`"), "{run:?}");
    // the install of the project's source, run from its root
    let install = format!(
        "cargo install --force --path {}/crates/cardinal --locked",
        source.path().display()
    );
    assert!(problems.iter().all(|p| p.contains(&install)), "{run:?}");
    // each subcommand is asked once
    assert_eq!(get(home.path(), "log").unwrap(), "help run\nhelp newcmd\n");
}

// spec: BIN-20
#[test]
fn check_names_the_cargo_install_of_a_git_source_at_a_revision_that_is_no_commit() {
    let home = Home::new();
    let (_source, root) = project_with_hooks(&home);
    install_cardinal_with_run(&home);
    let source = "https://example.com/owner/repo.git";

    for (rev, install) in [
        (
            "rev = \"main\"\n",
            format!("cargo install --force --git {source} --rev main --locked"),
        ),
        ("", format!("cargo install --force --git {source} --locked")),
    ] {
        put(
            root.path(),
            CONFIG,
            &format!("[cardinal]\nsource = \"{source}\"\n{rev}"),
        );
        let run = home.harness(root.path(), &["check"]);
        assert_eq!(run.code, Some(1), "{run:?}");
        assert!(!run.lines().is_empty(), "{run:?}");
        assert!(
            run.lines().iter().all(|p| p.contains(&install)),
            "{rev}: {run:?}"
        );
    }
}

// spec: BIN-15
#[test]
fn check_looks_at_nothing_without_a_launcher_on_the_path() {
    let home = Home::new();
    let (_source, root) = project_with_hooks(&home);
    fs::remove_file(home.path().join("bin/cardinal")).unwrap();

    let run = home.harness(root.path(), &["check"]);

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(run.stdout, "");
}

const CARDINAL_HOOKS: &str = r#"{"hooks": {"Stop": [{"hooks": [
    {"type": "command", "command": "cardinal run review-gate check"},
    {"type": "command", "command": "cardinal newcmd"},
    {"type": "command", "command": "harness run language-hint"}
]}]}}"#;

// spec: BIN-15
#[test]
fn check_judges_no_harness_hook_and_no_cardinal_hook_through_the_harness_of_before() {
    let home = Home::new();
    let (_source, root) = project_with_hooks(&home);
    put(root.path(), SETTINGS, CARDINAL_HOOKS);
    home.program(
        "harness",
        &format!("echo \"$*\" >> '{}/log'\nexit 1\n", home.path().display()),
    );
    fs::remove_file(home.path().join("bin/cardinal")).unwrap();

    let run = home.harness(root.path(), &["check"]);

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(run.stdout, "");

    // a `cardinal` that knows `run` only refuses its unknown subcommand, and the `harness` hook stays unjudged
    install_cardinal_with_run(&home);
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.lines().len(), 1, "{run:?}");
    assert!(run.lines()[0].contains("`cardinal newcmd`"), "{run:?}");
    assert_eq!(get(home.path(), "log").unwrap(), "help run\nhelp newcmd\n");
}

/// A git source whose head adds the hooks `cardinal newcmd` and `cardinal newcmd other`, a project moved to its tag
/// `v1`, and the head.
fn source_with_a_new_hook_command(home: &Home) -> (TempDir, TempDir, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/a.txt", "a1");
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
    put(
        repo.path(),
        &format!("entries/{SETTINGS}"),
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "cardinal newcmd"},
            {"type": "command", "command": "cardinal newcmd other"}]}]}}"#,
    );
    put(repo.path(), "files/a.txt", "a2");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "new hook"]);
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    (repo, root, head)
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

// spec: BIN-16, BIN-20
#[test]
fn update_writes_nothing_and_names_the_install_when_the_installed_cardinal_refuses_a_hook() {
    let home = Home::new();
    let (repo, root, head) = source_with_a_new_hook_command(&home);
    install_cardinal_with_run(&home);
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));

    let run = home.harness(root.path(), &["update"]);

    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("`cardinal newcmd`"), "{run:?}");
    assert!(run.stderr.contains("`cardinal newcmd other`"), "{run:?}");
    let install = format!("cardinal upgrade {} {head}", url(repo.path()));
    // one install for all the hooks
    assert_eq!(run.stderr.matches(&install).count(), 1, "{run:?}");
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert!(get(root.path(), SETTINGS).is_none());

    // once the cardinal of the target is installed, the same update goes through
    home.program("cardinal", "exit 0\n");
    let run = home.ok(root.path(), &["update"]);
    assert!(run.stdout.starts_with("rev: "), "{run:?}");
    assert!(get(root.path(), CONFIG).unwrap().contains(&head), "{run:?}");
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
}

// spec: BIN-16, BIN-20
#[test]
fn update_dry_run_warns_of_a_hook_the_installed_cardinal_refuses() {
    let home = Home::new();
    let (_repo, root, _head) = source_with_a_new_hook_command(&home);
    install_cardinal_with_run(&home);
    let config = get(root.path(), CONFIG);

    let run = home.ok(root.path(), &["update", "--dry-run"]);

    let warned = |text: &str| {
        run.stderr
            .lines()
            .any(|w| w.starts_with("warning:") && w.contains(text))
    };
    assert!(
        warned("`cardinal newcmd`") && warned("cardinal upgrade"),
        "{run:?}"
    );
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

// spec: FEA-14
#[test]
fn update_goes_through_when_the_refused_hook_belongs_to_a_feature_turned_off() {
    let home = Home::new();
    let (repo, root, head) = source_with_a_new_hook_command(&home);
    put(
        repo.path(),
        "features.toml",
        "[newcmd]\nentries = [\"hook Stop cardinal newcmd\", \"hook Stop cardinal newcmd other\"]\n",
    );
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "feature"]);
    let feature = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let config = get(root.path(), CONFIG).unwrap();
    put(
        root.path(),
        CONFIG,
        &config.replacen("[cardinal]\n", "[cardinal]\ndisabled = [\"newcmd\"]\n", 1),
    );
    install_cardinal_with_run(&home);

    let run = home.ok(root.path(), &["update"]);

    assert!(!run.stderr.contains("newcmd"), "{run:?}");
    let config = get(root.path(), CONFIG).unwrap();
    assert!(
        config.contains(&feature) && !config.contains(&head),
        "{run:?}"
    );
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
}
