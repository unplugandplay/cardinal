//! Behavior test of `cardinal tool format-code` with a format command the project declares in
//! `.claude/cardinal.toml`: a fake formatter, a shell script that logs its directory and arguments.
#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::{tempdir, TempDir};

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn hook(project: &Path, stdin: &str) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "format-code"])
        .env("CLAUDE_PROJECT_DIR", project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Outcome {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A git project whose `.claude/cardinal.toml` holds the `[project]` lines `config`, with the fake formatter
/// `fmt.sh`: it appends its directory and arguments to `fmt.log`, prints to both streams and exits 1.
fn project(config: &str) -> TempDir {
    let root = tempdir().unwrap();
    let dir = root.path();
    let ok = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    fs::create_dir_all(dir.join(".claude")).unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join(".claude/cardinal.toml"),
        format!("[project]\n{config}\n"),
    )
    .unwrap();
    fs::write(
        dir.join("fmt.sh"),
        "echo \"$PWD $*\" >> fmt.log\necho noise\necho noise >&2\nexit 1\n",
    )
    .unwrap();
    root
}

/// The event of an edit of `file`.
fn edit(file: &Path) -> String {
    format!(
        "{{\"tool_input\":{{\"file_path\":\"{}\"}}}}",
        file.display()
    )
}

fn log(project: &Path) -> String {
    fs::read_to_string(project.join("fmt.log")).unwrap_or_default()
}

const TS: &str = "format_command = \"sh fmt.sh\"\nformat_files = [\"*.ts\"]";

// spec: FMT-1, FMT-3, FMT-4
#[test]
fn a_declared_format_runs_on_the_edited_file_from_the_root_silently() {
    let root = project(TS);
    let dir = root.path().canonicalize().unwrap();
    let file = dir.join("src/a.ts");
    fs::write(&file, "let a=1\n").unwrap();

    let out = hook(&dir, &edit(&file));
    assert_eq!(
        (out.code, out.stdout.as_str(), out.stderr.as_str()),
        (0, "", "")
    );
    assert_eq!(log(&dir), format!("{} {}\n", dir.display(), file.display()));
}

// spec: FMT-2
#[test]
fn a_declared_format_skips_the_files_it_does_not_name() {
    let root = project(TS);
    let dir = root.path();
    fs::write(dir.join("README.md"), "x\n").unwrap();
    assert_eq!(hook(dir, &edit(&dir.join("README.md"))).code, 0);
    assert_eq!(hook(dir, "{\"tool_input\":{}}").code, 0);
    assert_eq!(log(dir), "");
}

// spec: FMT-1
#[test]
fn without_format_files_every_edited_file_is_formatted() {
    let root = project("format_command = \"sh fmt.sh\"");
    let dir = root.path();
    fs::write(dir.join("README.md"), "x\n").unwrap();
    assert_eq!(hook(dir, &edit(&dir.join("README.md"))).code, 0);
    assert_eq!(log(dir).lines().count(), 1);
}

// spec: FMT-2
#[test]
fn an_empty_or_blank_format_command_formats_nothing() {
    for command in ["", "  "] {
        let root = project(&format!("format_command = \"{command}\""));
        let dir = root.path();
        fs::write(dir.join("fmt.sh"), "#!/bin/sh\necho ran > ran.log\n").unwrap();
        fs::set_permissions(dir.join("fmt.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        let out = hook(dir, &edit(&dir.join("fmt.sh")));
        assert_eq!((out.code, out.stderr.as_str()), (0, ""), "{command:?}");
        assert!(
            !dir.join("ran.log").exists(),
            "{command:?} ran the edited file"
        );
    }
}

// spec: FMT-2
#[test]
fn a_declared_format_leaves_the_claude_directories_alone() {
    let root = project("format_command = \"sh fmt.sh\"");
    let dir = root.path();
    fs::create_dir_all(dir.join(".claude/skills")).unwrap();
    fs::write(dir.join(".claude/skills/x.md"), "x\n").unwrap();
    assert_eq!(hook(dir, &edit(&dir.join(".claude/skills/x.md"))).code, 0);
    assert_eq!(log(dir), "");
}

// spec: FMT-1
#[test]
fn a_configuration_that_does_not_parse_falls_back_to_cargo_fmt() {
    // the default formats only `.rs` files: the `.ts` edit runs nothing
    let root = project("format_command = \"sh fmt.sh\"\nformat_files = [");
    let dir = root.path();
    fs::write(dir.join("src/a.ts"), "x\n").unwrap();
    assert_eq!(hook(dir, &edit(&dir.join("src/a.ts"))).code, 0);
    assert_eq!(log(dir), "");
}
