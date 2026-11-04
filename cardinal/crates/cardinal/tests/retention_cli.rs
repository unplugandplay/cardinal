//! The retention warning of the binary (SYN-28): once after `check`, `sync` and `update`, never after `init`.
#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::Command;

use common::{output, project, put, source, Home, Run};

const ADVICE: &str = "(cleanupPeriodDays); set it to at least 365 in";

/// The lines of stderr about the retention of the transcripts.
fn retention(run: &Run) -> Vec<&str> {
    run.stderr
        .lines()
        .filter(|l| l.contains("cleanupPeriodDays") || l.contains("transcripts"))
        .collect()
}

fn user_warning(days: &str, path: &Path) -> String {
    format!(
        "warning: the transcripts are deleted after {days} days {ADVICE} {}",
        path.display()
    )
}

/// `cardinal check --root <root>` as `cmd` sets it up.
fn check(home: &Home, root: &Path, setup: impl FnOnce(&mut Command)) -> Run {
    let mut cmd = home.command();
    cmd.args(["check", "--root"]).arg(root);
    setup(&mut cmd);
    output(cmd, "")
}

// spec: SYN-28
#[test]
fn the_binary_warns_of_the_retention_once_after_check_sync_and_update_never_after_init() {
    let home = Home::new();
    let user = home.path().join("config/settings.json");
    put(home.path(), "config/settings.json", "{}");
    let source = source();
    put(source.path(), "files/a.txt", "a");
    let root = tempfile::tempdir().unwrap();
    let init = home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    assert!(retention(&init).is_empty(), "{init:?}");

    let warning = user_warning("30", &user);
    for args in [&["sync"][..], &["sync", "--dry-run"], &["check"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(retention(&run), [warning.as_str()], "{args:?}: {run:?}");
    }
    // a local source has no revision: `update` fails, and says nothing of the retention
    let update = home.harness(root.path(), &["update"]);
    assert!(
        update.stderr.contains("error:") && retention(&update).is_empty(),
        "{update:?}"
    );
}

// spec: SYN-28
#[test]
fn warns_when_the_user_settings_keep_the_transcripts_less_than_a_year() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let user = home.path().join("config/settings.json");
    for (text, days) in [
        (None, "30"),
        (Some("{}"), "30"),
        (Some(r#"{"cleanupPeriodDays": "long"}"#), "30"),
        (Some(r#"{"cleanupPeriodDays": 90}"#), "90"),
    ] {
        match text {
            Some(text) => put(home.path(), "config/settings.json", text),
            None => std::fs::remove_file(&user).unwrap(),
        }
        let run = home.harness(root.path(), &["check"]);
        assert_eq!(run.code, Some(0), "{text:?}: {run:?}");
        assert_eq!(
            retention(&run),
            [user_warning(days, &user).as_str()],
            "{text:?}"
        );
    }
    for text in [
        r#"{"cleanupPeriodDays": 365}"#,
        r#"{"cleanupPeriodDays": 3650}"#,
    ] {
        put(home.path(), "config/settings.json", text);
        let run = home.harness(root.path(), &["check"]);
        assert!(retention(&run).is_empty(), "{text}: {run:?}");
    }
}

// spec: SYN-28
#[test]
fn the_user_settings_are_in_the_config_dir_or_else_in_the_home() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let in_home = home.path().join(".claude/settings.json");
    put(home.path(), ".claude/settings.json", "{}");
    for config_dir in [None, Some("")] {
        let run = check(&home, root.path(), |cmd| {
            match config_dir {
                Some(dir) => cmd.env("CLAUDE_CONFIG_DIR", dir),
                None => cmd.env_remove("CLAUDE_CONFIG_DIR"),
            };
        });
        assert_eq!(
            retention(&run),
            [user_warning("30", &in_home).as_str()],
            "{config_dir:?}"
        );
    }
}

// spec: SYN-28
#[test]
fn says_when_the_user_settings_cannot_be_parsed_rather_than_guess_the_retention() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(home.path(), "config/settings.json", "{,}");
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(
        retention(&run),
        [format!(
            "warning: {} cannot be read or is not valid JSON; Claude Code pauses the cleanup of the transcripts until it is fixed",
            home.path().join("config/settings.json").display()
        )
        .as_str()]
    );
}

// spec: SYN-28
#[test]
fn says_when_the_user_settings_cannot_be_located() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    for unset in [
        |cmd: &mut Command| {
            cmd.env_remove("CLAUDE_CONFIG_DIR").env_remove("HOME");
        },
        |cmd: &mut Command| {
            cmd.env("CLAUDE_CONFIG_DIR", "").env("HOME", "");
        },
    ] {
        let run = check(&home, root.path(), unset);
        assert_eq!(run.code, Some(0), "{run:?}");
        assert_eq!(
            retention(&run),
            ["warning: the user settings cannot be located (set HOME or CLAUDE_CONFIG_DIR); the retention of the transcripts is not checked"]
        );
    }
}

// spec: SYN-28
#[test]
fn warns_of_each_project_settings_file_that_lowers_the_retention() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 30}"#,
    );
    put(
        root.path(),
        ".claude/settings.local.json",
        r#"{"cleanupPeriodDays": 7}"#,
    );
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(
        retention(&run),
        [
            "warning: .claude/settings.json sets cleanupPeriodDays to 30 days, below 365",
            "warning: .claude/settings.local.json sets cleanupPeriodDays to 7 days, below 365",
        ]
    );
    // a project file that raises it, leaves it unset or is not valid JSON says nothing of the retention
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 400}"#,
    );
    put(root.path(), ".claude/settings.local.json", "{,}");
    let run = home.harness(root.path(), &["check"]);
    assert!(retention(&run).is_empty(), "{run:?}");
}

// spec: SYN-28
#[test]
fn says_nothing_of_the_retention_under_ci() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(home.path(), "config/settings.json", "{}");
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 30}"#,
    );
    for args in [&["check"][..], &["sync"]] {
        let mut cmd = home.command();
        cmd.args(args).arg("--root").arg(root.path()).env("CI", "1");
        let run = output(cmd, "");
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        assert!(retention(&run).is_empty(), "{args:?}: {run:?}");
    }
}

// spec: FEA-28
#[test]
fn no_retention_warning_when_the_project_turns_off_the_measure_of_the_harness() {
    let home = Home::new();
    put(home.path(), "config/settings.json", "{}");
    let source = source();
    put(source.path(), "files/a.txt", "a");
    put(source.path(), "features.toml", "[harness-measure]\n");
    let root = tempfile::tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    let config = std::fs::read_to_string(root.path().join(".claude/cardinal.toml")).unwrap();
    let config = config.replacen(
        "[cardinal]\n",
        "[cardinal]\ndisabled = [\"harness-measure\"]\n",
        1,
    );
    put(root.path(), ".claude/cardinal.toml", &config);
    for args in [&["sync"][..], &["sync", "--dry-run"], &["check"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        assert!(retention(&run).is_empty(), "{args:?}: {run:?}");
    }
    // the project's own settings sweep the transcripts of every project: still warned of
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 7}"#,
    );
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(
        retention(&run),
        ["warning: .claude/settings.json sets cleanupPeriodDays to 7 days, below 365"],
        "{run:?}"
    );
}
