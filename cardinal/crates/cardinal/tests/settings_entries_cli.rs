//! The managed entries of the project's settings, run as the binary on temp dirs (SYN-29 to SYN-36).
#![cfg(unix)]

mod common;

use std::fs;
use std::path::Path;

use common::{get, project, put, snapshot, Home, LOCK};
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};

const SETTINGS: &str = ".claude/settings.json";
const FRAGMENT: &str = "entries/.claude/settings.json";
const DENY: &str = "Bash(just waive-review)";
const RECORD: &str = "rust-script .claude/hooks/review.rs record";
const GATE: &str = "rust-script .claude/hooks/gate.rs";

fn hook(command: &str) -> Value {
    json!({"type": "command", "command": command})
}

/// A fragment that denies `DENY` and runs `RECORD` after `Skill` and `GATE` at `Stop`.
fn fragment() -> Value {
    json!({
        "permissions": {"deny": [DENY]},
        "hooks": {
            "PostToolUse": [{"matcher": "Skill", "hooks": [hook(RECORD)]}],
            "Stop": [{"hooks": [hook(GATE)]}],
        },
    })
}

/// The project of `common::project` whose source manages `fragment`.
fn with_fragment(home: &Home, fragment: &Value) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    put(source.path(), FRAGMENT, &fragment.to_string());
    (source, root)
}

fn settings(root: &TempDir) -> Value {
    serde_json::from_str(&get(root.path(), SETTINGS).unwrap()).unwrap()
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

/// `<entry>` of SYN-34, as `sync` prints it after the outcome.
fn deny(rule: &str) -> String {
    format!("{SETTINGS} deny {rule}")
}

fn hook_entry(event_and_matcher: &str, command: &str) -> String {
    format!("{SETTINGS} hook {event_and_matcher} {command}")
}

/// The lines of stdout about the settings.
fn settings_lines(stdout: &str) -> Vec<&str> {
    stdout.lines().filter(|l| l.contains(SETTINGS)).collect()
}

// spec: SYN-29
#[test]
fn sync_adds_each_managed_entry_the_settings_lack_and_keeps_the_rest() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(
        root.path(),
        SETTINGS,
        &json!({
            "model": "opus",
            "permissions": {"allow": ["Bash(ls)"], "deny": ["Bash(rm *)"]},
            "hooks": {"PostToolUse": [
                {"matcher": "Edit", "hooks": [hook("my-formatter")]},
                {"matcher": "Skill", "hooks": [hook("mine-too")]},
            ]},
        })
        .to_string(),
    );
    home.ok(root.path(), &["sync"]);
    let got = settings(&root);
    assert_eq!(got["model"], "opus");
    assert_eq!(got["permissions"]["allow"], json!(["Bash(ls)"]));
    assert_eq!(got["permissions"]["deny"], json!(["Bash(rm *)", DENY]));
    assert_eq!(
        got["hooks"]["PostToolUse"],
        json!([
            {"matcher": "Edit", "hooks": [hook("my-formatter")]},
            {"matcher": "Skill", "hooks": [hook("mine-too"), hook(RECORD)]},
        ])
    );
    assert_eq!(got["hooks"]["Stop"], json!([{"hooks": [hook(GATE)]}]));
}

// spec: SYN-29
#[test]
fn sync_creates_the_keys_the_settings_lack() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(root.path(), SETTINGS, "{}");
    home.ok(root.path(), &["sync"]);
    assert_eq!(settings(&root), fragment());
}

// spec: SYN-29
#[test]
fn an_entry_the_settings_already_hold_is_not_added_again() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    let mut held = fragment();
    held["hooks"]["Stop"][0]["hooks"]
        .as_array_mut()
        .unwrap()
        .insert(0, hook("mine"));
    // key order differs from the fragment: JSON equality ignores it
    held["hooks"]["PostToolUse"][0]["hooks"][0] = json!({"command": RECORD, "type": "command"});
    let text = serde_json::to_string(&held).unwrap();
    put(root.path(), SETTINGS, &text);
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(get(root.path(), SETTINGS).unwrap(), text, "not written");
    assert_eq!(settings_lines(&run.stdout), Vec::<&str>::new());
}

// spec: SYN-29
#[test]
fn an_empty_or_star_matcher_is_the_same_as_none() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    let held = json!({"hooks": {"Stop": [{"matcher": "", "hooks": [hook(GATE)]}],
        "PostToolUse": [{"matcher": "Skill", "hooks": [hook(RECORD)]}]},
        "permissions": {"deny": [DENY]}});
    for text in [
        held.to_string(),
        held.to_string()
            .replace(r#""matcher":"""#, r#""matcher":"*""#),
    ] {
        put(root.path(), SETTINGS, &text);
        home.ok(root.path(), &["sync"]);
        assert_eq!(get(root.path(), SETTINGS), Some(text));
    }
}

// spec: SYN-29
#[test]
fn a_source_whose_fragment_is_not_valid_fails_and_writes_nothing() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &json!({"hooks": {"Stop": [{"hooks": "x"}]}}));
    put(root.path(), SETTINGS, "{}");
    let before = snapshot(root.path());
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-25, SYN-26, SYN-27, SYN-29
#[test]
fn the_repository_settings_once_synced_give_check_nothing_to_warn_of() {
    let home = Home::new();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", repo.to_str().unwrap()]);
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.stderr, "");
    let run = home.ok(root.path(), &["check"]);
    assert_eq!((run.stdout.as_str(), run.stderr.as_str()), ("", ""));
}

// spec: SYN-30
#[test]
fn the_lock_records_each_managed_entry() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(root.path(), SETTINGS, "{}");
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK).unwrap();
    let entries = lock.lines().filter(|l| l.starts_with("entry  ")).count();
    assert_eq!(entries, 3, "{lock}");
    // the lock reads back, and its entries are no file
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
}

// spec: SYN-32
#[test]
fn sync_keeps_the_key_order_and_writes_indented_json() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(
        root.path(),
        SETTINGS,
        r#"{"zeta": 1, "hooks": {"Stop": []}, "alpha": {"b": 1, "a": 2}}"#,
    );
    home.ok(root.path(), &["sync"]);
    let text = get(root.path(), SETTINGS).unwrap();
    let got: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(keys(&got), ["zeta", "hooks", "alpha", "permissions"]);
    assert_eq!(keys(&got["alpha"]), ["b", "a"]);
    assert_eq!(keys(&got["hooks"]), ["Stop", "PostToolUse"]);
    assert_eq!(
        text,
        format!("{}\n", serde_json::to_string_pretty(&got).unwrap())
    );
}

// spec: SYN-32
#[test]
fn a_second_sync_leaves_the_settings_byte_identical() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(root.path(), SETTINGS, "{}");
    home.ok(root.path(), &["sync"]);
    let before = snapshot(root.path());
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.stdout, "");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-34
#[test]
fn sync_prints_each_added_entry_and_writes_nothing_under_dry_run() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(root.path(), SETTINGS, "{}");
    let before = snapshot(root.path());
    let expected = [
        format!("added: {}", deny(DENY)),
        format!("added: {}", hook_entry("PostToolUse Skill", RECORD)),
        format!("added: {}", hook_entry("Stop", GATE)),
    ];
    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(settings_lines(&run.stdout), expected);
    assert_eq!(snapshot(root.path()), before);
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(settings_lines(&run.stdout), expected);
}

// spec: SYN-34
#[test]
fn a_dry_run_after_seeding_reports_the_entries_of_the_seeded_settings() {
    let home = Home::new();
    let (source, root) = with_fragment(&home, &fragment());
    put(
        source.path(),
        &format!("seed/{SETTINGS}"),
        &json!({"permissions": {"deny": [DENY]}}).to_string(),
    );
    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(run.outcome(SETTINGS), Some("seeded"));
    assert_eq!(run.outcome(&deny(DENY)), None);
    assert_eq!(run.outcome(&hook_entry("Stop", GATE)), Some("added"));
    assert!(get(root.path(), SETTINGS).is_none());
}

// spec: SYN-35
#[test]
fn settings_that_cannot_be_merged_are_left_alone_and_their_entries_kept() {
    let home = Home::new();
    let (_source, root) = with_fragment(&home, &fragment());
    put(root.path(), SETTINGS, "{}");
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK);
    let shape = "is not in the shape of Claude Code settings";
    for (text, why) in [
        ("{ not json", "not valid JSON"),
        ("[]", shape),
        (r#"{"hooks": []}"#, shape),
        (r#"{"permissions": {"deny": "all"}}"#, shape),
    ] {
        put(root.path(), SETTINGS, text);
        let run = home.ok(root.path(), &["sync"]);
        assert_eq!(get(root.path(), SETTINGS).as_deref(), Some(text));
        assert_eq!(get(root.path(), LOCK), lock, "{text}");
        let warning = format!("warning: {SETTINGS} ");
        assert!(
            run.stderr
                .lines()
                .any(|l| l.starts_with(&warning) && l.contains(why)),
            "{text}: a warning says why: {run:?}"
        );
    }
    fs::remove_file(root.path().join(SETTINGS)).unwrap();
    home.ok(root.path(), &["sync"]);
    assert!(get(root.path(), SETTINGS).is_none());
    assert_eq!(get(root.path(), LOCK), lock);
}

/// The `with_fragment` project, synced once from settings holding a hook of its own at `Stop`.
fn synced(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = with_fragment(home, &fragment());
    put(
        root.path(),
        SETTINGS,
        &json!({"hooks": {"Stop": [{"hooks": [hook("mine")]}]}}).to_string(),
    );
    home.ok(root.path(), &["sync"]);
    (source, root)
}

// spec: SYN-31
#[test]
fn sync_removes_the_entries_the_source_dropped_and_the_groups_they_empty() {
    let home = Home::new();
    let (source, root) = synced(&home);
    put(source.path(), FRAGMENT, "{}");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        settings(&root),
        json!({"hooks": {"Stop": [{"hooks": [hook("mine")]}]}, "permissions": {"deny": []}})
    );
    assert_eq!(
        run.outcome(&hook_entry("PostToolUse Skill", RECORD)),
        Some("removed")
    );
    assert_eq!(run.outcome(&deny(DENY)), Some("removed"));
    assert!(!get(root.path(), LOCK).unwrap().contains("entry  "));
}

// spec: SYN-31
#[test]
fn a_changed_hook_replaces_the_old_one_in_its_group() {
    let home = Home::new();
    let (source, root) = synced(&home);
    let mut changed = fragment();
    changed["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    put(source.path(), FRAGMENT, &changed.to_string());
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        settings(&root)["hooks"]["Stop"],
        json!([{"hooks": [hook("mine"), hook("gate")]}])
    );
    assert_eq!(run.outcome(&hook_entry("Stop", GATE)), Some("removed"));
    assert_eq!(run.outcome(&hook_entry("Stop", "gate")), Some("added"));
    // replaced once: the next sync has nothing to do
    let before = snapshot(root.path());
    home.ok(root.path(), &["sync"]);
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-31
#[test]
fn a_changed_hook_alone_in_its_group_keeps_its_place() {
    let home = Home::new();
    let (source, root) = with_fragment(&home, &fragment());
    let before = json!({"permissions": {"deny": [DENY]}, "hooks": {
        "PostToolUse": [
            {"matcher": "Skill", "hooks": [hook(RECORD)]},
            {"matcher": "Edit", "hooks": [hook("mine")]},
        ],
        "Stop": [{"hooks": [hook(GATE)]}],
        "PreCompact": [{"hooks": [hook("mine")]}],
    }});
    put(root.path(), SETTINGS, &before.to_string());
    home.ok(root.path(), &["sync"]);
    let mut changed = fragment();
    changed["hooks"]["PostToolUse"][0]["hooks"][0] = hook("record");
    changed["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    put(source.path(), FRAGMENT, &changed.to_string());
    home.ok(root.path(), &["sync"]);
    let mut expected = before;
    expected["hooks"]["PostToolUse"][0]["hooks"][0] = hook("record");
    expected["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    let got = settings(&root);
    assert_eq!(got, expected);
    assert_eq!(keys(&got["hooks"]), ["PostToolUse", "Stop", "PreCompact"]);
}

// spec: SYN-31
#[test]
fn a_dropped_entry_the_project_edited_is_left_as_it_is_and_said_once() {
    let home = Home::new();
    let (source, root) = synced(&home);
    let mut edited = settings(&root);
    edited["hooks"]["Stop"][0]["hooks"][1]["timeout"] = json!(5);
    put(root.path(), SETTINGS, &edited.to_string());
    let mut dropped = fragment();
    dropped["hooks"].as_object_mut().unwrap().remove("Stop");
    put(source.path(), FRAGMENT, &dropped.to_string());
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(settings(&root)["hooks"]["Stop"], edited["hooks"]["Stop"]);
    assert_eq!(run.outcome(&hook_entry("Stop", GATE)), Some("kept"));
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome(&hook_entry("Stop", GATE)), None);
}

// spec: SYN-33
#[test]
fn a_managed_entry_the_project_removed_or_edited_is_a_conflict_unless_forced() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let mut edited = settings(&root);
    edited["permissions"]["deny"] = json!([]);
    edited["hooks"]["Stop"][0]["hooks"][1]["timeout"] = json!(5);
    put(root.path(), SETTINGS, &edited.to_string());
    let before = snapshot(root.path());

    for _ in 0..2 {
        // the conflict survives the next run: the entries stay recorded, not added back
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(1), "{run:?}");
        assert_eq!(run.outcome(&deny(DENY)), Some("conflict"));
        assert_eq!(run.outcome(&hook_entry("Stop", GATE)), Some("conflict"));
        assert_eq!(snapshot(root.path()), before);
    }

    home.ok(root.path(), &["sync", "--force"]);
    let got = settings(&root);
    assert_eq!(got["permissions"]["deny"], json!([DENY]));
    assert_eq!(
        got["hooks"]["Stop"][0]["hooks"],
        json!([
            hook("mine"),
            edited["hooks"]["Stop"][0]["hooks"][1],
            hook(GATE)
        ])
    );
}

// spec: SYN-33
#[test]
fn settings_seeded_again_get_the_managed_entries_without_conflict() {
    let home = Home::new();
    let (source, root) = synced(&home);
    put(source.path(), &format!("seed/{SETTINGS}"), "{}");
    fs::remove_file(root.path().join(SETTINGS)).unwrap();
    home.ok(root.path(), &["sync"]);
    assert_eq!(settings(&root), fragment());
}

// spec: SYN-36
#[test]
fn check_reports_each_recorded_entry_the_settings_no_longer_hold() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
    let mut edited = settings(&root);
    edited["permissions"]["deny"] = json!(["Bash(rm *)"]);
    edited["hooks"]["Stop"][0]["hooks"][0] = hook("not mine any more");
    put(root.path(), SETTINGS, &edited.to_string());
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.lines(), [format!("missing: {}", deny(DENY))]);
}

// spec: SYN-36
#[test]
fn check_reports_nothing_of_settings_it_cannot_judge() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    for text in [None, Some("{ not json")] {
        match text {
            None => fs::remove_file(root.path().join(SETTINGS)).unwrap(),
            Some(text) => put(root.path(), SETTINGS, text),
        }
        assert_eq!(home.ok(root.path(), &["check"]).stdout, "", "{text:?}");
    }
}

const ALLOW: &str = "Bash(cardinal run *)";

/// The `with_fragment` project whose source allows `ALLOW`, synced once from settings that allow
/// `Bash(harness run *)` only, as a project seeded before the rename; the outcome of `ALLOW` in that sync.
fn allowing(home: &Home) -> (TempDir, TempDir, Option<String>) {
    let (source, root) = with_fragment(home, &json!({"permissions": {"allow": [ALLOW]}}));
    put(
        root.path(),
        SETTINGS,
        &json!({"permissions": {"allow": ["Bash(harness run *)"]}}).to_string(),
    );
    let run = home.ok(root.path(), &["sync"]);
    let outcome = run.outcome(&allow(ALLOW)).map(String::from);
    (source, root, outcome)
}

/// `<entry>` of SYN-34 for an allow rule.
fn allow(rule: &str) -> String {
    format!("{SETTINGS} allow {rule}")
}

// spec: SYN-29
// spec: SYN-34
#[test]
fn sync_appends_a_managed_allow_rule_and_keeps_those_of_the_project() {
    let home = Home::new();
    let (_source, root, outcome) = allowing(&home);
    assert_eq!(
        settings(&root)["permissions"]["allow"],
        json!(["Bash(harness run *)", ALLOW])
    );
    assert_eq!(outcome.as_deref(), Some("added"));
}

// spec: SYN-31
#[test]
fn sync_removes_an_allow_rule_the_source_dropped() {
    let home = Home::new();
    let (source, root, _) = allowing(&home);
    put(source.path(), FRAGMENT, "{}");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        settings(&root)["permissions"]["allow"],
        json!(["Bash(harness run *)"])
    );
    assert_eq!(run.outcome(&allow(ALLOW)), Some("removed"));
}

// spec: SYN-36
#[test]
fn check_reports_a_recorded_allow_rule_the_settings_no_longer_hold() {
    let home = Home::new();
    let (_source, root, _) = allowing(&home);
    put(
        root.path(),
        SETTINGS,
        &json!({"permissions": {"allow": []}}).to_string(),
    );
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.lines(), [format!("missing: {}", allow(ALLOW))]);
}

/// The writing forms of the release: a version holds a dot (REL-3), and with two wildcards the rule does not match the
/// bare proposal, as `Bash(cardinal run release *)` would.
const ASK: &str = "Bash(cardinal run release *.*)";

/// `<entry>` of SYN-34 for an ask rule.
fn ask(rule: &str) -> String {
    format!("{SETTINGS} ask {rule}")
}

// spec: SYN-29
// spec: SYN-34
#[test]
fn sync_appends_a_managed_ask_rule_records_it_and_removes_it_once_dropped() {
    let home = Home::new();
    let (source, root) = with_fragment(&home, &json!({"permissions": {"ask": [ASK]}}));
    put(
        root.path(),
        SETTINGS,
        &json!({"permissions": {"ask": ["Bash(git push *)"]}}).to_string(),
    );
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome(&ask(ASK)), Some("added"));
    assert_eq!(
        settings(&root)["permissions"]["ask"],
        json!(["Bash(git push *)", ASK])
    );
    assert_eq!(home.ok(root.path(), &["check"]).stdout, "");
    put(source.path(), FRAGMENT, "{}");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome(&ask(ASK)), Some("removed"));
    assert_eq!(
        settings(&root)["permissions"]["ask"],
        json!(["Bash(git push *)"])
    );
}

/// The policy of this repository's fragment, not a requirement of `sync`.
#[test]
fn the_repository_settings_ask_before_the_release_writes_only() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fragment: Value =
        serde_json::from_str(&fs::read_to_string(repo.join(FRAGMENT)).unwrap()).unwrap();
    // `just release <version>` is matched as typed, not as the `cardinal run release` its recipe runs
    assert_eq!(
        fragment["permissions"]["ask"],
        json!([ASK, "Bash(just release *.*)"])
    );
    assert!(fragment["permissions"]["allow"]
        .as_array()
        .unwrap()
        .contains(&json!("Bash(cardinal run *)")));
}
