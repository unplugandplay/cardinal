//! Behavior test for `cardinal tool cardinal-changes` on a fixture repository: the expectations of the commits that
//! change `files/`, the revisions in use that hold each change, the refusal of what it cannot read.

use std::path::Path;
use std::process::Command;

const SESSIONS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\tmain_context_peak\tadvisor_models\tadvisor_calls\tadvisor_input_tokens\tadvisor_output_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Writes `path` and commits it at `day` of January with `message`; returns the full hash.
fn commit(repo: &Path, day: u32, path: &str, message: &str) -> String {
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, format!("{day}\n")).unwrap();
    git(repo, &["add", "-A"]);
    let date = format!("2026-01-{day:02}T00:00:00Z");
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            message,
        ])
        .current_dir(repo)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    git(repo, &["rev-parse", "HEAD"])
}

/// A sessions file with one row per (revision, day of January).
fn sessions(path: &Path, rows: &[(&str, u32)]) {
    let mut text = format!("{SESSIONS}\n");
    for (i, (revision, day)) in rows.iter().enumerate() {
        let start = format!("2026-02-{day:02}T00:00:00.000Z");
        let mut cells = vec![String::new(); 27];
        cells[0] = format!("s{i}");
        cells[1] = "p".into();
        cells[3] = (*revision).into();
        cells[6] = start;
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    std::fs::write(path, text).unwrap();
}

fn run(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "cardinal-changes"])
        .args(args)
        .current_dir(cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// spec: VRD-3, VRD-4, VRD-7, VRD-13
#[test]
fn lists_the_expectations_of_the_changes_with_the_revisions_that_hold_them() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("harness");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    let c1 = commit(
        &repo,
        1,
        "files/a.md",
        "feat(files): SECRET subject\n\nSECRET body.\n\nExpected-effect: turns down\nExpected-effect: total_bytes up\n",
    );
    commit(
        &repo,
        2,
        "docs/a.md",
        "docs: x\n\nExpected-effect: turns up\n",
    );
    let c3 = commit(
        &repo,
        3,
        "files/b.md",
        "fix(files): y\n\nExpected-effect: speed up\n",
    );
    git(&repo, &["checkout", "-q", "-b", "side"]);
    let c4 = commit(
        &repo,
        4,
        "files/side.md",
        "feat(files): z\n\nExpected-effect: validations up\n",
    );
    git(&repo, &["checkout", "-q", "main"]);
    git(
        &repo,
        &[
            "merge",
            "-q",
            "--no-ff",
            "side",
            "-m",
            "Merge\n\nExpected-effect: turns up",
        ],
    );
    let c5 = commit(
        &repo,
        5,
        "files/c.md",
        "feat(files): w\n\nExpected-effect: breached_share:commit-attribution down\nExpected-effect: bytes_per_session:oversized-result down\n",
    );
    // Two commits of the same second keep the order they were made in.
    let c6 = commit(
        &repo,
        6,
        "files/d.md",
        "feat(files): v\n\nExpected-effect: turns up\n",
    );
    let c7 = commit(
        &repo,
        6,
        "files/e.md",
        "feat(files): u\n\nExpected-effect: turns down\n",
    );
    let file = tmp.path().join("sessions.tsv");
    sessions(
        &file,
        &[
            (&c5, 1),
            (&format!("self:{c3}"), 2),
            ("unpinned", 3),
            ("", 4),
            ("ffffffffffffffffffffffffffffffffffffffff", 5),
            (&c1, 6),
            (&c5, 7),
            ("main", 8),
        ],
    );
    let (code, stdout, stderr) = run(&repo, &["--sessions", file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    let held = format!("{c5},self:{c3},{c1}");
    let expected = [
        "change\tmetric\tdirection\trevisions".to_string(),
        format!("{c1}\tturns\tdown\t{held}"),
        format!("{c1}\ttotal_bytes\tup\t{held}"),
        format!("{c4}\tvalidations\tup\t{c5}"),
        format!("{c5}\tbreached_share:commit-attribution\tdown\t{c5}"),
        format!("{c5}\tbytes_per_session:oversized-result\tdown\t{c5}"),
        format!("{c6}\tturns\tup\t"),
        format!("{c7}\tturns\tdown\t"),
    ];
    assert_eq!(stdout.lines().collect::<Vec<_>>(), expected);
    assert!(!stdout.contains("SECRET"));

    // From another directory, the repository is the argument.
    let (code, other, _) = run(
        tmp.path(),
        &[repo.to_str().unwrap(), "--sessions", file.to_str().unwrap()],
    );
    assert_eq!((code, other), (0, stdout));
}

// spec: VRD-8
#[test]
fn refuses_what_it_cannot_read() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("harness");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    commit(&repo, 1, "files/a.md", "feat(files): a\n");
    let good = tmp.path().join("sessions.tsv");
    sessions(&good, &[]);
    let bad = tmp.path().join("bad.tsv");
    std::fs::write(&bad, "session\trevision\n").unwrap();
    let not_git = tmp.path().join("plain");
    std::fs::create_dir_all(&not_git).unwrap();
    let (good, bad) = (good.to_str().unwrap(), bad.to_str().unwrap());
    for args in [
        vec![],
        vec!["--sessions", bad],
        vec!["--sessions", good, "--nope"],
        vec!["--sessions"],
        vec![not_git.to_str().unwrap(), "--sessions", good],
    ] {
        let (code, stdout, stderr) = run(&repo, &args);
        assert_ne!(code, 0, "{args:?}");
        assert_eq!(stdout, "", "{args:?}");
        assert!(stderr.contains("error"), "{args:?}: {stderr}");
    }
}
