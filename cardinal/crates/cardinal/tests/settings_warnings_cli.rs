//! The warnings on the project's settings of `check`, `sync` and `update`, run as the binary on temp dirs (SYN-24 to
//! SYN-27), and the deny rules of the shipped managed entries (REV-19).
#![cfg(unix)]

mod common;

use std::fs;
use std::path::Path;

use common::{
    gated, get, project, put, record_hook, snapshot, Home, Run, CONFIG, FORMAT, LOCK, SOURCE_FORMAT,
};
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};

const SETTINGS: &str = ".claude/settings.json";
const FRAGMENT: &str = "entries/.claude/settings.json";
/// The deny rules of REV-19.
const DENY_RULES: [&str; 3] = [
    "Bash(just waive-review)",
    "Bash(just waive-review *)",
    "Edit(//**/claude-review-waived)",
];
const ATTRIBUTION_ON: &str = "warning: .claude/settings.json does not turn off the attribution";
const GONE: &str = "warning: .claude/settings.json runs the missing hook .claude/hooks/gone.rs";

/// Settings running `hooks`, the attribution turned off, with `deny`.
fn settings(hooks: &Value, deny: &[&str]) -> String {
    json!({
        "attribution": {"commit": "", "pr": ""},
        "permissions": {"deny": deny},
        "hooks": hooks,
    })
    .to_string()
}

/// The project of `common::project`, synced.
fn synced(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    (source, root)
}

/// The project of `common::gated`, synced.
fn gated_and_synced(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = gated(home);
    home.ok(root.path(), &["sync"]);
    (source, root)
}

/// The warnings of `run` on the settings, in order.
fn warnings(run: &Run) -> Vec<&str> {
    run.stderr
        .lines()
        .filter(|l| l.starts_with(&format!("warning: {SETTINGS}")))
        .collect()
}

fn deny_warning(rule: &str) -> String {
    format!("warning: {SETTINGS} lacks the deny rule {rule}")
}

/// `cardinal check`, which must exit 0: a warning never makes it fail.
fn check(home: &Home, root: &TempDir) -> Run {
    home.ok(root.path(), &["check"])
}

// spec: SYN-24
#[test]
fn check_warns_of_each_deny_rule_the_settings_lack() {
    let home = Home::new();
    let (_source, root) = gated_and_synced(&home);
    // no deny list at all
    let mut no_deny: Value = serde_json::from_str(&settings(&record_hook(), &[])).unwrap();
    no_deny["permissions"] = json!({"allow": ["Bash(just waive-review)"]});
    put(root.path(), SETTINGS, &no_deny.to_string());
    assert_eq!(
        warnings(&check(&home, &root)),
        DENY_RULES.map(deny_warning).to_vec()
    );
    put(root.path(), SETTINGS, &settings(&record_hook(), &[]));
    assert_eq!(
        warnings(&check(&home, &root)),
        DENY_RULES.map(deny_warning).to_vec()
    );
    put(
        root.path(),
        SETTINGS,
        &settings(&record_hook(), &DENY_RULES[..2]),
    );
    assert_eq!(
        warnings(&check(&home, &root)),
        vec![deny_warning(DENY_RULES[2])]
    );
    put(
        root.path(),
        SETTINGS,
        &settings(&record_hook(), &DENY_RULES),
    );
    assert_eq!(warnings(&check(&home, &root)), [] as [&str; 0]);
    // no settings file: nothing to warn about
    fs::remove_file(root.path().join(SETTINGS)).unwrap();
    let run = home.harness(root.path(), &["check"]);
    assert!(warnings(&run).is_empty(), "{run:?}");
}

// spec: SYN-24
#[test]
fn check_expects_no_deny_rule_without_the_review_gate() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.ok(root.path(), &["sync"]);
    put(root.path(), SETTINGS, &settings(&json!({}), &[]));
    assert_eq!(warnings(&check(&home, &root)), [] as [&str; 0]);
}

// spec: SYN-24, SYN-25, SYN-26
#[test]
fn unparsable_settings_get_one_warning() {
    let home = Home::new();
    let (_source, plain) = synced(&home);
    put(plain.path(), SETTINGS, "{,}");
    let run = check(&home, &plain);
    let said = warnings(&run);
    assert!(
        said.len() == 1 && said[0].starts_with(&format!("warning: {SETTINGS} is not valid JSON")),
        "{run:?}"
    );
    // with the review gate too: not reported as lacking every rule
    let (_source, root) = gated_and_synced(&home);
    put(root.path(), SETTINGS, "{,}");
    let run = home.harness(root.path(), &["check"]);
    let said = warnings(&run);
    assert!(
        said.len() == 1 && said[0].contains("is not valid JSON"),
        "{run:?}"
    );
}

// spec: SYN-25
#[test]
fn check_warns_of_each_hook_the_settings_run_and_the_project_lacks() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    put(root.path(), ".claude/hooks/present.rs", "// here");
    let hooks = json!({
        "PreToolUse": [{"matcher": "Bash", "hooks": [
            {"type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/present.rs"},
            {"type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/gone.rs check"},
        ]}],
        "Stop": [{"hooks": [
            {"type": "command", "command": "rust-script .claude/hooks/also_gone.rs", "timeout": 600},
            {"type": "command", "command": "just lint"},
            {"type": "command", "command": "bash ~/.claude/hooks/notify.sh; run-parts .claude/hooks/"},
            {"type": "command", "command": "sh -c \"$(cat .claude/hooks/present.rs)\" >/dev/null"},
            {"type": "command", "command": "rust-script ./.claude/hooks/gone.rs"},
        ]}],
    });
    put(root.path(), SETTINGS, &settings(&hooks, &[]));
    let run = check(&home, &root);
    assert_eq!(
        warnings(&run),
        vec![
            GONE,
            "warning: .claude/settings.json runs the missing hook .claude/hooks/also_gone.rs",
        ]
    );
    assert!(run.stdout.is_empty(), "{run:?}");
}

// spec: SYN-26
#[test]
fn check_warns_when_the_settings_leave_the_attribution_on() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let said = |text: &str| {
        put(root.path(), SETTINGS, text);
        let run = check(&home, &root);
        warnings(&run)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    for on in [
        "{}",
        r#"{"attribution": {"commit": ""}}"#,
        r#"{"attribution": {"commit": "", "pr": "Made by a bot"}}"#,
        r#"{"attribution": true}"#,
    ] {
        assert_eq!(said(on), vec![ATTRIBUTION_ON], "{on}");
    }
    for off in [
        r#"{"attribution": false}"#,
        r#"{"attribution": {"commit": "", "pr": "", "sessionUrl": false}}"#,
    ] {
        assert!(said(off).is_empty(), "{off}");
    }
}

/// Settings that run `.claude/hooks/gone.rs`, attribution off.
fn running_gone() -> String {
    settings(
        &json!({"Stop": [{"hooks": [
            {"type": "command", "command": "rust-script .claude/hooks/gone.rs"},
        ]}]}),
        &[],
    )
}

// spec: SYN-27
#[test]
fn sync_warns_once_of_the_hook_it_removed_that_the_settings_still_run() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "files/.claude/hooks/gone.rs", "// a hook");
    put(root.path(), SETTINGS, &running_gone());
    let run = home.ok(root.path(), &["sync"]);
    assert!(warnings(&run).is_empty(), "{run:?}");

    fs::remove_file(source.path().join("files/.claude/hooks/gone.rs")).unwrap();
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome(".claude/hooks/gone.rs"), Some("removed"));
    assert_eq!(warnings(&run), vec![GONE]);
}

// spec: SYN-27
#[test]
fn a_dry_run_warns_of_the_settings_as_the_sync_would_leave_them() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "files/.claude/hooks/gone.rs", "// a hook");
    put(root.path(), SETTINGS, &running_gone());
    // the sync would add the hook the settings run: nothing to warn of, though it is not there yet
    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert!(warnings(&run).is_empty(), "{run:?}");
    assert!(get(root.path(), ".claude/hooks/gone.rs").is_none());

    home.ok(root.path(), &["sync"]);
    fs::remove_file(source.path().join("files/.claude/hooks/gone.rs")).unwrap();
    let before = snapshot(root.path());
    // the sync would remove it: warned of, though it is still there
    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(warnings(&run), vec![GONE]);
    assert_eq!(snapshot(root.path()), before);
}

/// A git source with `files/a.txt` on `main`, and a project pointing at it whose settings run `gone.rs`.
fn git_project(home: &Home) -> (TempDir, TempDir) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/a.txt", "a1");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "[cardinal]\nsource = \"file://{}\"\n",
            repo.path().display()
        ),
    );
    put(root.path(), SETTINGS, &running_gone());
    (repo, root)
}

// spec: SYN-27
#[test]
fn update_and_its_dry_run_report_the_warnings_of_their_sync_once() {
    let home = Home::new();
    let (_repo, root) = git_project(&home);
    let run = home.ok(root.path(), &["update", "--dry-run"]);
    assert_eq!(warnings(&run), vec![GONE]);
    assert!(get(root.path(), LOCK).is_none());
    let run = home.ok(root.path(), &["update"]);
    assert_eq!(warnings(&run), vec![GONE]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

/// The managed entries this repository ships to every project.
fn shipped_entries() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(FRAGMENT),
    )
    .unwrap()
}

// spec: REV-19
#[test]
fn the_shipped_settings_entries_deny_the_waiver_to_the_agent() {
    let entries: Value = serde_json::from_str(&shipped_entries()).unwrap();
    let deny = &entries["permissions"]["deny"];
    for rule in DENY_RULES {
        assert!(
            deny.as_array().is_some_and(|d| d.contains(&json!(rule))),
            "the shipped entries lack the deny rule {rule}: {deny}"
        );
    }
    // and `check` asks no more of a project synced on them
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), FRAGMENT, &shipped_entries());
    put(source.path(), "seed/.claude/settings.json", "{}");
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.ok(root.path(), &["sync"]);
    let run = home.harness(root.path(), &["check"]);
    assert!(!run.stderr.contains("lacks the deny rule"), "{run:?}");
}

// spec: FEA-10
#[test]
fn no_attribution_warning_when_the_project_turns_off_the_ban() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "features.toml", "[no-ai-attribution]\n");
    let config = get(root.path(), CONFIG).unwrap();
    let config = config.replacen(
        "[cardinal]\n",
        "[cardinal]\ndisabled = [\"no-ai-attribution\"]\n",
        1,
    );
    put(root.path(), CONFIG, &config);
    put(root.path(), SETTINGS, "{}");
    let sync = home.ok(root.path(), &["sync"]);
    assert!(warnings(&sync).is_empty(), "{sync:?}");
    let run = check(&home, &root);
    assert!(warnings(&run).is_empty(), "{run:?}");
}
