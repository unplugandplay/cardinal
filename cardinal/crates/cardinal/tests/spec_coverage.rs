//! Behavior test for `cardinal tool spec-coverage`: the test files that cite no requirement of an `accepted` spec,
//! largest first.

use std::path::Path;
use std::process::Command;

const ACCEPTED: &str = "---\nid: RER\nstatus: accepted\nadrs: []\n---\n\n# Rerun\n\n## Intent\n\nReuse.\n\n## Requirements\n\n- **RER-1** (deterministic) A pass must reuse a saved result.\n- **RER-2** (deterministic) withdrawn: no longer.\n\n## Scenarios\n\n## Non-goals\n";
const DRAFT: &str = "---\nid: DRF\nstatus: draft\nadrs: []\n---\n\n# Draft\n\n## Intent\n\nLater.\n\n## Requirements\n\n- **DRF-1** (deterministic) A run must resume.\n\n## Scenarios\n\n## Non-goals\n";

fn write(root: &Path, path: &str, body: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, body).unwrap();
}

/// A root holding the two specs and `files`.
fn root(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "specs/rerun.md", ACCEPTED);
    write(dir.path(), "specs/draft.md", DRAFT);
    for (path, body) in files {
        write(dir.path(), path, body);
    }
    dir
}

/// The exit code and stdout of the report run on `root`.
fn report(root: &Path) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "spec-coverage"])
        .arg(root)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

// spec: COV-1, COV-4
#[test]
fn lists_each_test_file_with_its_line_count() {
    let dir = root(&[
        ("crates/a/tests/cli.rs", "#[test]\nfn t() {}\n"),
        (".claude/hooks/test_h.rs", "use x;\n\n#[test]\nfn t() {}\n"),
        ("crates/a/src/lib.rs", "fn helper() {}\n"),
        ("crates/a/tests/run.rs", "#[tokio::test]\nasync fn t() {}\n"),
        ("node_modules/x/t.rs", "#[test]\nfn t() {}\n"),
    ]);
    let (_, stdout) = report(dir.path());
    assert_eq!(
        stdout,
        "4\t.claude/hooks/test_h.rs\n2\tcrates/a/tests/cli.rs\n2\tcrates/a/tests/run.rs\n"
    );
}

// spec: COV-2
#[test]
fn a_file_citing_an_accepted_requirement_is_not_listed() {
    let dir = root(&[(
        "crates/a/tests/cli.rs",
        "// spec: RER-1\n#[test]\nfn t() {}\n\n#[test]\nfn u() {}\n",
    )]);
    assert!(!report(dir.path()).1.contains("cli.rs"));
}

// spec: COV-3
#[test]
fn a_citation_of_a_draft_or_unknown_id_is_no_citation() {
    let dir = root(&[
        (
            "crates/a/tests/draft.rs",
            "// spec: DRF-1\n#[test]\nfn t() {}\n",
        ),
        (
            "crates/a/tests/withdrawn.rs",
            "// spec: RER-2\n#[test]\nfn t() {}\n",
        ),
        (
            "crates/a/tests/unknown.rs",
            "// spec: RER-9 (internal: slow)\n#[test]\nfn t() {}\n",
        ),
    ]);
    let (_, stdout) = report(dir.path());
    assert!(stdout.contains("crates/a/tests/draft.rs"), "{stdout}");
    assert!(stdout.contains("crates/a/tests/unknown.rs"), "{stdout}");
    assert!(stdout.contains("crates/a/tests/withdrawn.rs"), "{stdout}");
}

// spec: COV-5
#[test]
fn orders_by_line_count_then_path() {
    let dir = root(&[
        ("crates/a/tests/b.rs", "#[test]\nfn t() {}\n"),
        ("crates/a/tests/a.rs", "#[test]\nfn t() {}\n"),
        ("crates/a/tests/z.rs", "\n\n#[test]\nfn t() {}\n"),
    ]);
    assert_eq!(
        report(dir.path()).1,
        "4\tcrates/a/tests/z.rs\n2\tcrates/a/tests/a.rs\n2\tcrates/a/tests/b.rs\n"
    );
}

/// Runs git in `dir`, offline and unsigned.
fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed");
}

// spec: COV-6
#[test]
fn reads_only_the_files_git_lists() {
    let dir = root(&[("crates/a/tests/cli.rs", "#[test]\nfn t() {}\n")]);
    let git = |args: &[&str]| git(dir.path(), args);
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&["worktree", "add", "-q", "-b", "x", ".claude/worktrees/x"]);
    write(
        dir.path(),
        ".claude/worktrees/x/crates/a/tests/nested.rs",
        "#[test]\nfn t() {}\n",
    );
    write(
        dir.path(),
        "crates/a/tests/untracked.rs",
        "#[test]\nfn t() {}\n",
    );
    let (_, stdout) = report(dir.path());
    assert_eq!(
        stdout,
        "2\tcrates/a/tests/cli.rs\n2\tcrates/a/tests/untracked.rs\n"
    );
}

// spec: COV-7, COV-8
#[test]
fn says_so_when_every_test_file_is_covered() {
    let dir = root(&[(
        "crates/a/tests/cli.rs",
        "// spec: RER-1\n#[test]\nfn t() {}\n",
    )]);
    assert_eq!(
        report(dir.path()),
        (
            Some(0),
            "no test file without an accepted spec\n".to_string()
        )
    );
}

// spec: COV-8
#[test]
fn exits_0_when_it_lists_files() {
    let dir = root(&[("crates/a/tests/cli.rs", "#[test]\nfn t() {}\n")]);
    assert_eq!(report(dir.path()).0, Some(0));
}

// spec: COV-9
#[test]
fn says_so_when_it_finds_no_test_file() {
    let dir = root(&[("crates/a/src/lib.rs", "fn helper() {}\n")]);
    assert_eq!(
        report(dir.path()),
        (Some(0), "no test file found\n".to_string())
    );
}

// spec: COV-11
#[test]
fn lists_the_tests_of_a_crate_at_the_root() {
    let dir = root(&[
        ("src/lib.rs", "#[test]\nfn t() {}\n"),
        ("tests/cli.rs", "#[test]\nfn t() {}\n"),
    ]);
    assert_eq!(
        report(dir.path()),
        (Some(0), "2\tsrc/lib.rs\n2\ttests/cli.rs\n".to_string())
    );
}

// spec: COV-10
#[test]
fn fails_when_git_cannot_list_the_files() {
    let dir = root(&[("crates/a/tests/cli.rs", "#[test]\nfn t() {}\n")]);
    git(dir.path(), &["init", "-q", "-b", "trunk", "."]);
    git(dir.path(), &["add", "-A"]);
    std::fs::write(dir.path().join(".git/index"), "corrupt").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "spec-coverage"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("git ls-files"));
    assert_eq!(out.stdout, [] as [u8; 0]);
}

// spec: COV-12
#[test]
fn a_declared_test_file_of_another_language_is_a_test_file() {
    let dir = root(&[
        (
            ".claude/cardinal.toml",
            "[project]\nspec_files = [\"*.rs\", \"test_*.py\"]\n",
        ),
        ("tests/test_cli.py", "def test_x():\n    pass\n"),
        (
            "tests/test_ok.py",
            "# spec: RER-1\ndef test_y():\n    pass\n",
        ),
        ("src/lib.rs", "fn helper() {}\n"),
    ]);
    assert_eq!(
        report(dir.path()),
        (Some(0), "2\ttests/test_cli.py\n".to_string())
    );
}

// spec: COV-8
#[test]
fn a_problem_of_the_configuration_is_no_failure() {
    let dir = root(&[
        (
            ".claude/cardinal.toml",
            "[project]\nspec_files = [\"tests/*.py\", \"*.rs\"]\n",
        ),
        ("tests/cli.rs", "#[test]\nfn t() {}\n"),
    ]);
    assert_eq!(
        report(dir.path()),
        (Some(0), "2\ttests/cli.rs\n".to_string())
    );
}
