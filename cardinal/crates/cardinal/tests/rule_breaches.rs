//! Behavior test for `cardinal tool rule-breaches` on fixture transcripts and repositories: the findings of each rule, `breach`
//! apart from `blocked`, their locations, the summary per rule and revision, no message text.

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "session\tproject\trevision\trule\tkind\tlocation\tdetail";
const SUMMARY: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked\tbytes";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// Commits `files` (path, content) in `repo` at `date`, with `message`; returns the abbreviated hash.
fn commit_at(repo: &Path, date: &str, message: &str, files: &[(&str, &str)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
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
            message,
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

/// A transcript under construction: each push returns the 1-based line number.
struct Transcript {
    cwd: String,
    ts: String,
    lines: Vec<String>,
}

impl Transcript {
    fn new(cwd: &Path, ts: &str) -> Self {
        Transcript {
            cwd: cwd.display().to_string(),
            ts: ts.to_string(),
            lines: Vec::new(),
        }
    }

    fn push(&mut self, mut line: serde_json::Value) -> usize {
        line["cwd"] = json!(self.cwd);
        line["timestamp"] = json!(self.ts);
        line["sessionId"] = json!("x");
        self.lines.push(line.to_string());
        self.lines.len()
    }

    fn typed(&mut self, text: &str) -> usize {
        self.push(json!({"type": "user", "gitBranch": "main", "message": {"role": "user", "content": text}}))
    }

    fn call(&mut self, id: &str, name: &str, input: &serde_json::Value) -> usize {
        self.push(json!({"type": "assistant", "message": {"id": format!("m{id}"), "content": [{"type": "tool_use", "id": id, "name": name, "input": input}]}}))
    }

    fn text(&mut self, id: &str, text: &str) -> usize {
        self.push(json!({"type": "assistant", "message": {"id": id, "content": [{"type": "text", "text": text}]}}))
    }

    fn result(&mut self, id: &str, error: bool, text: &str) -> usize {
        self.push(json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": id, "is_error": error, "content": text}]}}))
    }

    /// A call and its result; returns the line of the call.
    fn run(
        &mut self,
        id: &str,
        name: &str,
        input: &serde_json::Value,
        error: Option<&str>,
    ) -> usize {
        let at = self.call(id, name, input);
        self.result(id, error.is_some(), error.unwrap_or("SECRET output"));
        at
    }

    fn write(&self, path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            self.lines
                .iter()
                .map(|l| l.clone() + "\n")
                .collect::<String>(),
        )
        .unwrap();
    }
}

fn bash(command: &str) -> serde_json::Value {
    json!({"command": command})
}

fn edit(path: &Path) -> serde_json::Value {
    json!({"file_path": path, "old_string": "SECRET old", "new_string": "SECRET new"})
}

fn run(dir: &Path, projects: &Path, summary: bool) -> (bool, String) {
    let mut cmd = tool_command(dir.join("rule_breaches.rs"));
    cmd.arg(projects);
    if summary {
        cmd.arg("--summary");
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// A finding row of the session `s` at revision `rev`.
fn row(s: &str, rev: &str, rule: &str, kind: &str, loc: &str, detail: &str) -> String {
    format!("{s}\tproj\t{rev}\t{rule}\t{kind}\t{loc}\t{detail}")
}

/// Writes the session `a` of `repo` (and its subagent) under `proj`; returns its expected finding rows.
fn session_a(repo: &Path, proj: &Path) -> Vec<String> {
    let in_repo = |p: &str| repo.join(p);
    let mut t = Transcript::new(repo, "2026-02-01T10:00:00.000Z");
    t.typed("please SECRET");
    let no_verify = t.run("t1", "Bash", &bash("git commit --no-verify -m SECRET"), Some("PreToolUse:Bash hook error: [x/require_code_review.rs check]: Refused: --no-verify skips the gate."));
    let refused1 = t.call(
        "t2",
        "Bash",
        &bash("git commit -m \"SECRET mentions --no-verify\""),
    );
    t.result(
        "t2",
        true,
        "Refused: the last code review predates the last commit.",
    );
    t.run(
        "t3",
        "Skill",
        &json!({"skill": "code-review", "args": "low"}),
        None,
    );
    t.run(
        "t4",
        "Bash",
        &bash("git commit -q -F - <<'EOF'\nfeat: SECRET\n\n--no-verify\nEOF\n"),
        None,
    );
    t.call("t5", "Bash", &bash("git commit -m x"));
    let refused2 = t.result("t5", true, "Refused: the diff holds files the last code review never saw. Or ask the user to waive the review.");
    let bypass = t.run("t6", "Bash", &bash("git add a && git commit -m y"), None);
    let waive = t.run(
        "t7",
        "Bash",
        &bash("just waive-review"),
        Some("Permission to use Bash has been denied."),
    );
    let waiver_file = t.run(
        "t8",
        "Write",
        &json!({"file_path": in_repo(".git/claude-review-waived"), "content": ""}),
        None,
    );
    t.call("t9", "Bash", &bash("touch somewhere"));
    let waive_refused = t.result(
        "t9",
        true,
        "PreToolUse:Bash hook error: [x]: Refused: only the user waives the code review.",
    );
    let managed = t.run("t10", "Edit", &edit(&in_repo(".claude/hooks/x.rs")), None);
    t.run(
        "t11",
        "Edit",
        &edit(&in_repo("files/.claude/hooks/x.rs")),
        None,
    );
    t.run(
        "t12",
        "Write",
        &json!({"file_path": in_repo("issues/a.md"), "content": "SECRET\n# Tracking\n\nPlan: proposed\n"}),
        None,
    );
    t.run("t13", "Edit", &edit(&in_repo("specs/a.md")), None);
    t.run("t14", "Edit", &edit(&in_repo("docs/adr/0001-a.md")), None);
    let early = t.run("t15", "Edit", &edit(&in_repo("src/main.rs")), None);
    t.run(
        "t16",
        "Edit",
        &edit(Path::new("/elsewhere/memory.md")),
        None,
    );
    t.typed("ok");
    t.run("t17", "Edit", &edit(&in_repo("src/lib.rs")), None);
    t.write(&proj.join("a.jsonl"));
    let mut sub = Transcript::new(repo, "2026-02-01T10:30:00.000Z");
    let pushed = sub.run("s1", "Bash", &bash("git push --no-verify"), None);
    sub.write(&proj.join("a/subagents/agent-x.jsonl"));

    let a = |rule: &str, kind: &str, n: usize, detail: &str| {
        row("a", "r1", rule, kind, &format!("a.jsonl:{n}"), detail)
    };
    vec![
        a("no-verify", "blocked", no_verify, ""),
        a("review-before-commit", "blocked", refused1 + 1, ""),
        a("review-before-commit", "blocked", refused2, ""),
        a("review-before-commit", "breach", bypass, ""),
        a("review-waiver", "blocked", waive, ""),
        a("review-waiver", "breach", waiver_file, ""),
        a("review-waiver", "blocked", waive_refused, ""),
        a("managed-copy", "breach", managed, ".claude/hooks/x.rs"),
        a("validation-stop", "breach", early, "src/main.rs"),
        row(
            "a",
            "r1",
            "no-verify",
            "breach",
            &format!("subagents/agent-x.jsonl:{pushed}"),
            "",
        ),
    ]
}

/// Writes the sessions `b` to `h` of `repo` (and `c` of `bare`) under `projects`; returns their expected finding rows.
#[allow(clippy::many_single_char_names)] // each transcript is named after its file, `b` writes `b.jsonl`
fn other_sessions(tmp: &Path, repo: &Path, bare: &Path, projects: &Path) -> Vec<String> {
    let proj = projects.join("-proj");
    let in_repo = |p: &str| repo.join(p);
    let mut b = Transcript::new(repo, "2026-02-02T10:00:00.000Z");
    b.typed("hello");
    b.run("t1", "Edit", &edit(&in_repo("src/x.rs")), None);
    b.run(
        "t2",
        "Write",
        &json!({"file_path": in_repo("tests/t.rs"), "content": "SECRET \"Plan: proposed\""}),
        None,
    );
    b.run("t3", "Edit", &edit(&in_repo("src/y.rs")), None);
    b.run("t4", "Bash", &bash("python3 edit.py"), Some("PreToolUse:Bash hook error: [x]: Refused: the diff changed since the last code review."));
    b.run("t5", "Bash", &bash("git commit -m z"), None);
    b.write(&proj.join("b.jsonl"));

    let mut c = Transcript::new(bare, "2026-02-02T10:00:00.000Z");
    c.run("t1", "Bash", &bash("git commit --no-verify -m x"), None);
    c.write(&projects.join("-bare/c.jsonl"));

    let mut d = Transcript::new(repo, "2025-12-01T10:00:00.000Z");
    d.run("t1", "Bash", &bash("git commit --no-verify -m x"), None);
    d.write(&proj.join("d.jsonl"));

    let mut g = Transcript::new(repo, "2026-02-03T10:00:00.000Z");
    g.call("t1", "Bash", &bash("git commit -m x"));
    let refused3 = g.result(
        "t1",
        true,
        "Refused: no code review has run in this project.",
    );
    g.typed("<command-name>/code-review</command-name>\n<command-args>low</command-args>");
    g.run("t2", "Bash", &bash("git commit -m x"), None);
    g.write(&proj.join("g.jsonl"));

    let linked = tmp.join("proj-foo");
    sh(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "foo",
            linked.to_str().unwrap(),
        ],
    );
    let mut h = Transcript::new(repo, "2026-02-04T10:00:00.000Z");
    h.typed("start the issue");
    h.cwd = linked.display().to_string();
    let switched = h.run(
        "t1",
        "Edit",
        &edit(&linked.join(".claude/hooks/x.rs")),
        None,
    );
    let quoted = h.run(
        "t2",
        "Bash",
        &bash("echo \"a << b\"\ngit commit --no-verify -m x"),
        None,
    );
    h.write(&proj.join("h.jsonl"));

    let mut e = Transcript::new(repo, "2026-03-02T10:00:00.000Z");
    let late = e.run("t1", "Bash", &bash("git -C x commit --no-verify"), None);
    e.write(&proj.join("e.jsonl"));

    vec![
        row(
            "e",
            "r2",
            "no-verify",
            "breach",
            &format!("e.jsonl:{late}"),
            "",
        ),
        row(
            "g",
            "r1",
            "review-before-commit",
            "blocked",
            &format!("g.jsonl:{refused3}"),
            "",
        ),
        row(
            "h",
            "r1",
            "managed-copy",
            "breach",
            &format!("h.jsonl:{switched}"),
            ".claude/hooks/x.rs",
        ),
        row(
            "h",
            "r1",
            "no-verify",
            "breach",
            &format!("h.jsonl:{quoted}"),
            "",
        ),
    ]
}

/// Builds the fixture; returns the projects directory and the expected finding rows (without header).
fn fixture(tmp: &Path) -> (PathBuf, Vec<String>) {
    let repo = new_repo(&tmp.join("proj"));
    let toml = |rev: &str| {
        format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
    };
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[
            (".claude/harness.toml", &toml("r1")),
            (
                ".claude/harness.lock",
                "abc+x  .claude/hooks/x.rs\ndef  AGENTS.md\n",
            ),
        ],
    );
    commit_at(
        &repo,
        "2026-03-01T00:00:00Z",
        "chore: bump",
        &[(".claude/harness.toml", &toml("r2"))],
    );
    let bare = new_repo(&tmp.join("bare"));
    commit_at(&bare, "2026-01-01T00:00:00Z", "init", &[("README", "x")]);
    let projects = tmp.join("projects");
    let mut expected = session_a(&repo, &projects.join("-proj"));
    expected.extend(other_sessions(tmp, &repo, &bare, &projects));
    (projects, expected)
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// spec: BRE-1, BRE-2, BRE-4, BRE-5, BRE-6, BRE-7, BRE-8, BRE-11
fn finding_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (projects, expected) = fixture(tmp);
    let (ok, out) = run(dir, &projects, false);
    let mut lines = out.lines();
    let header = lines.next().unwrap_or_default().to_string();
    let rows: Vec<String> = lines.map(str::to_string).collect();
    let mut checks = vec![
        ("the script succeeds".to_string(), ok),
        (
            "the header names the columns".to_string(),
            header == COLUMNS,
        ),
        (
            "no text of a message, a tool input or a tool result is printed".to_string(),
            !out.contains("SECRET"),
        ),
        (
            "no finding for a project without harness, nor before its harness".to_string(),
            !out.contains("\tbare\t") && !out.contains("d.jsonl"),
        ),
        (
            "no finding beyond the expected ones".to_string(),
            sorted(rows.clone()) == sorted(expected.clone()),
        ),
    ];
    for want in expected {
        checks.push((format!("finds {want}"), rows.contains(&want)));
    }
    if !checks.iter().all(|(_, ok)| *ok) {
        println!("{out}");
    }
    checks
}

// spec: BRE-3
fn summary_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (projects, _) = fixture(&tmp.join("summary"));
    let (ok, out) = run(dir, &projects, true);
    let expected = [
        SUMMARY,
        "managed-copy\tr1\t3\t2\t2\t0\t",
        "no-verify\tr1\t4\t2\t2\t1\t",
        "no-verify\tr2\t1\t1\t1\t0\t",
        "oversized-result\tr1\t4\t0\t0\t0\t0",
        "oversized-result\tr2\t1\t0\t0\t0\t0",
        "oversized-result-subagent\tr1\t1\t0\t0\t0\t0",
        "review-before-commit\tr1\t4\t1\t1\t3\t",
        "review-before-commit\tr2\t1\t0\t0\t0\t",
        "review-waiver\tr1\t4\t1\t1\t2\t",
        "review-waiver\tr2\t1\t0\t0\t0\t",
        "stage-everything\tr1\t1\t0\t0\t0\t",
        "validation-stop\tr1\t1\t1\t1\t0\t",
    ];
    let lines: Vec<&str> = out.lines().collect();
    if lines != expected {
        println!("{out}");
    }
    vec![(
        "the summary counts per rule and revision, sorted".to_string(),
        ok && lines == expected,
    )]
}

/// Run git with `args` in `repo` as the user `t`, at the given author and committer dates.
fn git_at(repo: &Path, author_date: &str, committer_date: &str, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .env("GIT_AUTHOR_DATE", author_date)
        .env("GIT_COMMITTER_DATE", committer_date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed");
}

/// The repository `cm` of the commit checks; returns it with the commits that breach a rule: a long subject, an
/// unknown scope, an attribution trailer, a capitalized subject, a generated mark.
fn commit_repo(tmp: &Path) -> (PathBuf, [String; 5]) {
    let repo = new_repo(&tmp.join("cm"));
    let toml = "[harness]\nsource = \"https://example.org/harness\"\nrev = \"r1\"\n\n[project]\ncommit_scopes = [\"sync\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", toml)],
    );
    commit_at(&repo, "2026-02-01T09:00:00Z", "Not mine.", &[("a", "1")]);
    commit_at(
        &repo,
        "2026-02-01T10:15:00Z",
        "feat(sync): add x",
        &[("a", "2")],
    );
    let long = commit_at(
        &repo,
        "2026-02-01T10:25:00Z",
        &format!("feat: {}", "a".repeat(70)),
        &[("a", "3")],
    );
    let scope = commit_at(&repo, "2026-02-01T10:45:00Z", "fix(nope): y", &[("a", "4")]);
    let trailer = commit_at(
        &repo,
        "2026-02-01T10:50:00Z",
        "docs: z\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        &[("a", "5")],
    );
    let capital = commit_at(&repo, "2026-02-01T11:30:00Z", "Feat: Big.", &[("a", "6")]);
    sh(&repo, &["checkout", "-q", "-b", "other", "HEAD~3"]);
    let generated = commit_at(
        &repo,
        "2026-02-01T10:20:00Z",
        "chore(harness): w\n\nGenerated with [Claude Code]",
        &[("b", "1")],
    );
    let with_cli = toml.replace("[\"sync\"]", "[\"sync\", \"cli\"]");
    commit_at(
        &repo,
        "2026-02-01T10:21:00Z",
        "chore(sync): add the cli scope",
        &[(".claude/harness.toml", &with_cli)],
    );
    commit_at(
        &repo,
        "2026-02-01T10:22:00Z",
        "feat(cli): add x",
        &[("b", "2")],
    );
    sh(&repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("a"), "stashed").unwrap();
    git_at(
        &repo,
        "2026-02-01T10:40:00Z",
        "2026-02-01T10:40:00Z",
        &["stash", "-q"],
    );
    std::fs::write(repo.join("c"), "1").unwrap();
    sh(&repo, &["add", "c"]);
    git_at(
        &repo,
        "2026-01-15T10:00:00Z",
        "2026-02-01T10:41:00Z",
        &["commit", "-q", "-m", "Mine, rebased."],
    );
    commit_at(
        &repo,
        "2026-02-01T11:35:00Z",
        "fix(cardinal): v",
        &[("a", "7")],
    );
    git_at(
        &repo,
        "2026-02-01T11:40:00Z",
        "2026-02-01T11:40:00Z",
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge branch other.",
            "other",
        ],
    );
    (repo, [long, scope, trailer, capital, generated])
}

// spec: BRE-9
fn commit_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("commits");
    let (repo, [long, scope, trailer, capital, generated]) = commit_repo(&tmp);
    let projects = tmp.join("projects");
    for (id, start, end) in [
        ("s1", "2026-02-01T10:00:00.000Z", "2026-02-01T11:00:00.000Z"),
        ("s2", "2026-02-01T10:30:00.000Z", "2026-02-01T12:00:00.000Z"),
    ] {
        let mut t = Transcript::new(&repo, start);
        t.typed("SECRET start");
        t.ts = end.to_string();
        t.typed("ok");
        t.write(&projects.join("-cm").join(format!("{id}.jsonl")));
    }
    let row = |s: &str, rule: &str, hash: &str| format!("{s}\tcm\tr1\t{rule}\tbreach\t\t{hash}");
    let expected = vec![
        row("s1", "commit-format", &long),
        row("s1", "commit-attribution", &generated),
        row("s2", "commit-format", &scope),
        row("s2", "commit-attribution", &trailer),
        row("s2", "commit-format", &capital),
    ];
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().collect();
    let want_summary = [
        SUMMARY,
        "commit-attribution\tr1\t2\t2\t2\t0\t",
        "commit-format\tr1\t2\t2\t3\t0\t",
    ];
    if sorted(rows.clone()) != sorted(expected.clone()) || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the commit rules succeed".to_string(), ok),
        ("each commit of a session is checked once, against the latest-starting session; merges, stashes and the user's commits are skipped; a scope added on the branch counts".to_string(), sorted(rows) == sorted(expected)),
        ("the commit rules apply to the sessions with a checked commit".to_string(), summary == want_summary),
    ]
}

// spec: FEA-11
fn attribution_off_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("attribution-off");
    let repo = new_repo(&tmp.join("off"));
    let config = "[cardinal]\nsource = \"x\"\nrev = \"r1\"\ndisabled = [\"no-ai-attribution\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/cardinal.toml", config)],
    );
    commit_at(
        &repo,
        "2026-02-01T10:10:00Z",
        "docs: z\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        &[("a", "1")],
    );
    let projects = tmp.join("projects");
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("SECRET start");
    t.ts = "2026-02-01T11:00:00.000Z".to_string();
    t.typed("ok");
    t.write(&projects.join("-off").join("s1.jsonl"));
    let (ok, out) = run(dir, &projects, false);
    let (_, summary) = run(dir, &projects, true);
    vec![(
        "a commit of a project that turned off no-ai-attribution is not checked for an attribution"
            .to_string(),
        ok && !out.contains("commit-attribution") && !summary.contains("commit-attribution"),
    )]
}

// spec: FEA-22
fn convention_off_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("convention-off");
    let repo = new_repo(&tmp.join("off"));
    let config =
        "[cardinal]\nsource = \"x\"\nrev = \"r1\"\ndisabled = [\"conventional-commits\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/cardinal.toml", config)],
    );
    commit_at(
        &repo,
        "2026-02-01T10:10:00Z",
        "Added the parser.",
        &[("a", "1")],
    );
    let projects = tmp.join("projects");
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("SECRET start");
    t.ts = "2026-02-01T11:00:00.000Z".to_string();
    t.typed("ok");
    t.write(&projects.join("-off").join("s1.jsonl"));
    let (ok, out) = run(dir, &projects, false);
    let (_, summary) = run(dir, &projects, true);
    vec![(
        "a commit of a project that turned off conventional-commits is not checked for its format"
            .to_string(),
        ok && !out.contains("commit-format")
            && !summary.contains("commit-format")
            && summary.contains("commit-attribution"),
    )]
}

const FRENCH: &str = "Le livrable est fait : les tests passent et la revue ne trouve rien, donc je committe puis je passe au suivant dans la foulée.";
const ENGLISH: &str = "The deliverable is done: the tests pass and the review finds nothing, so I commit it and move on to the next one with the plan.";
/// French with no French stop word and many `an` and `as`, also French words: an English block for the old lists.
const AN_AS: &str = "Bilan : an premier, an deuxième, an troisième, an quatrième, an cinquième, an sixième ; as gagnant, as perdant, as volant, as rouge, as noir, as vert.";

// spec: BRE-10
fn language_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("language");
    let projects = tmp.join("projects");
    let mut expected = Vec::new();
    for (name, language) in [
        ("fr", Some(" french ")),
        ("de", Some("German")),
        ("none", None),
    ] {
        let repo = new_repo(&tmp.join(name));
        let key = language
            .map(|l| format!("user_language = \"{l}\"\n"))
            .unwrap_or_default();
        commit_at(
            &repo,
            "2026-01-01T00:00:00Z",
            "chore: sync",
            &[(
                ".claude/harness.toml",
                &format!("[harness]\nsource = \"x\"\nrev = \"r1\"\n\n[project]\n{key}"),
            )],
        );
        let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
        t.typed("SECRET");
        t.text("m1", FRENCH);
        t.text("m7", AN_AS);
        let english = t.text("m2", ENGLISH);
        t.text("m3", "The tests pass, so I commit it and move on.");
        t.text(
            "m4",
            &format!("Voici le diff :\n```\n{ENGLISH}\n```\net `{ENGLISH}` fini."),
        );
        t.text(
            "m6",
            &format!("Voici la sortie :\n   ~~~\n{ENGLISH}\n   ~~~\nfini."),
        );
        t.write(&projects.join(format!("-{name}/s.jsonl")));
        let mut sub = Transcript::new(&repo, "2026-02-01T10:01:00.000Z");
        sub.text("m5", ENGLISH);
        sub.write(&projects.join(format!("-{name}/s/subagents/agent-y.jsonl")));
        if name == "fr" {
            expected.push(format!(
                "s\tfr\tr1\tuser-language\tbreach\ts.jsonl:{english}\tintermediate"
            ));
        }
    }
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().collect();
    let want_summary = [SUMMARY, "user-language\tr1\t1\t1\t1\t0\t"];
    if rows != expected || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the language rule succeeds".to_string(), ok && !out.contains("SECRET") && !out.contains("deliverable")),
        ("a long block of the session in the other language is a breach; short, code, sub-agent blocks and French with `an` and `as` are not".to_string(), rows == expected),
        ("the language rule applies only with a French or English user_language, in any case".to_string(), summary == want_summary),
    ]
}

// spec: BRE-14
fn personal_language_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("personal");
    let projects = tmp.join("projects");
    let mut expected = Vec::new();
    for (name, personal) in [("en", "English"), ("de", "German")] {
        let repo = new_repo(&tmp.join(name));
        commit_at(
            &repo,
            "2026-01-01T00:00:00Z",
            "chore: sync",
            &[(
                ".claude/cardinal.toml",
                "[cardinal]\nsource = \"x\"\nrev = \"r1\"\n\n[project]\nuser_language = \"French\"\n",
            )],
        );
        // written after the session's start, never committed: read as it is when the measure runs
        std::fs::write(
            repo.join(".claude/cardinal.local.toml"),
            format!("[project]\nuser_language = \"{personal}\"\n"),
        )
        .unwrap();
        let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
        t.typed("go");
        t.text("m1", ENGLISH);
        let french = t.text("m2", FRENCH);
        t.write(&projects.join(format!("-{name}/s.jsonl")));
        if name == "en" {
            expected.push(format!(
                "s\ten\tr1\tuser-language\tbreach\ts.jsonl:{french}\tfinal"
            ));
        }
    }
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().collect();
    let want_summary = [SUMMARY, "user-language\tr1\t1\t1\t1\t0\t"];
    if rows != expected || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the personal language rule succeeds".to_string(), ok),
        (
            "the personal user_language wins over the committed one".to_string(),
            rows == expected,
        ),
        (
            "a personal language with no stop words leaves the session unjudged".to_string(),
            summary == want_summary,
        ),
    ]
}

const HINT: &str =
    "Your text is not in French, the user_language of .claude/cardinal.toml: write in French.";

// spec: BRE-13
fn language_kind_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("kinds");
    let projects = tmp.join("projects");
    let repo = new_repo(&tmp.join("k"));
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(
            ".claude/cardinal.toml",
            "[cardinal]\nsource = \"x\"\nrev = \"r1\"\n\n[project]\nuser_language = \"French\"\n",
        )],
    );
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    let hinted_tool = t.text("e1", ENGLISH);
    t.push(json!({"type": "attachment", "attachment": {"type": "hook_additional_context", "content": [HINT], "hookName": "PostToolUse:Bash"}}));
    t.text("f1", FRENCH);
    t.typed("go");
    let intermediate = t.text("e2", ENGLISH);
    t.text("f2", "Fini.");
    t.typed("go");
    let last = t.text("e3", ENGLISH);
    t.typed("go");
    let hinted_stop = t.text("e4", ENGLISH);
    t.push(json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": format!("Stop hook feedback:\n[cardinal run language-hint]: {HINT}")}}));
    let end = t.text("e5", ENGLISH);
    // each hint covers the run of blocks it judged: the blocks of later calls in the turn are not hinted
    t.typed("go");
    let first_run = t.text("e6", ENGLISH);
    t.run("c1", "Read", &json!({}), None);
    t.push(json!({"type": "attachment", "attachment": {"type": "hook_additional_context", "content": [HINT], "hookName": "PostToolUse:Read"}}));
    let second_run = t.text("e7", ENGLISH);
    t.run("c2", "Read", &json!({}), None);
    let third_run = t.text("e8", ENGLISH);
    t.run("c3", "Read", &json!({}), None);
    // a meta line that is not Stop hook feedback is no hint, even holding its words
    t.push(json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": format!("Read {HINT}")}}));
    let reply = t.text("e9", ENGLISH);
    t.push(json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": format!("Stop hook feedback:\n[cardinal run language-hint]: {HINT}")}}));
    t.write(&projects.join("-k/s.jsonl"));
    let row = |line: usize, kind: &str| {
        format!("s\tk\tr1\tuser-language\tbreach\ts.jsonl:{line}\t{kind}")
    };
    let expected = vec![
        row(hinted_tool, "hinted"),
        row(intermediate, "intermediate"),
        row(last, "final"),
        row(hinted_stop, "hinted"),
        row(end, "final"),
        row(first_run, "hinted"),
        row(second_run, "intermediate"),
        row(third_run, "intermediate"),
        row(reply, "hinted"),
    ];
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    if rows != expected {
        println!("{out}");
    }
    vec![
        ("the language kinds succeed".to_string(), ok),
        ("a language breach is hinted when the hook's hint follows it in the turn, else final or intermediate".to_string(), rows == expected),
    ]
}

/// The lock of a project that synced the three routed skills.
const ROUTED_LOCK: &str = "# lock\nh1  .claude/skills/commit-message/SKILL.md\nh2  .claude/skills/issue-workflow/SKILL.md\nh3  .claude/skills/write-spec/SKILL.md\n";

/// The sessions of the routing checks besides the main one: a typed skill, a sub-agent, a project with no routed skill.
fn routing_other_sessions(tmp: &Path, repo: &Path, config: &str) {
    let projects = tmp.join("projects");
    // a skill typed with its prefix is loaded
    let mut typed = Transcript::new(repo, "2026-02-01T11:00:00.000Z");
    typed.push(json!({"type": "user", "message": {"role": "user", "content": "<command-name>/files:commit-message</command-name>"}}));
    typed.run("c1", "Bash", &bash("git commit -m SECRET"), None);
    typed.write(&projects.join("-r/u.jsonl"));
    // a sub-agent shares neither the skills nor the triggers of the session
    let mut sub = Transcript::new(repo, "2026-02-01T10:00:00.000Z");
    sub.run("x1", "Bash", &bash("git commit -m SECRET"), None);
    sub.write(&projects.join("-r/s/subagents/agent-1.jsonl"));
    // a session that synced none of the routed skills is not due to load them
    let bare = new_repo(&tmp.join("n"));
    commit_at(
        &bare,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[
            (".claude/cardinal.toml", config),
            (".claude/cardinal.lock", "# lock\nh1  AGENTS.md\n"),
        ],
    );
    let mut n = Transcript::new(&bare, "2026-02-01T10:00:00.000Z");
    n.typed("go");
    n.run("c1", "Bash", &bash("git commit -m SECRET"), None);
    n.write(&projects.join("-n/s.jsonl"));
}

/// The commits of BRE-16: the throwaway or checked ones trigger nothing; the lines of those in the project, which do.
fn skipped_commits(t: &mut Transcript, repo: &Path) -> Vec<usize> {
    let repo = repo.display();
    let skipped = [
        "cd \"$(mktemp -d)\" && git init -q && git commit -q -m SECRET".to_string(),
        "git init -q t && cd t && git commit -m SECRET".to_string(),
        "git clone -q . t && cd t && git commit -m SECRET".to_string(),
        "cd /tmp/x && git commit -m SECRET".to_string(),
        "git -C /tmp/y commit -m SECRET".to_string(),
        "printf '%s\\n' \"$m\" | cardinal run commit-check && git commit -m \"$m\"".to_string(),
        "git commit -m SECRET && just check-commit".to_string(),
        "d=$(mktemp -d); cd $d && git init -q && git commit -m SECRET".to_string(),
        format!("cd {repo}/../x && git commit -m SECRET"),
    ];
    let counted = [
        "git commit -m 'fix: drop the mktemp dirs'".to_string(),
        "git commit -m 'fix: stop leaking $(mktemp -d) dirs'".to_string(),
        "git add src/commit_check.rs && git commit -m SECRET".to_string(),
        "git commit -m SECRET && out=$(mktemp)".to_string(),
        format!("cd /tmp/s && cd {repo} && git commit -m SECRET"),
        format!("cd {repo}/sub/.. && git commit -m SECRET"),
    ];
    for (i, command) in skipped.iter().enumerate() {
        t.run(&format!("k{i}"), "Bash", &bash(command), None);
    }
    counted
        .iter()
        .enumerate()
        .map(|(i, command)| t.run(&format!("n{i}"), "Bash", &bash(command), None))
        .collect()
}

// spec: BRE-15, BRE-16, BRE-17, BRE-18
fn routing_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("routing");
    let projects = tmp.join("projects");
    let config = "[cardinal]\nsource = \"x\"\nrev = \"r1\"\n";
    let repo = new_repo(&tmp.join("r"));
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[
            (".claude/cardinal.toml", config),
            (".claude/cardinal.lock", ROUTED_LOCK),
            ("specs/old.md", "SECRET"),
        ],
    );
    let write =
        |path: &str, content: &str| json!({"file_path": repo.join(path), "content": content});
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    let refused = t.run("c0", "Bash", &bash("git commit -m SECRET"), Some("exit 1"));
    // a call that failed loaded nothing; a command that writes no commit message triggers nothing
    t.run(
        "c1a",
        "Skill",
        &json!({"skill": "commit-message"}),
        Some("Unknown skill"),
    );
    t.run(
        "c1b",
        "Bash",
        &bash("git log --oneline | grep commit"),
        None,
    );
    t.run("c1c", "Bash", &bash("git commit --amend --no-edit"), None);
    let unloaded = t.run(
        "c1",
        "Bash",
        &bash("git add a && git -C . commit -m SECRET && git rebase --continue --no-edit"),
        None,
    );
    let counted = skipped_commits(&mut t, &repo);
    t.run("c2", "Skill", &json!({"skill": "commit-message"}), None);
    t.run("c3", "Bash", &bash("git commit -m SECRET"), None);
    let plan = t.run(
        "w1",
        "Write",
        &write("issues/i.md", "# Tracking\n\nPlan: proposed\n"),
        None,
    );
    t.push(json!({"type": "user", "message": {"role": "user", "content": "<command-name>/issue-workflow</command-name>\n<command-args>SECRET</command-args>"}}));
    t.run("w2", "Write", &write("issues/j.md", "Plan: proposed"), None);
    let spec = t.run("w3", "Write", &write("specs/new.md", "SECRET"), None);
    t.run("w4", "Write", &write("specs/old.md", "SECRET"), None);
    t.run("w4b", "Write", &write("specs/new.md", "SECRET again"), None);
    t.run("w5", "Write", &write("specs/README.md", "SECRET"), None);
    t.run("w6", "Write", &write("specs/notes.txt", "SECRET"), None);
    t.run("w7", "Skill", &json!({"skill": "files:write-spec"}), None);
    t.run("w8", "Write", &write("specs/other.md", "SECRET"), None);
    t.write(&projects.join("-r/s.jsonl"));
    routing_other_sessions(&tmp, &repo, config);
    let row = |rule: &str, kind: &str, line: usize, detail: &str| {
        format!("s\tr\tr1\tskill-routing-{rule}\t{kind}\ts.jsonl:{line}\t{detail}")
    };
    let mut expected = vec![
        row("commit-message", "blocked", refused, ""),
        row("commit-message", "breach", unloaded, ""),
    ];
    expected.extend(
        counted
            .iter()
            .map(|&line| row("commit-message", "breach", line, "")),
    );
    expected.extend([
        row("issue-workflow", "breach", plan, ""),
        row("write-spec", "breach", spec, "specs/new.md"),
    ]);
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out
        .lines()
        .filter(|l| l.contains("skill-routing"))
        .map(str::to_string)
        .collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary
        .lines()
        .filter(|l| l.starts_with("skill-routing"))
        .collect();
    let want_summary = [
        "skill-routing-commit-message\tr1\t2\t1\t7\t1\t",
        "skill-routing-issue-workflow\tr1\t1\t1\t1\t0\t",
        "skill-routing-write-spec\tr1\t1\t1\t1\t0\t",
    ];
    if rows != expected || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the routing rules succeed".to_string(), ok),
        ("a trigger is a finding until its skill is called or typed, in the session's own transcript only".to_string(), rows == expected),
        ("a routing rule applies to the sessions holding its trigger whose lock lists its skill".to_string(), summary == want_summary),
    ]
}

// spec: BRE-19, BRE-20, BRE-22
fn waste_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("waste");
    let projects = tmp.join("projects");
    let repo = new_repo(&tmp.join("w"));
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(
            ".claude/cardinal.toml",
            "[cardinal]\nsource = \"x\"\nrev = \"r1\"\n",
        )],
    );
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    // results over 25,000 bytes, a string or parts joined by newlines (the newline tips this one over), an error too
    t.call("o1", "Bash", &bash("cat big"));
    t.result("o1", false, &format!("{}SECR", "SECRET".repeat(4_166)));
    t.call("o2", "Bash", &bash("cat bigger"));
    let oversized = t.result("o2", true, &format!("{}SECRE", "SECRET".repeat(4_166)));
    t.call("o3", "Read", &json!({"file_path": repo.join("d.rs")}));
    let parts = t.push(json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "o3", "content": [{"type": "text", "text": "SECRET".repeat(2_500)}, {"type": "text", "text": format!("{}SECR", "SECRET".repeat(1_666))}]}]}}));
    t.write(&projects.join("-w/s.jsonl"));
    // a reviewer of `/code-review` (no description), an `Explore` sub-agent, and one with no `.meta.json`
    let subagents = projects.join("-w/s/subagents");
    let mut subs = Vec::new();
    for (name, meta) in [
        (
            "agent-1",
            Some(r#"{"agentType":"general-purpose","isBuiltIn":true}"#),
        ),
        (
            "agent-2",
            Some(r#"{"agentType":"Explore","description":"find the parser"}"#),
        ),
        ("agent-3", None),
    ] {
        let mut sub = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
        sub.call("x1", "Bash", &bash("cat big"));
        subs.push(sub.result("x1", false, &"SECRET".repeat(5_000)));
        sub.write(&subagents.join(format!("{name}.jsonl")));
        if let Some(meta) = meta {
            std::fs::write(subagents.join(format!("{name}.meta.json")), meta).unwrap();
        }
    }
    let row = |rule: &str, location: String, detail: &str| {
        format!("s\tw\tr1\t{rule}\tbreach\t{location}\t{detail}")
    };
    let expected = vec![
        row("oversized-result", format!("s.jsonl:{oversized}"), "25001"),
        row("oversized-result", format!("s.jsonl:{parts}"), "25001"),
        row(
            "oversized-result-review",
            format!("subagents/agent-1.jsonl:{}", subs[0]),
            "30000",
        ),
        row(
            "oversized-result-subagent",
            format!("subagents/agent-2.jsonl:{}", subs[1]),
            "30000",
        ),
        row(
            "oversized-result-subagent",
            format!("subagents/agent-3.jsonl:{}", subs[2]),
            "30000",
        ),
    ];
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().skip(1).collect();
    let want_summary = [
        "oversized-result\tr1\t1\t1\t2\t0\t50002",
        "oversized-result-review\tr1\t1\t1\t1\t0\t30000",
        "oversized-result-subagent\tr1\t1\t1\t2\t0\t60000",
    ];
    if sorted(rows.clone()) != sorted(expected.clone()) || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        (
            "the waste rules succeed".to_string(),
            ok && !out.contains("SECRET"),
        ),
        (
            "a tool result over 25,000 bytes is a breach at its line, of the rule of its transcript's source"
                .to_string(),
            sorted(rows) == sorted(expected),
        ),
        (
            "each waste rule applies to the sessions with a tool result of its source and sums its bytes"
                .to_string(),
            summary == want_summary,
        ),
    ]
}

// spec: BRE-21
fn stage_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("stage");
    let projects = tmp.join("projects");
    let repo = new_repo(&tmp.join("g"));
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(
            ".claude/cardinal.toml",
            "[cardinal]\nsource = \"x\"\nrev = \"r1\"\n",
        )],
    );
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    let chained = t.run(
        "a1",
        "Bash",
        &bash("git add -A && git commit -m 'SECRET'"),
        None,
    );
    t.run(
        "a2",
        "Bash",
        &bash("git add src/a.rs && git commit -m 'git add -A .'"),
        None,
    );
    let in_repo = t.run(
        "a3",
        "Bash",
        &bash("git -C repo add ."),
        Some("SECRET refused"),
    );
    let long = t.run("a4", "Bash", &bash("git add --all; ls ."), None);
    t.run("a5", "Bash", &bash("git commit -am x"), None);
    t.run("a7", "Bash", &bash("git add a.rs && ls ."), None);
    t.run("a6", "Bash", &bash("git status . && git add b.rs"), None);
    t.write(&projects.join("-g/s.jsonl"));
    let mut other = Transcript::new(&repo, "2026-02-02T10:00:00.000Z");
    other.typed("go");
    let alone = other.run("b1", "Bash", &bash("git add -A"), None);
    other.write(&projects.join("-g/t.jsonl"));
    let row = |s: &str, kind: &str, line: usize| {
        format!("{s}\tg\tr1\tstage-everything\t{kind}\t{s}.jsonl:{line}\t")
    };
    let expected = vec![
        row("s", "breach", chained),
        row("s", "blocked", in_repo),
        row("s", "breach", long),
        row("t", "breach", alone),
    ];
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out
        .lines()
        .filter(|l| l.contains("\tstage-everything\t"))
        .map(str::to_string)
        .collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary
        .lines()
        .filter(|l| l.starts_with("stage-everything\t"))
        .collect();
    let want_summary = ["stage-everything\tr1\t2\t2\t3\t1\t"];
    if sorted(rows.clone()) != sorted(expected.clone()) || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        (
            "the staging rule succeeds".to_string(),
            ok && !out.contains("SECRET"),
        ),
        (
            "a git add of -A, --all or . is a finding, not one of named files nor a quoted one"
                .to_string(),
            sorted(rows) == sorted(expected),
        ),
        (
            "the staging rule applies to the sessions with a git add".to_string(),
            summary == want_summary,
        ),
    ]
}

// spec: FEA-26
fn gate_off_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("gate-off");
    let repo = new_repo(&tmp.join("off"));
    let config = "[cardinal]\nsource = \"x\"\nrev = \"r1\"\ndisabled = [\"review-gate\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/cardinal.toml", config)],
    );
    let projects = tmp.join("projects");
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    t.run("a1", "Bash", &bash("git add -A"), None);
    t.run(
        "a2",
        "Bash",
        &bash("git commit -m x"),
        Some("Refused: run a code review first"),
    );
    t.run("a3", "Bash", &bash("git commit --no-verify -m x"), None);
    t.run("a4", "Bash", &bash("just waive-review"), None);
    t.run(
        "a5",
        "Write",
        &json!({"file_path": repo.join(".git/claude-review-waived"), "content": "SECRET"}),
        Some("Refused: only the user waives the review"),
    );
    t.write(&projects.join("-off/s.jsonl"));
    let (ok, out) = run(dir, &projects, false);
    let (_, summary) = run(dir, &projects, true);
    let rules = [
        "no-verify",
        "stage-everything",
        "review-waiver",
        "review-before-commit",
    ];
    let quiet = |text: &str| !rules.iter().any(|r| text.contains(&format!("{r}\t")));
    if !quiet(&out) || !quiet(&summary) {
        println!("{out}{summary}");
    }
    // the session is read: the waste rule applies to it
    vec![(
        "a session of a project that turned off review-gate is not checked against its rules"
            .to_string(),
        ok && quiet(&out) && quiet(&summary) && summary.contains("oversized-result\t"),
    )]
}

// spec: FEA-38
fn issue_files_off_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("issue-files-off");
    let repo = new_repo(&tmp.join("off"));
    let config = "[cardinal]\nsource = \"x\"\nrev = \"r1\"\ndisabled = [\"issue-files\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/cardinal.toml", config)],
    );
    let projects = tmp.join("projects");
    let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    t.typed("go");
    t.run(
        "p1",
        "Write",
        &json!({"file_path": repo.join("issues/x.md"), "content": "# Tracking\n\nPlan: proposed\n"}),
        None,
    );
    t.run(
        "p2",
        "Write",
        &json!({"file_path": repo.join("src/main.rs"), "content": "fn main() {}\n"}),
        None,
    );
    t.write(&projects.join("-off/s.jsonl"));
    let (ok, out) = run(dir, &projects, false);
    let (_, summary) = run(dir, &projects, true);
    let quiet = |text: &str| !text.contains("validation-stop\t");
    if !quiet(&out) || !quiet(&summary) {
        println!("{out}{summary}");
    }
    vec![(
        "a session of a project that turned off issue-files is not checked against validation-stop"
            .to_string(),
        // the session is read: the waste rule applies to it
        ok && quiet(&out) && quiet(&summary) && summary.contains("oversized-result\t"),
    )]
}

#[test]
fn rule_breaches() {
    let dir = std::path::PathBuf::from("/tools");
    let scratch = tempfile::tempdir().unwrap();
    let tmp = scratch.path().to_path_buf();
    let checks: Vec<_> = finding_checks(&dir, &tmp)
        .into_iter()
        .chain(summary_checks(&dir, &tmp))
        .chain(commit_checks(&dir, &tmp))
        .chain(attribution_off_checks(&dir, &tmp))
        .chain(convention_off_checks(&dir, &tmp))
        .chain(language_checks(&dir, &tmp))
        .chain(language_kind_checks(&dir, &tmp))
        .chain(personal_language_checks(&dir, &tmp))
        .chain(routing_checks(&dir, &tmp))
        .chain(waste_checks(&dir, &tmp))
        .chain(stage_checks(&dir, &tmp))
        .chain(gate_off_checks(&dir, &tmp))
        .chain(issue_files_off_checks(&dir, &tmp))
        .collect();
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
