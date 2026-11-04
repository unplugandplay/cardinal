//! The default projects directory of the tools that read the transcripts (EVA-15), when the configuration directory
//! cannot be located or holds no `projects/`; `session-metrics` proves the default itself, the others share its
//! `projects_root`.

use std::process::{Command, Output};

/// `cardinal tool <name> <args>` with neither `CLAUDE_CONFIG_DIR` nor `HOME` set to a value.
fn unlocated(name: &str, args: &[&std::path::Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .arg("tool")
        .arg(name)
        .args(args)
        .env("CLAUDE_CONFIG_DIR", "")
        .env_remove("HOME")
        .output()
        .unwrap()
}

// spec: EVA-15
#[test]
fn the_transcript_tools_fail_naming_both_variables() {
    for name in [
        "session-metrics",
        "rule-breaches",
        "fixed-context",
        "audit-sources",
    ] {
        let out = unlocated(name, &[]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{name}");
        assert!(
            stderr.contains(
                "the configuration directory cannot be located (set HOME or CLAUDE_CONFIG_DIR)"
            ),
            "{name}: {stderr}"
        );
    }
}

// spec: EVA-15
#[test]
fn delivered_quality_runs_without_the_transcripts() {
    let repo = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(repo.path())
        .status()
        .unwrap()
        .success());
    let out = unlocated("delivered-quality", &[repo.path()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `cardinal tool <name> <args>` with `config` as the configuration directory and an empty home directory.
fn configured(name: &str, args: &[&std::path::Path], config: &std::path::Path) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .arg("tool")
        .arg(name)
        .args(args)
        .env("CLAUDE_CONFIG_DIR", config)
        .env("HOME", home.path())
        .output()
        .unwrap()
}

// spec: EVA-15
#[test]
fn the_transcript_tools_warn_when_the_default_projects_directory_cannot_be_read() {
    let repo = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(repo.path())
        .status()
        .unwrap()
        .success());
    let absent = tempfile::tempdir().unwrap();
    let file = tempfile::tempdir().unwrap();
    std::fs::write(file.path().join("projects"), "").unwrap();
    for config in [absent.path(), file.path()] {
        let root = config.join("projects");
        for (name, args) in [
            ("session-metrics", vec![]),
            ("rule-breaches", vec![]),
            ("fixed-context", vec![]),
            ("audit-sources", vec![]),
            ("delivered-quality", vec![repo.path()]),
        ] {
            let out = configured(name, &args, config);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(out.status.success(), "{name}: {stderr}");
            assert!(
                stderr.contains(&format!(
                    "the projects directory {} cannot be read (",
                    root.display()
                )),
                "{name}: {stderr}"
            );
        }
    }
}

// spec: EVA-15
#[test]
fn session_metrics_prints_its_header_only_when_the_default_projects_directory_is_missing() {
    let config = tempfile::tempdir().unwrap();
    let out = configured("session-metrics", &[], config.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.starts_with("session\tproject\t"), "{stdout}");
}
