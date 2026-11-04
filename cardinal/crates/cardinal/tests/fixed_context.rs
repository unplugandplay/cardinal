//! Behavior test for `cardinal tool fixed-context` on fixture transcripts and repositories: one row per project and revision,
//! the commit the revision came in force with, the bytes read from its tree, the skills it ships.

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str =
    "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// Writes `files` (path, content; an empty content removes the file) and commits them in `repo` at `date`; returns
/// the abbreviated hash.
fn commit_at(repo: &Path, date: &str, files: &[(&str, &str)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        if body.is_empty() {
            let _ = std::fs::remove_file(&file);
            continue;
        }
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    sh(repo, &["add", "-A", "--force"]);
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "chore: fixture",
        ])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let head = Command::new("git")
        .args(["log", "-1", "--format=%h"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn new_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    sh(dir, &["init", "-q", "-b", "main"]);
    dir.to_path_buf()
}

/// A one-line session transcript in `cwd` at `ts`.
fn session(projects: &Path, project: &str, id: &str, cwd: &Path, ts: &str) {
    let dir = projects.join(project);
    std::fs::create_dir_all(&dir).unwrap();
    let line = json!({"type": "user", "cwd": cwd, "timestamp": ts, "gitBranch": "main", "sessionId": id, "message": {"role": "user", "content": "SECRET ok"}});
    std::fs::write(dir.join(format!("{id}.jsonl")), format!("{line}\n")).unwrap();
}

fn skill(name: &str, description: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n\n# SECRET body not counted\n")
}

fn run(dir: &Path, projects: &Path) -> (bool, String) {
    let out = tool_command(dir.join("fixed_context.rs"))
        .arg(projects)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The project `pinned` with its sessions under `projects`; returns the commits that put `r1` and `r2` in force.
fn pinned_project(tmp: &Path, projects: &Path) -> (String, String) {
    let toml = |rev: &str| {
        format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
    };
    let pinned = new_repo(&tmp.join("pinned"));
    commit_at(&pinned, "2025-12-01T00:00:00Z", &[("README.md", "x")]);
    // a rule linked to a file of another directory, read in the commit
    std::fs::create_dir_all(pinned.join(".claude/rules")).unwrap();
    std::os::unix::fs::symlink("../../shared/rule.md", pinned.join(".claude/rules/link.md"))
        .unwrap();
    let c1 = commit_at(
        &pinned,
        "2026-01-01T00:00:00Z",
        &[
            (".claude/harness.toml", &toml("r1")),
            ("AGENTS.md", "aaaa"),
            ("CLAUDE.md", "@AGENTS.md"),
            (".claude/CLAUDE.md", "@../docs/x.md `@AGENTS.md`"),
            ("docs/x.md", "xxx"),
            // imported from the directory of the linked rule's target
            ("shared/rule.md", "@inc.md"),
            ("shared/inc.md", "iii"),
            (".claude/rules/r.md", "rrrrr"),
            // personal memory, loaded at launch but never measured, even when committed
            ("CLAUDE.local.md", "local"),
            (".claude/rules/s.md", "---\npaths: src\n---\nSECRET scoped"),
            (".claude/skills/one/SKILL.md", &skill("one", "d1d1")),
        ],
    );
    let c2 = commit_at(
        &pinned,
        "2026-01-05T00:00:00Z",
        &[
            (".claude/harness.toml", &toml("r2")),
            ("AGENTS.md", "aaaaaaaaaa"),
            ("CLAUDE.md", ""),
            (".claude/CLAUDE.md", ""),
            (".claude/skills/two/SKILL.md", &skill("two", "\"d2\"")),
            (
                ".claude/skills/three/SKILL.md",
                "---\nname: t\nmetadata:\n  name: SECRETNESTED\ndescription: >\n  ab\n  cd\n---\n",
            ),
        ],
    );
    commit_at(
        &pinned,
        "2026-01-06T00:00:00Z",
        &[("AGENTS.md", "aaaaaaaaaaaaaaaaaaaa")],
    );
    session(
        projects,
        "-pinned-b",
        "s3",
        &pinned,
        "2026-01-08T00:00:00.000Z",
    );
    session(
        projects,
        "-pinned-a",
        "s2",
        &pinned,
        "2026-01-07T00:00:00.000Z",
    );
    session(
        projects,
        "-pinned-a",
        "s1",
        &pinned,
        "2026-01-02T00:00:00.000Z",
    );
    (c1, c2)
}

// spec: REP-1, REP-2
fn row_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let projects = tmp.join("projects");
    let (c1, c2) = pinned_project(tmp, &projects);
    let harness = new_repo(&tmp.join("harness"));
    commit_at(
        &harness,
        "2026-01-01T00:00:00Z",
        &[
            (".claude/harness.toml", "[harness]\nsource = \".\"\n"),
            ("files/AGENTS.md", "f"),
        ],
    );
    std::os::unix::fs::symlink("AGENTS.md", harness.join("CLAUDE.md")).unwrap();
    let review = "---\nname: review\ndescription: |\n  a\n\n  b\n---\n";
    let review_pr = "---\nname: review-pr\ndescription: \"x\n  y\"\n---\n";
    let own = commit_at(
        &harness,
        "2026-01-03T00:00:00Z",
        &[
            ("files/AGENTS.md", "ff"),
            ("AGENTS.md", "ff"),
            (".claude/skills/review/SKILL.md", review),
            (".claude/skills/review-pr/SKILL.md", review_pr),
        ],
    );
    commit_at(&harness, "2026-01-04T00:00:00Z", &[("AGENTS.md", "fff")]);
    session(
        &projects,
        "-harness",
        "h1",
        &harness,
        "2026-01-05T00:00:00.000Z",
    );
    let bare = new_repo(&tmp.join("bare"));
    commit_at(&bare, "2026-01-01T00:00:00Z", &[("README.md", "x")]);
    session(&projects, "-bare", "b1", &bare, "2026-01-02T00:00:00.000Z");

    let (ok, out) = run(dir, &projects);
    let full = |c: &str| {
        let o = Command::new("git")
            .args(["rev-parse", c])
            .current_dir(&harness)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    let expected = [
        COLUMNS.to_string(),
        format!(
            "harness\tself:{}\t{own}\t2\t0\t22\treview,review-pr",
            full(&own)
        ),
        // the memory files and their imports; AGENTS.md only when imported or in place of the CLAUDE.md files
        format!("pinned\tr1\t{c1}\t54\t4\t7\tone"),
        format!("pinned\tr2\t{c2}\t15\t10\t18\tone,three,two"),
    ];
    let lines: Vec<&str> = out.lines().collect();
    if lines != expected {
        println!("got:\n{out}expected:\n{}", expected.join("\n"));
    }
    vec![
        (
            "one row per project and revision, sorted, no row for an empty revision",
            ok && lines.first() == Some(&COLUMNS) && lines.len() == 4 && !out.contains("bare"),
        ),
        (
            "the bytes and skills come from the commit the revision came in force with",
            lines == expected,
        ),
        (
            "no text of a message nor of a skill body",
            !out.contains("SECRET"),
        ),
    ]
}

#[test]
fn fixed_context() {
    let dir = std::path::PathBuf::from("/tools");
    let temp = tempfile::tempdir().unwrap();
    let checks: Vec<_> = row_checks(&dir, temp.path()).into_iter().collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// `cardinal tool <name>` for the script path the checks were written against (`next_issue.rs` is `next-issue`).
fn tool_command(script: impl AsRef<std::path::Path>) -> std::process::Command {
    let stem = script
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('_', "-");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.arg("tool").arg(stem);
    cmd
}
