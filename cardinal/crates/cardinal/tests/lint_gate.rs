//! Behavior test of `cardinal tool lint-gate` with a lint command the project declares in `.claude/cardinal.toml`:
//! a fake linter, a shell script that logs its runs, in a git project with no `Cargo.toml`.
#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::{tempdir, TempDir};

struct Outcome {
    code: i32,
    stderr: String,
}

fn gate(project: &Path, stdin: &str) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "lint-gate"])
        .env("CLAUDE_PROJECT_DIR", project)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
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
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn git(project: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

/// A committed git project whose `.claude/cardinal.toml` holds the `[project]` lines `config`, with the fake linter
/// `lint.sh`: it logs each run to `lint.log` (ignored by git), prints `out.txt` and `err.txt` when they exist and
/// exits with the code in `code.txt`, 0 when it is absent; while `fix.txt` exists, it changes `a.ts` as a fixer does.
fn project(config: &str) -> TempDir {
    let root = tempdir().unwrap();
    let dir = root.path();
    git(dir, &["init", "-q", "."]);
    fs::create_dir_all(dir.join(".claude")).unwrap();
    fs::write(
        dir.join(".claude/cardinal.toml"),
        format!("[project]\n{config}\n"),
    )
    .unwrap();
    fs::write(
        dir.join("lint.sh"),
        "echo run >> lint.log\n[ -e fix.txt ] && echo '// fixed' >> a.ts\n[ -e out.txt ] && cat out.txt\n[ -e err.txt ] && cat err.txt >&2\n\
         exit $(cat code.txt 2>/dev/null || echo 0)\n",
    )
    .unwrap();
    fs::write(
        dir.join(".gitignore"),
        "lint.log\nout.txt\nerr.txt\ncode.txt\nfix.txt\n",
    )
    .unwrap();
    fs::write(dir.join("a.ts"), "let a = 1;\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "init"]);
    root
}

fn runs(project: &Path) -> usize {
    fs::read_to_string(project.join("lint.log")).map_or(0, |l| l.lines().count())
}

const TS: &str = "lint_command = \"sh lint.sh\"\nlint_files = [\"*.ts\"]";

// spec: CLG-1, CLG-7
#[test]
fn a_declared_lint_runs_only_for_a_change_of_a_matching_file() {
    let root = project(TS);
    let dir = root.path();
    fs::write(dir.join("README.md"), "edited\n").unwrap();
    let out = gate(dir, "{}");
    assert_eq!((out.code, runs(dir)), (0, 0), "{}", out.stderr);

    fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();
    let out = gate(dir, "{}");
    assert_eq!((out.code, runs(dir)), (0, 1), "{}", out.stderr);

    // the agent already retried once: let go
    fs::write(dir.join("code.txt"), "1").unwrap();
    fs::write(dir.join("a.ts"), "let a = 3;\n").unwrap();
    let out = gate(dir, "{\"stop_hook_active\":true}");
    assert_eq!((out.code, runs(dir)), (0, 1), "{}", out.stderr);
}

// spec: CLG-1, CLG-7
#[test]
fn without_lint_files_every_changed_file_runs_the_lint() {
    let root = project("lint_command = \"sh lint.sh\"");
    let dir = root.path();
    fs::write(dir.join("README.md"), "edited\n").unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 1);
}

// spec: CLG-1
#[test]
fn an_empty_or_blank_lint_command_disables_the_gate() {
    for command in ["", "  "] {
        let root = project(&format!(
            "lint_command = \"{command}\"\nlint_files = [\"*.ts\"]"
        ));
        let dir = root.path();
        fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();
        let out = gate(dir, "{}");
        assert_eq!((out.code, out.stderr.as_str()), (0, ""), "{command:?}");
    }
}

// spec: CLG-7
#[test]
fn a_configuration_that_does_not_parse_falls_back_to_clippy() {
    // clippy needs a `Cargo.toml`: none here, so nothing runs
    let root = project("lint_command = \"sh lint.sh\"\nlint_files = [");
    let dir = root.path();
    fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 0);
}

// spec: CLG-8
#[test]
fn a_failing_lint_is_reported_in_at_most_40_lines() {
    let root = project(TS);
    let dir = root.path();
    let out: String = (1..=70)
        .flat_map(|i| [format!("out {i}"), "\n\n".into()])
        .collect();
    fs::write(dir.join("out.txt"), out).unwrap();
    let err: String = (1..=30)
        .flat_map(|i| [format!("err {i}"), "\n".into()])
        .collect();
    fs::write(dir.join("err.txt"), err).unwrap();
    fs::write(dir.join("code.txt"), "1").unwrap();
    fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();

    let out = gate(dir, "{}");
    assert_eq!(out.code, 2, "{}", out.stderr);
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert_eq!(lines.len(), 42, "{}", out.stderr);
    assert!(lines[0].contains("`sh lint.sh`"), "{}", lines[0]);
    assert_eq!(lines[1], "out 1");
    assert_eq!(lines[40], "out 40");
    assert!(lines[41].starts_with("... 60 more lines"), "{}", lines[41]);

    // the standard error follows the standard output
    fs::write(dir.join("out.txt"), "out 1\n").unwrap();
    let out = gate(dir, "{}");
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert_eq!((lines.len(), lines[1], lines[2]), (32, "out 1", "err 1"));
}

// spec: CLG-4, CLG-5
#[test]
fn a_passing_lint_is_stamped_until_the_files_or_the_command_change() {
    let root = project(TS);
    let dir = root.path();
    fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();
    fs::write(dir.join("code.txt"), "1").unwrap();
    assert_eq!(gate(dir, "{}").code, 2);
    // a failing run records no stamp
    assert_eq!(gate(dir, "{}").code, 2);
    assert_eq!(runs(dir), 2);

    fs::remove_file(dir.join("code.txt")).unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 3, "an unchanged state runs no lint");

    fs::write(dir.join("a.ts"), "let a = 3;\n").unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 4, "a changed file runs the lint");

    fs::write(dir.join("b.ts"), "let b = 1;\n").unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 5, "a new untracked file runs the lint");

    fs::write(
        dir.join(".claude/cardinal.toml"),
        "[project]\nlint_command = \"sh ./lint.sh\"\nlint_files = [\"*.ts\"]\n",
    )
    .unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 6, "a changed command runs the lint");
}

// spec: CLG-4
#[test]
fn the_stamp_records_the_files_as_a_fixing_lint_leaves_them() {
    let root = project(TS);
    let dir = root.path();
    fs::write(dir.join("fix.txt"), "").unwrap();
    fs::write(dir.join("a.ts"), "let a = 2;\n").unwrap();
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(gate(dir, "{}").code, 0);
    assert_eq!(runs(dir), 1, "the fix of the passing run is in the stamp");
}

// spec: CLG-5, CLG-6
#[test]
fn a_project_in_a_subdirectory_lints_again_after_each_edit() {
    let root = project(TS);
    let dir = root.path();
    let web = dir.join("web");
    fs::create_dir_all(&web).unwrap();
    fs::copy(dir.join("lint.sh"), web.join("lint.sh")).unwrap();
    fs::write(web.join("b.ts"), "let b = 1;\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "web"]);

    fs::write(web.join("b.ts"), "let b = 2;\n").unwrap();
    assert_eq!(gate(&web, "{}").code, 0);
    fs::write(web.join("b.ts"), "let b = 3;\n").unwrap();
    assert_eq!(gate(&web, "{}").code, 0);
    assert_eq!(runs(&web), 2, "a new edit of a dirty file runs the lint");
}
