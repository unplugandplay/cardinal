//! The exit code of the binary on an error of `init`, `sync`, `update` and `check` (SYN-37): 3, never the 2 of a
//! refusal.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

/// The exit code and stderr of `cardinal <args>` run in `root`.
fn harness(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("CLAUDE_CONFIG_DIR", root)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

// spec: SYN-37
#[test]
fn an_error_of_a_subcommand_exits_3_with_the_error_on_stderr() {
    let root = tempdir().unwrap();
    // a configuration with no source; `init` refuses to overwrite it.
    fs::create_dir_all(root.path().join(".claude")).unwrap();
    fs::write(root.path().join(".claude/cardinal.toml"), "[cardinal]\n").unwrap();
    for args in [
        &["sync"][..],
        &["update"][..],
        &["check"][..],
        &["init", "/a/source"][..],
    ] {
        let (code, stderr) = harness(root.path(), args);
        assert_eq!(code, Some(3), "{args:?}: {stderr}");
        assert!(stderr.contains("error:"), "{args:?}: {stderr}");
    }
}

// spec: SYN-37
#[test]
fn a_drift_exits_1_not_3() {
    let source = tempdir().unwrap();
    fs::create_dir_all(source.path().join("files")).unwrap();
    fs::write(source.path().join("files/managed.txt"), "managed\n").unwrap();
    fs::write(source.path().join("cardinal-format"), "5\n").unwrap();
    let root = tempdir().unwrap();
    let source = source.path().to_str().unwrap();
    assert_eq!(harness(root.path(), &["init", source]).0, Some(0));
    assert_eq!(harness(root.path(), &["sync"]).0, Some(0));
    fs::write(root.path().join("managed.txt"), "edited\n").unwrap();
    let (code, stderr) = harness(root.path(), &["check"]);
    assert_eq!(code, Some(1), "{stderr}");
}
