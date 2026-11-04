//! Behavior test for the Rust hooks `cardinal tool lint-gate` and `format-code`: runs them against a throwaway Cargo
//! project. A hook is named by a path holding its tool name.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Outcome {
    code: i32,
    stderr: String,
}

fn run_hook(script: &Path, project: &Path, stdin: &str) -> Outcome {
    run_hook_with(script, project, stdin, &[])
}

fn run_hook_with(script: &Path, project: &Path, stdin: &str, env: &[(&str, String)]) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .arg("tool")
        .arg(script)
        .env("CLAUDE_PROJECT_DIR", project)
        .envs(env.iter().map(|(k, v)| (k, v)))
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

fn sh(project: &Path, program: &str, args: &[&str]) {
    let ok = Command::new(program)
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(ok, "{program} {args:?} failed");
}

fn commit_all(project: &Path, msg: &str) {
    sh(project, "git", &["add", "-A"]);
    sh(
        project,
        "git",
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            msg,
        ],
    );
}

/// A fake `cargo` first on the `PATH` of `env`: it counts its runs in `log`, passes while `pass` exists and fails with
/// the diagnostics of `out` otherwise.
struct FakeCargo {
    out: PathBuf,
    log: PathBuf,
    pass: PathBuf,
    env: [(&'static str, String); 1],
}

impl FakeCargo {
    fn new(project: &Path) -> Self {
        let bin = project.join("fake-bin");
        fs::create_dir_all(&bin).unwrap();
        let fake = FakeCargo {
            out: project.join("fake-out.txt"),
            log: project.join("fake-log.txt"),
            pass: project.join("fake-pass"),
            env: [(
                "PATH",
                format!(
                    "{}:{}",
                    bin.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )],
        };
        let cargo = bin.join("cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\necho run >> '{}'\n[ -e '{}' ] && exit 0\ncat '{}' >&2\nexit 101\n",
                fake.log.display(),
                fake.pass.display(),
                fake.out.display()
            ),
        )
        .unwrap();
        sh(project, "chmod", &["+x", cargo.to_str().unwrap()]);
        fake
    }

    fn runs(&self) -> usize {
        fs::read_to_string(&self.log).map_or(0, |l| l.lines().count())
    }
}

// spec: CLG-1, CLG-2, CLG-3, CLG-4, CLG-5, CLG-6
/// The clippy gate in one Cargo project, in order: the stamp of a passing run decides the runs that follow.
fn clippy_gate_checks(gate: &Path, project: &Path) -> Vec<(&'static str, bool)> {
    let main_rs = project.join("src/main.rs");
    let mut checks = Vec::new();
    let fake = FakeCargo::new(project);
    fs::write(project.join(".gitignore"), "/target\n/fake-*\n").unwrap();
    commit_all(project, "ignore");

    checks.push((
        "gate passes with no Rust change",
        run_hook_with(gate, project, "{}", &fake.env).code == 0,
    ));
    checks.push(("gate runs no clippy with no Rust change", fake.runs() == 0));
    fs::create_dir_all(project.join("tools/.claude/hooks")).unwrap();
    fs::write(project.join("tools/.claude/hooks/hook.rs"), "").unwrap();
    checks.push((
        "gate runs no clippy for a changed harness script only",
        run_hook_with(gate, project, "{}", &fake.env).code == 0 && fake.runs() == 0,
    ));
    fs::remove_dir_all(project.join("tools")).unwrap();

    fs::write(&main_rs, "fn main() {\n    let unused = 1;\n}\n").unwrap();
    let out = run_hook(gate, project, "{}");
    checks.push(("gate blocks on a clippy warning", out.code == 2));
    checks.push((
        "gate reports the file, line and message",
        out.stderr.contains("src/main.rs:2:9:") && out.stderr.contains("unused"),
    ));
    checks.push((
        "gate reports no build progress",
        !out.stderr.contains("Checking")
            && !out.stderr.contains("Compiling")
            && !out.stderr.contains("could not compile"),
    ));

    checks.push((
        "gate lets go when stop_hook_active",
        run_hook_with(gate, project, "{\"stop_hook_active\":true}", &fake.env).code == 0
            && fake.runs() == 0,
    ));
    checks.extend(stamp_checks(gate, project, &fake));
    checks.extend(diagnostic_checks(gate, project, &fake));
    checks.extend(worktree_gate_checks(gate, project, &fake));
    checks
}

/// The gate's report of many diagnostics, and the runs its stamp skips or not.
fn stamp_checks(gate: &Path, project: &Path, fake: &FakeCargo) -> Vec<(&'static str, bool)> {
    let main_rs = project.join("src/main.rs");
    let mut checks = Vec::new();
    let many: String = (1..=45)
        .map(|i| format!("src/main.rs:{i}:1: error: problem {i}\n"))
        .collect::<Vec<_>>()
        .concat();
    fs::write(
        &fake.out,
        format!("    Checking hooktest v0.1.0\n{many}error: could not compile `hooktest`\n"),
    )
    .unwrap();
    let out = run_hook_with(gate, project, "{}", &fake.env);
    let reported = out
        .stderr
        .lines()
        .filter(|l| l.starts_with("src/main.rs:"))
        .count();
    checks.push((
        "gate reports at most 40 diagnostics",
        out.code == 2 && reported == 40,
    ));
    checks.push((
        "gate says how many it left out",
        out.stderr.contains("5 more"),
    ));

    let before = fake.runs();
    run_hook_with(gate, project, "{}", &fake.env);
    checks.push((
        "gate runs clippy again after a failure with nothing changed",
        fake.runs() == before + 1,
    ));

    fs::write(&main_rs, "fn main() {}\n// edited\n").unwrap();
    checks.push((
        "gate passes once fixed",
        run_hook(gate, project, "{}").code == 0,
    ));
    let before = fake.runs();
    fs::write(project.join("README.md"), "edited\n").unwrap();
    checks.push((
        "gate skips clippy while the Rust state is unchanged",
        run_hook_with(gate, project, "{}", &fake.env).code == 0 && fake.runs() == before,
    ));

    fs::write(&main_rs, "fn main() {}\n// edited again\n").unwrap();
    checks.push((
        "gate runs clippy again after a .rs edit",
        run_hook_with(gate, project, "{}", &fake.env).code == 2,
    ));
    fs::write(&main_rs, "fn main() {}\n// edited\n").unwrap();
    checks.push((
        "gate skips again once back to the passing state",
        run_hook_with(gate, project, "{}", &fake.env).code == 0,
    ));
    fs::write(project.join("src/extra.rs"), "").unwrap();
    checks.push((
        "gate runs clippy again for a new untracked .rs file",
        run_hook_with(gate, project, "{}", &fake.env).code == 2,
    ));
    fs::remove_file(project.join("src/extra.rs")).unwrap();
    sh(project, "git", &["add", "README.md"]);
    sh(
        project,
        "git",
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "readme",
        ],
    );
    checks.push((
        "gate runs clippy again after HEAD moved",
        run_hook_with(gate, project, "{}", &fake.env).code == 2,
    ));
    checks
}

/// The errors with no location, and the stamp read from the content of the files.
fn diagnostic_checks(gate: &Path, project: &Path, fake: &FakeCargo) -> Vec<(&'static str, bool)> {
    let main_rs = project.join("src/main.rs");
    let mut checks = Vec::new();
    fs::write(
        &fake.out,
        "   Compiling dep v1.0.0\nsrc/main.rs:1:1: error: located\nerror: failed to run custom build command for `dep`\n\
         error: could not compile `hooktest`\n",
    )
    .unwrap();
    let out = run_hook_with(gate, project, "{}", &fake.env);
    checks.push((
        "gate reports the errors that have no location",
        out.stderr.contains("src/main.rs:1:1: error: located")
            && out.stderr.contains("custom build command"),
    ));
    checks.push((
        "gate reports no build progress with an unlocated error",
        !out.stderr.contains("Compiling") && !out.stderr.contains("could not compile"),
    ));
    fs::write(
        &fake.out,
        "   Compiling dep v1.0.0\n    Checking hooktest\nerror: failed to parse manifest\n",
    )
    .unwrap();
    let out = run_hook_with(gate, project, "{}", &fake.env);
    checks.push((
        "gate reports no build progress without any located diagnostic",
        out.code == 2
            && out.stderr.contains("failed to parse manifest")
            && !out.stderr.contains("Compiling"),
    ));

    // the stamp reads the content of the files, whatever their name or the diff settings
    fs::write(&fake.pass, "").unwrap();
    let cafe = project.join("src/café.rs");
    fs::write(&cafe, "// one\n").unwrap();
    run_hook_with(gate, project, "{}", &fake.env);
    fs::remove_file(&fake.pass).unwrap();
    fs::write(&cafe, "// two\n").unwrap();
    checks.push((
        "gate runs clippy again after an edit of a non-ASCII file",
        run_hook_with(gate, project, "{}", &fake.env).code == 2,
    ));
    fs::remove_file(&cafe).unwrap();
    fs::write(project.join(".gitattributes"), "*.rs -diff\n").unwrap();
    fs::write(&fake.pass, "").unwrap();
    run_hook_with(gate, project, "{}", &fake.env);
    fs::remove_file(&fake.pass).unwrap();
    fs::write(&main_rs, "fn main() {}\n// a -diff edit\n").unwrap();
    checks.push((
        "gate runs clippy again after an edit git shows no diff of",
        run_hook_with(gate, project, "{}", &fake.env).code == 2,
    ));
    checks
}

/// The project the gate checks: the worktree of the event's `cwd`, its crate subdirectory, or `CLAUDE_PROJECT_DIR`.
fn worktree_gate_checks(
    gate: &Path,
    project: &Path,
    fake: &FakeCargo,
) -> Vec<(&'static str, bool)> {
    let main_rs = project.join("src/main.rs");
    let mut checks = Vec::new();
    // a session that entered an issue worktree: the gate checks the worktree of the event's `cwd`, not the passing
    // `CLAUDE_PROJECT_DIR`
    fs::write(&fake.pass, "").unwrap();
    run_hook_with(gate, project, "{}", &fake.env);
    let worktree = project.with_file_name(format!(
        "{}-gate",
        project.file_name().unwrap().to_string_lossy()
    ));
    sh(
        project,
        "git",
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            worktree.to_str().unwrap(),
        ],
    );
    fs::write(
        worktree.join("src/main.rs"),
        "fn main() {}\n// worktree edit\n",
    )
    .unwrap();
    fs::remove_file(&fake.pass).unwrap();
    let before = fake.runs();
    let event = format!("{{\"cwd\":\"{}\"}}", worktree.join("src").display());
    checks.push((
        "gate checks the worktree of the event's cwd",
        run_hook_with(gate, project, &event, &fake.env).code == 2 && fake.runs() == before + 1,
    ));
    sh(
        project,
        "git",
        &["worktree", "remove", "--force", worktree.to_str().unwrap()],
    );

    // the same with the crate in a subdirectory of the repository: the gate checks that subdirectory of the worktree
    let mono = project.join("fake-mono");
    fs::create_dir_all(mono.join("backend/src")).unwrap();
    sh(&mono, "git", &["init", "-q", "."]);
    fs::write(mono.join("backend/Cargo.toml"), "").unwrap();
    fs::write(mono.join("backend/src/main.rs"), "fn main() {}\n").unwrap();
    commit_all(&mono, "init");
    let mono_tree = project.with_file_name(format!(
        "{}-mono",
        project.file_name().unwrap().to_string_lossy()
    ));
    sh(
        &mono,
        "git",
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            mono_tree.to_str().unwrap(),
        ],
    );
    fs::write(
        mono_tree.join("backend/src/main.rs"),
        "fn main() {}\n// worktree edit\n",
    )
    .unwrap();
    let before = fake.runs();
    let event = format!("{{\"cwd\":\"{}\"}}", mono_tree.display());
    checks.push((
        "gate checks the crate's subdirectory in the worktree of the event's cwd",
        run_hook_with(gate, &mono.join("backend"), &event, &fake.env).code == 2
            && fake.runs() == before + 1,
    ));
    sh(
        &mono,
        "git",
        &["worktree", "remove", "--force", mono_tree.to_str().unwrap()],
    );

    // an event's cwd outside any git work tree: the gate checks CLAUDE_PROJECT_DIR
    let outside = project.with_file_name(format!(
        "{}-nogit",
        project.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(&outside).unwrap();
    fs::write(&main_rs, "fn main() {}\n// outside edit\n").unwrap();
    let before = fake.runs();
    let event = format!("{{\"cwd\":\"{}\"}}", outside.display());
    checks.push((
        "gate checks CLAUDE_PROJECT_DIR when the event's cwd is outside git",
        run_hook_with(gate, project, &event, &fake.env).code == 2 && fake.runs() == before + 1,
    ));
    fs::remove_dir_all(&outside).unwrap();
    checks.extend(fresh_repo_checks(gate, project, fake));
    checks
}

/// A repository with no commit yet, and a project without `Cargo.toml`.
fn fresh_repo_checks(gate: &Path, project: &Path, fake: &FakeCargo) -> Vec<(&'static str, bool)> {
    let mut checks = Vec::new();
    // a repository with no commit yet: a staged file edited again
    let fresh = project.join("fake-fresh");
    fs::create_dir_all(fresh.join("src")).unwrap();
    sh(&fresh, "git", &["init", "-q", "."]);
    fs::write(fresh.join("Cargo.toml"), "").unwrap();
    fs::write(fresh.join("src/main.rs"), "fn main() {}\n").unwrap();
    sh(&fresh, "git", &["add", "-A"]);
    fs::write(&fake.pass, "").unwrap();
    run_hook_with(gate, &fresh, "{}", &fake.env);
    fs::remove_file(&fake.pass).unwrap();
    fs::write(fresh.join("src/main.rs"), "fn main() { let unused = 1; }\n").unwrap();
    checks.push((
        "gate runs clippy again after an edit of a staged file with no commit",
        run_hook_with(gate, &fresh, "{}", &fake.env).code == 2,
    ));
    fs::remove_file(fresh.join("Cargo.toml")).unwrap();
    let before = fake.runs();
    checks.push((
        "gate runs no clippy in a project without Cargo.toml",
        run_hook_with(gate, &fresh, "{}", &fake.env).code == 0 && fake.runs() == before,
    ));
    checks
}

// spec: PAR-12
/// An edit in an issue worktree: the format hook formats the project of the edited file, whatever the event's `cwd`
/// and `CLAUDE_PROJECT_DIR` say.
fn format_hook_follows_cwd(format_hook: &Path, project: &Path) -> Vec<(&'static str, bool)> {
    let main_rs = project.join("src/main.rs");
    let mut checks = Vec::new();
    let worktree = project.with_file_name(format!(
        "{}-wt",
        project.file_name().unwrap().to_string_lossy()
    ));
    sh(
        project,
        "git",
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            worktree.to_str().unwrap(),
        ],
    );
    let unformatted = "fn main(){let  a=1;println!(\"{a}\");}\n";
    fs::write(&main_rs, unformatted).unwrap();
    fs::write(worktree.join("src/main.rs"), unformatted).unwrap();
    let json = format!(
        "{{\"cwd\":\"{}\",\"tool_input\":{{\"file_path\":\"{}\"}}}}",
        project.display(),
        worktree.join("src/main.rs").display()
    );
    run_hook(format_hook, project, &json);
    checks.push((
        "format hook formats the worktree of the edited file",
        fs::read_to_string(worktree.join("src/main.rs"))
            .unwrap()
            .contains("let a = 1;"),
    ));
    let untouched = fs::read_to_string(&main_rs).unwrap() == unformatted;
    checks.push((
        "format hook leaves the project of the session alone",
        untouched,
    ));
    sh(
        project,
        "git",
        &["worktree", "remove", "--force", worktree.to_str().unwrap()],
    );
    fs::write(&main_rs, "fn main() {}\n").unwrap();
    checks
}

// spec: FMT-1
#[test]
fn rust_hooks() {
    let format_hook = PathBuf::from("format-code");
    let gate = PathBuf::from("lint-gate");

    let project: PathBuf = tempfile::tempdir().unwrap().keep();
    fs::create_dir_all(&project).unwrap();
    sh(&project, "git", &["init", "-q", "."]);
    sh(
        &project,
        "cargo",
        &["init", "-q", "--name", "hooktest", "."],
    );
    commit_all(&project, "init");

    let main_rs = project.join("src/main.rs");
    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };

    // format hook: reformats an edited .rs file, ignores other files
    fs::write(&main_rs, "fn main(){let  a=1;println!(\"{a}\");}\n").unwrap();
    let json = format!(
        "{{\"tool_input\":{{\"file_path\":\"{}\"}}}}",
        main_rs.display()
    );
    check(
        "format hook exits 0",
        run_hook(&format_hook, &project, &json).code == 0,
    );
    check(
        "format hook reformatted the file",
        fs::read_to_string(&main_rs).unwrap().contains("let a = 1;"),
    );
    fs::write(&main_rs, "fn main(){let  a=1;println!(\"{a}\");}\n").unwrap();
    let json = format!(
        "{{\"tool_input\":{{\"file_path\":\"{}/README.md\"}}}}",
        project.display()
    );
    check(
        "format hook ignores non-Rust files",
        run_hook(&format_hook, &project, &json).code == 0,
    );
    check(
        "format hook left the .rs file alone for a non-Rust edit",
        fs::read_to_string(&main_rs).unwrap().contains("let  a=1;"),
    );
    fs::write(&main_rs, "fn main() {}\n").unwrap();
    commit_all(&project, "clean");

    for (name, ok) in format_hook_follows_cwd(&format_hook, &project) {
        check(name, ok);
    }

    for (name, ok) in clippy_gate_checks(&gate, &project) {
        check(name, ok);
    }

    let _ = fs::remove_dir_all(&project);
    assert!(!failed, "a check failed: see the FAIL lines");
}
