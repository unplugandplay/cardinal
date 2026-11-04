//! Behavior test for the hook `cardinal tool read-hint` (`specs/read-hint.md`): a `PreToolUse` event of `Read` on
//! stdin, files of a temp dir, and a temp `TMPDIR` (no `XDG_RUNTIME_DIR`, `USER` `tester`) for the record of the
//! sessions sent back.

mod common;

use common::{output, put, Run};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

/// The directory of the records under the test's `TMPDIR`.
const RECORDS: &str = "cardinal-read-hint-tester";

fn hook_raw(tmp: &Path, stdin: &str) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.args(["tool", "read-hint"])
        .env("TMPDIR", tmp)
        .env("USER", "tester")
        .env_remove("XDG_RUNTIME_DIR");
    output(cmd, stdin)
}

fn hook(tmp: &Path, event: &Value) -> Run {
    hook_raw(tmp, &event.to_string())
}

/// A `PreToolUse` event of `Read` by `session` with `input`.
fn read(session: &str, input: &Value) -> Value {
    json!({"hook_event_name": "PreToolUse", "session_id": session, "tool_name": "Read", "tool_input": input})
}

/// A file named `name` in `dir` of `lines` lines of `width` bytes, newline included.
fn file(dir: &Path, name: &str, lines: usize, width: usize) -> PathBuf {
    put(
        dir,
        name,
        &format!("{}\n", "x".repeat(width - 1)).repeat(lines),
    );
    dir.join(name)
}

fn silent_pass(run: &Run) -> bool {
    run.code == Some(0) && run.stderr.is_empty() && run.stdout.is_empty()
}

/// The reason of a deny, or a failure when the run is not one.
fn denied(run: &Run) -> String {
    assert_eq!(run.code, Some(0), "{run:?}");
    let output: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|_| panic!("{run:?}"));
    let specific = &output["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PreToolUse", "{run:?}");
    assert_eq!(specific["permissionDecision"], "deny", "{run:?}");
    specific["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .to_string()
}

// spec: RDH-1, RDH-3, RDH-7
#[test]
fn a_whole_read_of_a_large_file_is_sent_back_once() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    // 300 lines of 100 bytes and their 7 bytes of line number
    let path = file(dir.path(), "big.rs", 300, 100);
    let event = read("s1", &json!({"file_path": path}));
    let reason = denied(&hook(tmp.path(), &event));
    for word in ["32100 bytes", "`offset`", "`limit`", "`Grep`", "same call"] {
        assert!(reason.contains(word), "{word}: {reason}");
    }
    assert!(silent_pass(&hook(tmp.path(), &event)), "the repeat");
}

// spec: RDH-2, RDH-7
#[test]
fn a_bounded_small_missing_or_rendered_read_passes() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    let big = file(dir.path(), "big.md", 300, 100);
    // 233 lines of 107 bytes: 24,931, at most the threshold
    let small = file(dir.path(), "small.md", 233, 100);
    // one line cut at 2,000 characters, and 3,000 lines of which 2,000 are read
    let minified = file(dir.path(), "app.min.js", 1, 200_000);
    let long = file(dir.path(), "long.txt", 3_000, 5);
    // over the size Claude Code reads whole
    let journal = file(dir.path(), "big.log", 4_000, 100);
    let shot = file(dir.path(), "shot.PNG", 300, 100);
    let pdf = file(dir.path(), "doc.pdf", 300, 100);
    let notebook = file(dir.path(), "a.ipynb", 300, 100);
    let locked = file(dir.path(), "locked.md", 300, 100);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    for (n, input) in [
        json!({"file_path": big, "limit": 200}),
        json!({"file_path": big, "offset": 10}),
        json!({"file_path": small}),
        json!({"file_path": minified}),
        json!({"file_path": long}),
        json!({"file_path": journal}),
        json!({"file_path": notebook}),
        json!({"file_path": locked}),
        json!({"file_path": dir.path().join("missing.md")}),
        json!({"file_path": dir.path()}),
        json!({"file_path": shot}),
        json!({"file_path": pdf}),
    ]
    .into_iter()
    .enumerate()
    {
        let run = hook(tmp.path(), &read(&format!("s{n}"), &input));
        assert!(silent_pass(&run), "{input}: {run:?}");
    }
}

// spec: RDH-3, RDH-5
#[test]
fn the_record_is_per_session_under_the_temporary_directory() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    let a = file(dir.path(), "a.rs", 300, 100);
    let b = file(dir.path(), "b.rs", 300, 100);
    denied(&hook(tmp.path(), &read("s1", &json!({"file_path": a}))));
    denied(&hook(tmp.path(), &read("s1", &json!({"file_path": b}))));
    denied(&hook(tmp.path(), &read("s2", &json!({"file_path": a}))));
    assert!(silent_pass(&hook(
        tmp.path(),
        &read("s1", &json!({"file_path": a}))
    )));
    let records = tmp.path().join(RECORDS);
    assert_eq!(
        fs::read_dir(&records).unwrap().count(),
        2,
        "one file per session"
    );
    let mode = fs::metadata(&records).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
}

// spec: RDH-5
#[test]
fn the_record_goes_to_the_runtime_directory_when_set() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    let runtime = tempdir().unwrap();
    let path = file(dir.path(), "a.rs", 300, 100);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.args(["tool", "read-hint"])
        .env("TMPDIR", tmp.path())
        .env("XDG_RUNTIME_DIR", runtime.path());
    denied(&output(
        cmd,
        &read("s1", &json!({"file_path": path})).to_string(),
    ));
    assert!(runtime.path().join("cardinal-read-hint/s1").is_file());
    assert!(!tmp.path().join(RECORDS).exists());
}

// spec: RDH-4, RDH-5
#[test]
fn a_record_directory_others_may_enter_is_not_used() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    let path = file(dir.path(), "a.rs", 300, 100);
    let records = tmp.path().join(RECORDS);
    fs::create_dir(&records).unwrap();
    fs::set_permissions(&records, fs::Permissions::from_mode(0o777)).unwrap();
    let run = hook(tmp.path(), &read("s1", &json!({"file_path": path})));
    assert!(silent_pass(&run), "{run:?}");
}

// spec: RDH-4
#[test]
fn another_event_or_an_unrecordable_one_passes() {
    let dir = tempdir().unwrap();
    let tmp = tempdir().unwrap();
    let path = file(dir.path(), "big.rs", 300, 100);
    let mut bash = read("s1", &json!({"file_path": path}));
    bash["tool_name"] = json!("Bash");
    let mut post = read("s1", &json!({"file_path": path}));
    post["hook_event_name"] = json!("PostToolUse");
    let mut anonymous = read("s1", &json!({"file_path": path}));
    anonymous.as_object_mut().unwrap().remove("session_id");
    for event in [bash, post, anonymous] {
        assert!(silent_pass(&hook(tmp.path(), &event)), "{event}");
    }
    assert!(silent_pass(&hook_raw(tmp.path(), "not json")));
    // a `TMPDIR` that is a file: the record cannot be written
    let blocked = file(dir.path(), "tmp-file", 1, 1);
    let run = hook(&blocked, &read("s1", &json!({"file_path": path})));
    assert!(silent_pass(&run), "{run:?}");
}
