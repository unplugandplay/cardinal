//! The features a project turns off, run as the binary on temp dirs (`specs/features.md`).
#![cfg(unix)]

mod common;

use common::{get, project, put, snapshot, Home, CONFIG, LOCK};
use serde_json::{json, Value};
use tempfile::TempDir;

const SETTINGS: &str = ".claude/settings.json";
const FRAGMENT: &str = "entries/.claude/settings.json";
const HINT: &str = "cardinal run hint";
const GUARD: &str = "cardinal run guard";
/// A source feature `hint` made of `sub/b.txt` and the `Stop` hook running `HINT`.
const FEATURES: &str =
    "[hint]\nfiles = [\"sub/b.txt\"]\nentries = [\"hook Stop cardinal run hint\"]\n";

fn hook(command: &str) -> Value {
    json!({"type": "command", "command": command})
}

/// The project of `common::project` whose source runs `HINT` and `GUARD` at `Stop` and declares `FEATURES`, synced
/// once with every feature on.
fn with_features(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    let fragment = json!({"hooks": {"Stop": [{"hooks": [hook(HINT), hook(GUARD)]}]}});
    put(source.path(), FRAGMENT, &fragment.to_string());
    put(source.path(), "features.toml", FEATURES);
    put(root.path(), SETTINGS, "{}");
    home.ok(root.path(), &["sync"]);
    (source, root)
}

/// Set the `disabled` list of the `[cardinal]` table of the project's configuration to `value`.
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

fn stop_commands(root: &TempDir) -> Vec<String> {
    let settings: Value = serde_json::from_str(&get(root.path(), SETTINGS).unwrap()).unwrap();
    settings["hooks"]["Stop"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|group| group["hooks"].as_array().unwrap().clone())
        .map(|h| h["command"].as_str().unwrap().to_string())
        .collect()
}

// spec: FEA-1, FEA-2, FEA-3
#[test]
fn sync_removes_the_files_and_entries_of_a_feature_turned_off() {
    let home = Home::new();
    let (_source, root) = with_features(&home);
    assert_eq!(stop_commands(&root), [HINT, GUARD]);
    disable(&root, "[\"hint\"]");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("sub/b.txt"), Some("removed"), "{run:?}");
    assert!(get(root.path(), "sub/b.txt").is_none());
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert_eq!(stop_commands(&root), [GUARD]);
    let lock = get(root.path(), LOCK).unwrap();
    assert!(
        !lock.contains("sub/b.txt") && !lock.contains(HINT),
        "{lock}"
    );
}

// spec: FEA-3
#[test]
fn sync_adds_back_a_feature_turned_on_again() {
    let home = Home::new();
    let (_source, root) = with_features(&home);
    disable(&root, "[\"hint\"]");
    home.ok(root.path(), &["sync"]);
    disable(&root, "[]");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("sub/b.txt"), Some("added"), "{run:?}");
    assert_eq!(stop_commands(&root), [GUARD, HINT]);
}

// spec: FEA-3
#[test]
fn sync_keeps_an_edited_file_of_a_feature_turned_off() {
    let home = Home::new();
    let (_source, root) = with_features(&home);
    put(root.path(), "sub/b.txt", "mine");
    disable(&root, "[\"hint\"]");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("sub/b.txt"), Some("kept"), "{run:?}");
    assert_eq!(get(root.path(), "sub/b.txt").as_deref(), Some("mine"));
}

// spec: FEA-3
#[test]
fn a_first_sync_never_installs_a_feature_turned_off() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(
        source.path(),
        "features.toml",
        "[hint]\nfiles = [\"sub/b.txt\"]\n",
    );
    disable(&root, "[\"hint\"]");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("sub/b.txt"), None, "{run:?}");
    assert!(get(root.path(), "sub/b.txt").is_none());
}

// spec: FEA-4
#[test]
fn check_reports_nothing_of_a_feature_turned_off() {
    let home = Home::new();
    let (_source, root) = with_features(&home);
    disable(&root, "[\"hint\"]");
    home.ok(root.path(), &["sync"]);
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(!run.stdout.contains("missing"), "{run:?}");
}

// spec: FEA-5
#[test]
fn sync_refuses_an_unknown_feature_and_writes_nothing() {
    let home = Home::new();
    let (_source, root) = with_features(&home);
    disable(&root, "[\"hnit\"]");
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        assert!(
            run.stderr
                .contains(".claude/cardinal.toml: disabled names the unknown feature hnit; the source declares hint"),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: FEA-6
#[test]
fn sync_refuses_a_feature_naming_what_the_source_does_not_manage() {
    let cases = [
        (
            "[hint]\nfiles = [\"gone.txt\"]\n",
            "the feature hint names the file gone.txt",
        ),
        (
            "[hint]\nentries = [\"hook Stop cardinal run gone\"]\n",
            "the feature hint names the entry hook Stop cardinal run gone",
        ),
    ];
    for (features, said) in cases {
        let home = Home::new();
        let (source, root) = with_features(&home);
        put(source.path(), "features.toml", features);
        let before = snapshot(root.path());
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(3), "{run:?}");
        assert!(run.stderr.contains(said), "{run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: FEA-7
#[test]
fn sync_refuses_a_features_file_or_a_disabled_key_of_another_shape() {
    for features in [
        "hint = 1\n",
        "[hint]\nfiles = \"sub/b.txt\"\n",
        "[hint]\nhooks = []\n",
        "[hint]\nneeds = \"other\"\n",
        "[hint\n",
    ] {
        let home = Home::new();
        let (source, root) = with_features(&home);
        put(source.path(), "features.toml", features);
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(3), "{features}: {run:?}");
        assert!(run.stderr.contains("features.toml"), "{features}: {run:?}");
    }
    for disabled in ["\"hint\"", "[1]"] {
        let home = Home::new();
        let (_source, root) = with_features(&home);
        disable(&root, disabled);
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(3), "{disabled}: {run:?}");
        assert!(run.stderr.contains("disabled"), "{disabled}: {run:?}");
    }
}

// spec: FEA-15
#[test]
fn sync_never_seeds_the_path_of_a_file_turned_off() {
    let home = Home::new();
    let (source, root) = with_features(&home);
    put(source.path(), "seed/sub/b.txt", "seeded");
    disable(&root, "[\"hint\"]");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("sub/b.txt"), Some("removed"), "{run:?}");
    assert!(get(root.path(), "sub/b.txt").is_none(), "{run:?}");
}

// spec: FEA-17
#[test]
fn sync_refuses_a_feature_needing_one_the_source_does_not_declare() {
    let home = Home::new();
    let (source, root) = with_features(&home);
    put(
        source.path(),
        "features.toml",
        &format!("{FEATURES}needs = [\"gone\"]\n"),
    );
    let before = snapshot(root.path());
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(
        run.stderr
            .contains("the feature hint needs the feature gone, which the source does not declare"),
        "{run:?}"
    );
    assert_eq!(snapshot(root.path()), before);
}

/// The features `a` and `b` of no content, whose `needs` are `a_needs` and `b_needs`.
fn needing(a_needs: &str, b_needs: &str) -> String {
    format!("[a]\nneeds = {a_needs}\n[b]\nneeds = {b_needs}\n")
}

// spec: FEA-18
#[test]
fn sync_refuses_to_turn_off_a_feature_one_left_on_needs() {
    for (features, disabled, needed, by) in [
        (needing("[\"b\"]", "[]"), "[\"b\"]", "b", "a"),
        (needing("[\"b\"]", "[\"a\"]"), "[\"a\"]", "a", "b"),
    ] {
        let home = Home::new();
        let (source, root) = with_features(&home);
        put(source.path(), "features.toml", &features);
        disable(&root, disabled);
        let before = snapshot(root.path());
        for args in [&["sync"][..], &["sync", "--dry-run"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(3), "{features}: {run:?}");
            let said =
                format!(".claude/cardinal.toml: disabled turns off {needed}, which {by} needs");
            assert!(run.stderr.contains(&said), "{features}: {run:?}");
            assert_eq!(snapshot(root.path()), before);
        }
    }
}

// spec: FEA-17, FEA-18
#[test]
fn sync_judges_the_needs_of_the_features_left_on_only() {
    for (features, disabled) in [
        (needing("[\"b\"]", "[]"), "[\"a\", \"b\"]"),
        (needing("[\"b\"]", "[\"a\"]"), "[\"a\", \"b\"]"),
        (needing("[\"a\"]", "[]"), "[\"b\"]"),
        (needing("[\"b\"]", "[]"), "[]"),
    ] {
        let home = Home::new();
        let (source, root) = with_features(&home);
        put(source.path(), "features.toml", &features);
        disable(&root, disabled);
        let run = home.harness(root.path(), &["sync"]);
        assert_eq!(run.code, Some(0), "{features} {disabled}: {run:?}");
    }
}
