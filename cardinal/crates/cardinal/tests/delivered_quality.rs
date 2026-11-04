//! Behavior test for `cardinal tool delivered-quality` on a fixture repository: the commits of each done issue, the fixes and
//! reverts of its files after the close, its deliverables, revision and reviews, the failed checks and typed messages
//! of its sessions, no text of a commit, an issue or a message.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "issue\trevision\tsetup\tclosed\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\ttraced\ttraced_without_red\tspec_lines\tdoc_lines\tsessions\tfailed_checks\tvalidations\tother_messages\tinput_tokens\tcache_read_tokens\toutput_tokens";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// Writes `files` (path, content; `None` removes it) in `repo` and commits them at `date` with `message`; returns the
/// full hash.
fn commit_at(repo: &Path, date: &str, message: &str, files: &[(&str, Option<&str>)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        match body {
            Some(body) => {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, body).unwrap();
            }
            None => std::fs::remove_file(file).unwrap(),
        }
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
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn toml(rev: &str) -> String {
    format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
}

const ALPHA: &str = "# SECRET alpha\n\nRun the code review.\n\n# Tracking\n\n## Plan\n1. [x] SECRET one\n2. [x] two\n   3. [ ] not a plan line\n\n## Decisions\n1. [x] not in the plan\n\n## Deliverable 1\n```sh\n# a comment, not a heading\n```\n| code review | low |\n- Code review: 6 findings, 1 finding dismissed.\n- code review (low): 1 finding; second review: no findings\n- Code review: no findings.\n\n# Notes\n- Code review: 4 findings, after the Tracking section.\n";

/// The commits after `alpha` is closed: fixes in and out of the window, reverts of `gap` and of `alpha`, a fix on a
/// side branch, each changing its own line of `src/a.rs`, which `alpha` wrote.
fn commits_after_close(repo: &Path, gap: &str) {
    commit_at(
        repo,
        "2026-01-10T00:00:00Z",
        "fix(files): later",
        &[("src/a.rs", Some("b1\nc2\na3\na4\na5\na6\na7\n"))],
    );
    commit_at(
        repo,
        "2026-01-10T01:00:00Z",
        "fix: docs and lock only",
        &[
            ("docs/arch.md", Some("b")),
            (".claude/harness.lock", Some("b")),
            (".claude/cardinal.lock", Some("b")),
        ],
    );
    commit_at(
        repo,
        "2026-01-11T00:00:00Z",
        "chore: undo",
        &[("src/b.rs", Some("x"))],
    );
    commit_at(
        repo,
        "2026-01-11T01:00:00Z",
        &format!("chore: undo the gap\n\nThis reverts commit {gap}."),
        &[("src/b.rs", Some("y"))],
    );
    commit_at(
        repo,
        "2026-01-12T00:00:00Z",
        "Revert \"feat(files): a\"",
        &[("src/a.rs", Some("b1\nc2\nd3\na4\na5\na6\na7\n"))],
    );
    commit_at(
        repo,
        "2026-01-13T00:00:00Z",
        "revert(files): a",
        &[("src/a.rs", Some("b1\nc2\nd3\ne4\na5\na6\na7\n"))],
    );
    commit_at(
        repo,
        "2026-02-20T00:00:00Z",
        "fix(files): too late",
        &[("src/a.rs", Some("b1\nc2\nd3\ne4\nf5\na6\na7\n"))],
    );
    commit_at(
        repo,
        "2026-01-04T23:00:00Z",
        "fix(files): rebased after the close",
        &[("src/a.rs", Some("b1\nc2\nd3\ne4\nf5\nh6\na7\n"))],
    );
    sh(repo, &["checkout", "-q", "-b", "side"]);
    commit_at(
        repo,
        "2026-01-09T00:00:00Z",
        "fix(files): on a side branch",
        &[("src/a.rs", Some("b1\nc2\nd3\ne4\nf5\nh6\ng7\n"))],
    );
    sh(repo, &["checkout", "-q", "main"]);
}

/// The fixture repository: `alpha` (three commits, one fix inside, fixes and reverts after), `beta` (closed in one
/// commit that also rewrites a citation in `alpha`), `gamma` (done in the work tree only).
fn fixture(tmp: &Path) -> PathBuf {
    let repo = tmp.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", Some(&toml("r1")))],
    );
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "docs: open the issues",
        &[
            ("issues/alpha.md", Some("# SECRET alpha\n")),
            ("issues/beta.md", Some("# beta\n")),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "feat(files): SECRET a",
        &[
            ("src/a.rs", Some("a1\na2\na3\na4\na5\na6\na7\n")),
            ("docs/arch.md", Some("a")),
            (".claude/harness.lock", Some("a")),
            (".claude/cardinal.lock", Some("a")),
            ("issues/alpha.md", Some(ALPHA)),
        ],
    );
    let gap = commit_at(
        &repo,
        "2026-01-04T00:00:00Z",
        "fix(files): gap",
        &[
            ("src/a.rs", Some("b1\na2\na3\na4\na5\na6\na7\n")),
            ("issues/alpha.md", Some(&format!("{ALPHA}x\n"))),
        ],
    );
    commit_at(
        &repo,
        "2026-01-04T06:00:00Z",
        "chore: bump",
        &[(".claude/harness.toml", Some(&toml("r2")))],
    );
    commit_at(&repo, "2026-01-04T12:00:00Z", "docs: close beta", &[
        ("issues/beta.md", None),
        ("issues/done/beta.md", Some("# beta\n\n# Tracking\n\n1. [x] one\n2. [x] two\n\n## Decisions\n1. [x] not in the plan\n* Code review: 2 findings.\n* Code review: fixed the doc.\n")),
        ("issues/alpha.md", Some(&format!("{ALPHA}y\n"))),
        (".claude/harness.toml", Some(&toml("r3"))),
    ]);
    commit_at(
        &repo,
        "2026-01-05T00:00:00Z",
        "docs: close alpha",
        &[
            ("issues/alpha.md", None),
            ("issues/done/alpha.md", Some(ALPHA)),
        ],
    );
    commits_after_close(&repo, &gap);
    std::fs::write(repo.join("issues/done/gamma.md"), "# gamma\n\n# Tracking\n").unwrap();
    repo
}

/// Runs the script on `repo`; with no `--projects` in `args`, on a projects directory that does not exist, so the
/// real transcripts are never read.
fn run(dir: &Path, repo: &Path, args: &[&str]) -> (bool, String) {
    let mut cmd = tool_command(dir.join("delivered_quality.rs"));
    cmd.arg(repo).args(args);
    if !args.contains(&"--projects") {
        cmd.arg("--projects").arg(repo.join("no-projects"));
    }
    let out = cmd.output().unwrap();
    if !out.status.success() {
        println!("{}", String::from_utf8_lossy(&out.stderr));
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The rows of `out` by issue, each a map from column to value.
fn rows(out: &str) -> HashMap<String, HashMap<String, String>> {
    let columns: Vec<&str> = COLUMNS.split('\t').collect();
    out.lines()
        .skip(1)
        .map(|l| {
            let row: HashMap<String, String> = columns
                .iter()
                .zip(l.split('\t'))
                .map(|(c, v)| (c.to_string(), v.to_string()))
                .collect();
            (row["issue"].clone(), row)
        })
        .collect()
}

fn has(
    rows: &HashMap<String, HashMap<String, String>>,
    issue: &str,
    column: &str,
    value: &str,
) -> bool {
    let got = rows
        .get(issue)
        .and_then(|r| r.get(column))
        .map(String::as_str);
    if got != Some(value) {
        println!("{issue}.{column}: expected {value:?}, got {got:?}");
    }
    got == Some(value)
}

// spec: QUA-1, QUA-2, QUA-3, QUA-4, QUA-5, QUA-10
fn git_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = fixture(tmp);
    let (ok, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    let issues: Vec<&str> = out
        .lines()
        .skip(1)
        .map(|l| l.split('\t').next().unwrap_or_default())
        .collect();
    let (_, wide) = run(dir, &repo, &["--window-days", "60"]);
    let w = rows(&wide);
    vec![
        (
            "the script runs and prints the header".to_string(),
            ok && out.lines().next() == Some(COLUMNS),
        ),
        (
            "one row per closed done issue, sorted by close date".to_string(),
            issues == ["beta", "alpha"],
        ),
        (
            "every row has every column".to_string(),
            out.lines()
                .all(|l| l.split('\t').count() == COLUMNS.split('\t').count()),
        ),
        (
            "the close date is the author date of the close commit".to_string(),
            has(&r, "alpha", "closed", "2026-01-05T00:00:00+00:00"),
        ),
        (
            "the commits run from the first with a Tracking to the close, another close excluded"
                .to_string(),
            has(&r, "alpha", "commits", "3") && has(&r, "beta", "commits", "1"),
        ),
        (
            "the deliverables are the plan lines of the done file, under Tracking when no Plan"
                .to_string(),
            has(&r, "alpha", "deliverables", "2") && has(&r, "beta", "deliverables", "2"),
        ),
        (
            "the fixes inside the issue are counted".to_string(),
            has(&r, "alpha", "fixes_in_issue", "1") && has(&r, "beta", "fixes_in_issue", "0"),
        ),
        (
            "a later fix of its lines counts, docs, lock, side branch and window excluded"
                .to_string(),
            has(&r, "alpha", "fixes_after", "2") && has(&r, "beta", "fixes_after", "0"),
        ),
        (
            "the window can be widened".to_string(),
            has(&w, "alpha", "fixes_after", "3"),
        ),
        (
            "a negative window and a --projects with no value are refused".to_string(),
            !run(dir, &repo, &["--window-days", "-1"]).0 && !run(dir, &repo, &["--projects"]).0,
        ),
        (
            "reverts by type, by subject and by cited hash are counted".to_string(),
            has(&r, "alpha", "reverts_after", "3"),
        ),
        (
            "the revision is the one in force before the first commit of the issue".to_string(),
            has(&r, "alpha", "revision", "r1") && has(&r, "beta", "revision", "r2"),
        ),
        (
            "no text of a commit or an issue is printed".to_string(),
            !out.contains("SECRET"),
        ),
    ]
}

// spec: QUA-6
fn review_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = fixture(&tmp.join("reviews"));
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        (
            "each Code review line of the Tracking counts a review".to_string(),
            has(&r, "alpha", "reviews", "3") && has(&r, "beta", "reviews", "2"),
        ),
        (
            "the findings of the reviews are summed".to_string(),
            has(&r, "alpha", "review_findings", "7"),
        ),
        (
            "a review whose findings are not counted leaves the sum empty".to_string(),
            has(&r, "beta", "review_findings", ""),
        ),
    ]
}

/// An issue file whose Goal quotes the template in a backtick fence and whose old-layout Tracking holds an indented
/// `~~~` fence and a longer backtick fence, each with a line that would be a heading outside it; its Findings open a
/// line with inline code, not a fence.
const DELTA: &str = "# delta\n\n# Goal\nThe template:\n```md\n# Tracking\n\n## Plan\n1. [x] a\n2. [x] b\n- Code review: 9 findings\n- red: ABC-1: failed\n```\n\n# Findings\n```just check``` fails.\n\n# Tracking\n\n1. [x] one\n   ~~~sh\n# build\n   ```\n   ~~~\n````md\n```\n# not a heading\n````\n2. [x] two\n- Code review: 2 findings\n- red: ABC-2: failed\n\n# Notes\n- Code review: 4 findings\n";

// spec: QUA-11, QUA-12
fn fence_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = tmp.join("fences");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", Some(&toml("r1")))],
    );
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "feat: delta",
        &[
            ("issues/delta.md", Some(DELTA)),
            ("tests/d.rs", Some("// spec: ABC-1, ABC-2\nfn d() {}\n")),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "docs: close delta",
        &[
            ("issues/delta.md", None),
            ("issues/done/delta.md", Some(DELTA)),
        ],
    );
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        (
            "a heading inside a fence neither starts nor ends a section, whatever the fence"
                .to_string(),
            has(&r, "delta", "deliverables", "2")
                && has(&r, "delta", "reviews", "1")
                && has(&r, "delta", "review_findings", "2"),
        ),
        (
            "a red: item quoted in a fence records no red run".to_string(),
            has(&r, "delta", "traced", "2") && has(&r, "delta", "traced_without_red", "1"),
        ),
    ]
}

/// The Goal of an issue quoting the template in a fence, and the Tracking added once its plan is written.
const KAPPA_GOAL: &str = "# kappa\n\n# Goal\nThe template:\n```md\n# Tracking\n```\n";
const KAPPA_TRACKING: &str = "\n# Tracking\n\n## Plan\n1. [x] one\n";

// spec: QUA-2
fn start_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = tmp.join("start");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", Some(&toml("r1")))],
    );
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "docs: open kappa",
        &[("issues/kappa.md", Some(KAPPA_GOAL))],
    );
    let planned = format!("{KAPPA_GOAL}{KAPPA_TRACKING}");
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "docs: plan kappa",
        &[("issues/kappa.md", Some(&planned))],
    );
    commit_at(
        &repo,
        "2026-01-04T00:00:00Z",
        "feat: kappa",
        &[
            ("src/k.rs", Some("k")),
            ("issues/kappa.md", Some(&format!("{planned}done\n"))),
        ],
    );
    commit_at(
        &repo,
        "2026-01-05T00:00:00Z",
        "docs: close kappa",
        &[
            ("issues/kappa.md", None),
            ("issues/done/kappa.md", Some(&planned)),
        ],
    );
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![(
        "a Tracking quoted in a fence does not start the commits of the issue".to_string(),
        has(&r, "kappa", "commits", "3"),
    )]
}

/// An issue file whose Goal fence is left open before its Tracking, and one with no Tracking nor Plan.
const THETA: &str = "# theta\n\n# Goal\n```md\n# Tracking\n\n## Plan\n1. [x] one\n- Code review: 2 findings\n- red: ABC-1: failed\n";
const IOTA: &str = "# iota\n\n# Goal\nClosed with no plan.\n";
/// Issue files whose Goal quotes `## Plan` in a fence, with an older-layout Tracking (`lambda`) or none (`mu`).
const LAMBDA: &str = "# lambda\n\n# Goal\n```md\n## Plan\n1. [x] quoted\n```\n\n# Tracking\n\n1. [x] one\n2. [x] two\n";
const MU: &str = "# mu\n\n# Goal\n```md\n## Plan\n1. [x] quoted\n```\n";

// spec: QUA-12
fn unread_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = tmp.join("unread");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", Some(&toml("r1")))],
    );
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "docs: close theta",
        &[
            ("issues/done/theta.md", Some(THETA)),
            ("tests/t.rs", Some("// spec: ABC-1\nfn t() {}\n")),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "docs: close iota",
        &[
            ("issues/done/iota.md", Some(IOTA)),
            ("issues/done/lambda.md", Some(LAMBDA)),
            ("issues/done/mu.md", Some(MU)),
        ],
    );
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        (
            "a section whose heading stands only inside a fence leaves its columns empty"
                .to_string(),
            [
                "deliverables",
                "reviews",
                "review_findings",
                "traced_without_red",
            ]
            .iter()
            .all(|c| has(&r, "theta", c, ""))
                && has(&r, "theta", "traced", "1"),
        ),
        (
            "a section the file does not hold reads as empty".to_string(),
            [
                "deliverables",
                "reviews",
                "review_findings",
                "traced_without_red",
            ]
            .iter()
            .all(|c| has(&r, "iota", c, "0")),
        ),
        (
            "a Plan quoted in a fence leaves the plan lines to the Tracking, or empty without one"
                .to_string(),
            has(&r, "lambda", "deliverables", "2") && has(&r, "mu", "deliverables", ""),
        ),
    ]
}

/// A repository where `eps` adds three citations, one with a reason, and records the red run of two, `zeta` only renames a test and
/// `eta` extends its citation.
fn traced_fixture(tmp: &Path) -> PathBuf {
    let repo = tmp.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    let test = "// spec: OLD-1\nfn old() {}\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[
            (".claude/harness.toml", Some(&toml("r1"))),
            ("tests/old.rs", Some(test)),
            ("issues/eps.md", Some("# eps\n")),
            ("issues/zeta.md", Some("# zeta\n")),
        ],
    );
    let eps = "# eps\n\n# Tracking\n\n## Plan\n1. [x] one\n\n## Deliverable 1\n- red: ABC-1,\n  ABC-3: assertion failed, see ABC-2\n- Not covered: ABC-2\n- red: none\n\n# Notes\nred: ABC-2\n";
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "feat(files): eps",
        &[
            ("tests/a.rs", Some("// spec: ABC-1, ABC-2\nfn a() {}\n    // spec: ABC-3 (internal: the ABC-7 fixture is slow)\nfn b() {}\n// spec: ABC-1\nfn c() {}\nconst T: &str = \"// spec: ABC-9 in a string\";\n")),
            ("specs/a.md", Some("Cite with // spec: ABC-8 above a test.\n")),
            ("docs/a.md", Some("// spec: ABC-4\n")),
            ("issues/eps.md", Some(eps)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "docs: close eps",
        &[("issues/eps.md", None), ("issues/done/eps.md", Some(eps))],
    );
    let zeta = "# zeta\n\n# Tracking\n\n## Plan\n1. [x] one\n";
    commit_at(
        &repo,
        "2026-01-04T00:00:00Z",
        "refactor(files): move the test",
        &[
            ("tests/old.rs", None),
            ("tests/moved.rs", Some(test)),
            ("issues/zeta.md", Some(zeta)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-05T00:00:00Z",
        "docs: close zeta",
        &[
            ("issues/zeta.md", None),
            ("issues/done/zeta.md", Some(zeta)),
        ],
    );
    let eta = "# eta\n\n# Tracking\n\n## Plan\n1. [x] one\n- red: none (OLD-5 not covered: OLD-6)\n- Not covered: OLD-5\n";
    commit_at(
        &repo,
        "2026-01-06T00:00:00Z",
        "feat(files): extend the citation",
        &[
            (
                "tests/moved.rs",
                Some("// spec: OLD-1, OLD-5\nfn old() {}\n"),
            ),
            ("issues/eta.md", Some(eta)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-06T12:00:00Z",
        "refactor(files): move and reorder the extended citation",
        &[
            ("tests/moved.rs", None),
            (
                "tests/again.rs",
                Some("fn other() {}\n// spec: OLD-5, OLD-1\nfn old() {}\n"),
            ),
            ("issues/eta.md", Some(&format!("{eta}x\n"))),
        ],
    );
    commit_at(
        &repo,
        "2026-01-07T00:00:00Z",
        "docs: close eta",
        &[("issues/eta.md", None), ("issues/done/eta.md", Some(eta))],
    );
    theta_commits(&repo);
    repo
}

/// An issue whose tests cite with the marker of their language (QUA-13).
fn theta_commits(repo: &Path) {
    let theta = "# theta\n\n# Tracking\n\n## Plan\n1. [x] one\n";
    commit_at(
        repo,
        "2026-01-08T00:00:00Z",
        "feat(files): theta",
        &[
            (
                "tests/test_a.py",
                Some("# spec: ABC-1\ndef test_a():\n    pass\n"),
            ),
            (
                "src/lib.rs",
                Some("const T: &str = r\"\n# spec: ABC-2\n\";\n"),
            ),
            ("ci.yml", Some("# spec: ABC-3\n")),
            ("scripts/gen.py", Some("# spec: ABC-3\n")),
            (
                "src/my mod.rs",
                Some("const T: &str = r\"\n# spec: ABC-4\n\";\n"),
            ),
            (
                ".claude/cardinal.toml",
                Some("[project]\nspec_files = [\"*.rs\", \"test_*.py\"]\n"),
            ),
            ("issues/theta.md", Some(theta)),
        ],
    );
    commit_at(
        repo,
        "2026-01-09T00:00:00Z",
        "docs: close theta",
        &[
            ("issues/theta.md", None),
            ("issues/done/theta.md", Some(theta)),
        ],
    );
}

// spec: QUA-11
fn traced_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = traced_fixture(&tmp.join("traced"));
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        (
            "the distinct ids of the added citation lines are traced, docs, prose, strings and reasons excluded".to_string(),
            has(&r, "eps", "traced", "3"),
        ),
        (
            "the traced ids missing from the id list of every red item of the Tracking have no red run".to_string(),
            has(&r, "eps", "traced_without_red", "1"),
        ),
        (
            "a renamed test traces nothing".to_string(),
            has(&r, "zeta", "traced", "0") && has(&r, "zeta", "traced_without_red", "0"),
        ),
        (
            "an extended citation traces its new id, kept when a later commit rewrites it; a red none or a not covered line names none"
                .to_string(),
            has(&r, "eta", "traced", "1") && has(&r, "eta", "traced_without_red", "1"),
        ),
    ]
}

// spec: QUA-13
fn traced_marker_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = traced_fixture(&tmp.join("traced_markers"));
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![(
        "a citation is read with the marker of its file's extension".to_string(),
        has(&r, "theta", "traced", "1"),
    )]
}

/// `n` numbered lines starting at `from`.
fn numbered(from: usize, n: usize) -> String {
    (from..from + n)
        .flat_map(|i| ["line ".to_string(), i.to_string(), "\n".to_string()])
        .collect()
}

/// A repository where `iota` changes specs, docs, ADRs and root files, `kappa` only renames a doc; with `root_docs`,
/// its `.claude/cardinal.toml` lists them.
fn lines_fixture(tmp: &Path, root_docs: Option<&str>) -> PathBuf {
    let repo = tmp.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    let mut sync = vec![
        (".claude/harness.toml", Some(toml("r1"))),
        ("docs/guide.md", Some(numbered(0, 10))),
        ("docs/a.md", Some(numbered(0, 3))),
        ("issues/iota.md", Some("# iota\n".to_string())),
        ("issues/kappa.md", Some("# kappa\n".to_string())),
    ];
    if let Some(list) = root_docs {
        sync.push((
            ".claude/cardinal.toml",
            Some(format!("[project]\nroot_docs = {list}\n")),
        ));
    }
    let sync: Vec<(&str, Option<&str>)> = sync.iter().map(|(p, b)| (*p, b.as_deref())).collect();
    commit_at(&repo, "2026-01-01T00:00:00Z", "chore: sync", &sync);
    let iota = "# iota\n\n# Tracking\n\n## Plan\n1. [x] one\n";
    let guide = format!("{}{}", numbered(0, 5), numbered(100, 5));
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "feat(files): iota",
        &[
            ("specs/a.md", Some(&numbered(0, 40))),
            ("specs/README.md", Some(&numbered(0, 3))),
            ("docs/guide.md", Some(&guide)),
            ("docs/adr/0001-x.md", Some(&numbered(0, 20))),
            ("README.md", Some(&numbered(0, 2))),
            ("CLAUDE.md", Some(&numbered(0, 7))),
            ("PLAN.md", Some(&numbered(0, 4))),
            ("GUIDE", Some(&numbered(0, 3))),
            ("design/arch.md", Some(&numbered(0, 3))),
            ("docs/TODO.md", Some(&numbered(0, 3))),
            ("docs/say \"hi\".md", Some(&numbered(0, 1))),
            ("issues/iota.md", Some(iota)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "docs: close iota",
        &[
            ("issues/iota.md", None),
            ("issues/done/iota.md", Some(iota)),
        ],
    );
    let kappa = "# kappa\n\n# Tracking\n\n## Plan\n1. [x] one\n";
    commit_at(
        &repo,
        "2026-01-04T00:00:00Z",
        "refactor(files): kappa",
        &[
            ("docs/a.md", None),
            ("docs/b.md", Some(&numbered(0, 3))),
            ("issues/kappa.md", Some(kappa)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-05T00:00:00Z",
        "docs: close kappa",
        &[
            ("issues/kappa.md", None),
            ("issues/done/kappa.md", Some(kappa)),
        ],
    );
    repo
}

// spec: QUA-14
fn lines_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (_, out) = run(dir, &lines_fixture(&tmp.join("lines"), None), &[]);
    let r = rows(&out);
    let (_, listed) = run(
        dir,
        &lines_fixture(&tmp.join("lines_root_docs"), Some("[\"PLAN.md\"]")),
        &[],
    );
    let l = rows(&listed);
    let (_, configured) = run(
        dir,
        &lines_fixture(
            &tmp.join("lines_configured"),
            Some("[\"GUIDE\", \"README.md\"]\narchitecture = \"design/arch.md\"\nroadmap = [\"docs/TODO.md\"]"),
        ),
        &[],
    );
    let c = rows(&configured);
    vec![
        (
            "the spec lines leave out the README and the template of specs/".to_string(),
            has(&r, "iota", "spec_lines", "40"),
        ),
        (
            "the doc lines count docs/ outside docs/adr/, a quoted name included, and README.md when no root_docs is listed"
                .to_string(),
            has(&r, "iota", "doc_lines", "16"),
        ),
        (
            "a renamed doc counts no line".to_string(),
            has(&r, "kappa", "doc_lines", "0") && has(&r, "kappa", "spec_lines", "0"),
        ),
        (
            "the root files counted are the root_docs of cardinal.toml".to_string(),
            has(&l, "iota", "doc_lines", "18"),
        ),
        (
            "a root doc that is no markdown and the architecture document count, the roadmap does not".to_string(),
            has(&c, "iota", "doc_lines", "19"),
        ),
    ]
}

/// A transcript line of `kind` in `cwd` at `ts`, with `fields` merged in.
fn line(kind: &str, cwd: &Path, ts: &str, fields: &serde_json::Value) -> String {
    let mut line = serde_json::json!({"type": kind, "cwd": cwd, "timestamp": ts, "sessionId": "x", "gitBranch": "main"});
    line.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    format!("{line}\n")
}

fn typed(cwd: &Path, ts: &str, text: &str) -> String {
    line(
        "user",
        cwd,
        ts,
        &serde_json::json!({"message": {"role": "user", "content": text}}),
    )
}

/// A `Bash` call at `ts` and its result a second later, an error when `error`.
fn bash(cwd: &Path, ts: &str, id: &str, command: &str, error: bool) -> String {
    let call = line(
        "assistant",
        cwd,
        &format!("{ts}:00Z"),
        &serde_json::json!({"message": {"id": id, "content": [{"type": "tool_use", "id": id, "name": "Bash", "input": {"command": command}}]}}),
    );
    let result = line(
        "user",
        cwd,
        &format!("{ts}:01Z"),
        &serde_json::json!({"message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": id, "is_error": error, "content": "SECRET output"}]}}),
    );
    call + &result
}

/// An assistant message `id` at `ts` with the usage `input`, `cache_read`, `cache_creation`, `output`.
fn usage(cwd: &Path, ts: &str, id: &str, tokens: [u64; 4]) -> String {
    let [input, cache_read, cache_creation, output] = tokens;
    line(
        "assistant",
        cwd,
        ts,
        &serde_json::json!({"message": {"id": id, "content": [{"type": "text", "text": "SECRET answer"}], "usage": {"input_tokens": input, "cache_read_input_tokens": cache_read, "cache_creation_input_tokens": cache_creation, "output_tokens": output}}}),
    )
}

/// An advised assistant message `id` at `ts`: a top-level usage of 1 input and 1 output token, and iterations whose
/// tokens it already adds up (the executor's) or never holds (the advisor's).
fn advised(cwd: &Path, ts: &str, id: &str) -> String {
    let iteration = |kind: &str| serde_json::json!({"type": kind, "input_tokens": 500, "cache_read_input_tokens": 500, "output_tokens": 500});
    line(
        "assistant",
        cwd,
        ts,
        &serde_json::json!({"message": {"id": id, "content": [], "usage": {"input_tokens": 1, "output_tokens": 1, "iterations": [iteration("message"), iteration("advisor_message")]}}}),
    )
}

fn write(path: &Path, lines: &[String]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.concat()).unwrap();
}

// spec: QUA-7, QUA-8, QUA-9, QUA-15
fn session_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("sessions");
    let repo = fixture(&tmp);
    let projects = tmp.join("projects");
    // S0, on another branch, starts after S1 and holds the first commit of alpha too: S1, on alpha's branch, wins.
    write(
        &projects.join("-proj/s0.jsonl"),
        &[
            typed(&repo, "2026-01-02T20:00:00Z", "SECRET early"),
            bash(&repo, "2026-01-03T10:00", "s0a", "just check", true),
        ],
    );
    // S1 runs on the branch of alpha from before the issues were opened: all its span up to alpha's last commit counts.
    let s1 = [
        typed(&repo, "2026-01-01T23:00:00Z", "SECRET prepare alpha"),
        typed(&repo, "2026-01-02T23:00:00Z", "SECRET do alpha"),
        usage(&repo, "2026-01-02T23:30:00Z", "u1", [10, 1000, 200, 50]),
        usage(&repo, "2026-01-02T23:31:00Z", "u1", [10, 1000, 200, 50]),
        advised(&repo, "2026-01-02T23:35:00Z", "u4"),
        bash(&repo, "2026-01-03T01:00", "a1", "just check", true),
        bash(&repo, "2026-01-03T02:00", "a2", "just test-harness", false),
        bash(&repo, "2026-01-03T03:00", "a3", "cargo check", true),
        bash(&repo, "2026-01-03T03:30", "a4", "echo just a check", true),
        bash(
            &repo,
            "2026-01-04T05:00",
            "a5",
            "cd x && just check-harness",
            true,
        ),
        typed(&repo, "2026-01-04T08:00:00Z", "ok"),
        bash(&repo, "2026-01-04T09:00", "a6", "just check", true),
        typed(&repo, "2026-01-05T00:30:00Z", "1 a, 2 b"),
        typed(&repo, "2026-01-05T01:00:00Z", "SECRET after"),
        usage(&repo, "2026-01-05T00:45:00Z", "u2", [1000, 0, 0, 1000]),
    ];
    write(
        &projects.join("-proj/s1.jsonl"),
        &s1.map(|l| l.replace("\"gitBranch\":\"main\"", "\"gitBranch\":\"alpha\"")),
    );
    write(
        &projects.join("-proj/s1/subagents/agent-1.jsonl"),
        &[
            bash(&repo, "2026-01-03T04:00", "b1", "just check", true),
            usage(&repo, "2026-01-02T23:40:00Z", "u3", [5, 0, 0, 7]),
        ],
    );
    // delta, closed after every session ended, has no session: its tokens are unknown
    commit_at(
        &repo,
        "2026-03-01T00:00:00Z",
        "docs: close delta",
        &[("issues/done/delta.md", Some("# delta\n\n# Tracking\n"))],
    );
    let (ok, out) = run(dir, &repo, &["--projects", projects.to_str().unwrap()]);
    let r = rows(&out);
    let (_, summed) = run(
        dir,
        &repo,
        &["--projects", projects.to_str().unwrap(), "--summary"],
    );
    let tail = |revision: &str| {
        summed
            .lines()
            .find(|l| l.starts_with(&format!("{revision}\t")))
            .map(|l| l.split('\t').rev().take(4).collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    };
    let (_, none) = run(dir, &repo, &[]);
    let n = rows(&none);
    vec![
        ("the script reads the transcripts".to_string(), ok),
        ("the sessions of an issue are the latest-starting ones holding its commits, its branch first".to_string(), has(&r, "alpha", "sessions", "1") && has(&r, "beta", "sessions", "1")),
        ("the failed just checks of its span are counted, not other commands nor sub-agents".to_string(), has(&r, "alpha", "failed_checks", "3") && has(&r, "beta", "failed_checks", "1")),
        ("the typed messages of its span are split into validations and the rest".to_string(), has(&r, "alpha", "validations", "1") && has(&r, "alpha", "other_messages", "2") && has(&r, "beta", "validations", "1") && has(&r, "beta", "other_messages", "0")),
        ("the top-level tokens of its span are summed once per message, sub-agents included, not the iterations".to_string(), has(&r, "alpha", "input_tokens", "1216") && has(&r, "alpha", "cache_read_tokens", "1000") && has(&r, "alpha", "output_tokens", "58") && has(&r, "beta", "input_tokens", "0")),
        ("the tokens of an issue with no session are empty, not 0".to_string(), has(&r, "delta", "sessions", "0") && ["input_tokens", "cache_read_tokens", "output_tokens"].iter().all(|c| has(&r, "delta", c, ""))),
        ("the summary sums the known tokens and counts their issues".to_string(), tail("r1") == "1 58 1000 1216" && tail("r3") == "0 0 0 0"),
        ("the transcript columns are empty when the projects cannot be read".to_string(), ["sessions", "failed_checks", "validations", "other_messages", "input_tokens", "cache_read_tokens", "output_tokens"].iter().all(|c| has(&n, "alpha", c, ""))),
        ("no text of a message is printed".to_string(), !out.contains("SECRET")),
    ]
}

/// An assistant message `id` at `ts` of `model`, with `advisor` set as its advisor model when not empty.
fn modeled(cwd: &Path, ts: &str, id: &str, model: &str, advisor: &str) -> String {
    let mut fields = serde_json::json!({"message": {"id": id, "model": model, "content": [], "usage": {"input_tokens": 1, "output_tokens": 1}}});
    if !advisor.is_empty() {
        fields["advisorModel"] = advisor.into();
    }
    line("assistant", cwd, ts, &fields)
}

/// The repository of `fixture` and its sessions under several setups, with two more issues closed in `r3`.
fn setup_fixture(tmp: &Path) -> (PathBuf, PathBuf) {
    let repo = fixture(tmp);
    let projects = tmp.join("projects");
    let (sonnet, opus) = ("claude-sonnet-5-5", "claude-opus-5-5");
    let on_alpha = |lines: Vec<String>| -> Vec<String> {
        lines
            .iter()
            .map(|l| l.replace("\"gitBranch\":\"main\"", "\"gitBranch\":\"alpha\""))
            .collect()
    };
    // alpha: two sessions of Sonnet advised by Opus, one with a Haiku sub-agent, then one of Opus alone: mixed.
    write(
        &projects.join("-proj/sa.jsonl"),
        &on_alpha(vec![
            modeled(&repo, "2026-01-02T22:00:00Z", "a1", sonnet, opus),
            modeled(&repo, "2026-01-03T01:00:00Z", "a2", sonnet, opus),
        ]),
    );
    write(
        &projects.join("-proj/sb.jsonl"),
        &on_alpha(vec![
            modeled(&repo, "2026-01-03T22:00:00Z", "b1", sonnet, opus),
            modeled(&repo, "2026-01-04T01:00:00Z", "b2", sonnet, opus),
        ]),
    );
    write(
        &projects.join("-proj/sb/subagents/agent-1.jsonl"),
        &[modeled(
            &repo,
            "2026-01-03T23:00:00Z",
            "b3",
            "claude-haiku-5-5",
            "",
        )],
    );
    write(
        &projects.join("-proj/sc.jsonl"),
        &on_alpha(vec![
            modeled(&repo, "2026-01-04T23:00:00Z", "c1", opus, ""),
            modeled(&repo, "2026-01-05T01:00:00Z", "c2", opus, ""),
        ]),
    );
    // beta: one session of Sonnet advised by Opus holds its close commit.
    write(
        &projects.join("-proj/sd.jsonl"),
        &[
            modeled(&repo, "2026-01-04T11:00:00Z", "d1", sonnet, opus),
            modeled(&repo, "2026-01-04T13:00:00Z", "d2", sonnet, opus),
        ],
    );
    // in r3, epsilon has a session of Opus with an Opus advisor set, alone all the same, after a failed request, and delta none: two rows of the summary for one revision; gamma, left in
    // the work tree, would be closed with them.
    std::fs::remove_file(repo.join("issues/done/gamma.md")).unwrap();
    commit_at(
        &repo,
        "2026-03-01T00:00:00Z",
        "docs: close epsilon",
        &[("issues/done/epsilon.md", Some("# epsilon\n\n# Tracking\n"))],
    );
    write(
        &projects.join("-proj/se.jsonl"),
        &[
            modeled(&repo, "2026-02-28T22:00:00Z", "e0", "<synthetic>", ""),
            modeled(&repo, "2026-02-28T23:00:00Z", "e1", opus, opus),
            modeled(&repo, "2026-03-01T01:00:00Z", "e2", opus, ""),
        ],
    );
    commit_at(
        &repo,
        "2026-03-02T00:00:00Z",
        "docs: close delta",
        &[("issues/done/delta.md", Some("# delta\n\n# Tracking\n"))],
    );
    (repo, projects)
}

// spec: QUA-16, QUA-17
fn setup_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (repo, projects) = setup_fixture(&tmp.join("setups"));
    let projects_arg = projects.to_str().unwrap();
    let (ok, out) = run(dir, &repo, &["--projects", projects_arg]);
    let r = rows(&out);
    let (_, summed) = run(dir, &repo, &["--projects", projects_arg, "--summary"]);
    let heads: Vec<String> = summed
        .lines()
        .map(|l| l.split('\t').take(3).collect::<Vec<_>>().join("|"))
        .collect();
    let (_, none) = run(dir, &repo, &[]);
    vec![
        ("the setups are read".to_string(), ok),
        (
            "an issue made under several setups is mixed".to_string(),
            has(&r, "alpha", "setup", "mixed") && has(&r, "alpha", "sessions", "3"),
        ),
        (
            "an issue made under one setup gives it, the main model first".to_string(),
            has(
                &r,
                "beta",
                "setup",
                "claude-sonnet-5-5 advised by claude-opus-5-5",
            ),
        ),
        (
            "an issue with no session has no setup".to_string(),
            has(&r, "delta", "setup", "") && has(&r, "epsilon", "setup", "claude-opus-5-5 solo"),
        ),
        (
            "the setup is empty when the projects cannot be read".to_string(),
            has(&rows(&none), "beta", "setup", ""),
        ),
        (
            "the summary has one row per revision and setup, the empty setup first".to_string(),
            heads
                == [
                    "revision|setup|issues",
                    "r1|mixed|1",
                    "r2|claude-sonnet-5-5 advised by claude-opus-5-5|1",
                    "r3||1",
                    "r3|claude-opus-5-5 solo|1",
                ],
        ),
    ]
}

// spec: QUA-9
fn summary_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("summary");
    let repo = fixture(&tmp);
    // gamma, closed in the revision of beta, makes r2 a sum of two issues; its copy left in the work tree goes first.
    std::fs::remove_file(repo.join("issues/done/gamma.md")).unwrap();
    commit_at(
        &repo,
        "2026-02-01T00:00:00Z",
        "chore: unpin back",
        &[(".claude/harness.toml", Some(&toml("r2")))],
    );
    commit_at(
        &repo,
        "2026-02-02T00:00:00Z",
        "feat(files): gamma",
        &[
            (
                "issues/done/gamma.md",
                Some("# gamma\n\n# Tracking\n\n1. [x] one\n- Code review: 2 findings\n"),
            ),
            ("src/g.rs", Some("g")),
        ],
    );
    // delta, in a revision named before the others, comes last: the revisions are in the order they were used.
    commit_at(
        &repo,
        "2026-02-03T00:00:00Z",
        "chore: pin",
        &[(".claude/harness.toml", Some(&toml("a0")))],
    );
    commit_at(
        &repo,
        "2026-02-04T00:00:00Z",
        "feat(files): delta",
        &[(
            "issues/done/delta.md",
            Some("# delta\n\n# Tracking\n\n1. [x] one\n"),
        )],
    );
    let (ok, out) = run(dir, &repo, &["--summary"]);
    let expected = [
        "revision\tsetup\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\ttraced\ttraced_without_red\tspec_lines\tdoc_lines\tsessions\tfailed_checks\tvalidations\tother_messages\tinput_tokens\tcache_read_tokens\toutput_tokens\ttoken_issues",
        "r1\t\t1\t2\t3\t1\t2\t3\t3\t7\t0\t0\t0\t1\t\t\t\t\t0\t0\t0\t0",
        "r2\t\t2\t3\t2\t0\t0\t0\t3\t\t0\t0\t0\t0\t\t\t\t\t0\t0\t0\t0",
        "a0\t\t1\t1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t\t\t0\t0\t0\t0",
    ];
    let lines: Vec<&str> = out.lines().collect();
    if lines != expected {
        println!("{out}");
    }
    vec![("the summary sums the columns per revision in the order they were used, an empty value leaving its sum empty".to_string(), ok && lines == expected)]
}

/// Two issues sharing `src/a.rs`, renamed `src/b.rs` after both closed, and the fixes of the lines of each: `alpha`
/// wrote `x1` to `x3`, `beta` `y4` and `src/c "q".rs` (a name git quotes); the line `-- x0`, written before them,
/// reads `--- x0` in the diff of the fix that removes it.
fn blame_fixture(tmp: &Path) -> PathBuf {
    let repo = tmp.join("blame");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    let (alpha, beta) = ("# alpha\n\n# Tracking\n", "# beta\n\n# Tracking\n");
    let steps = [
        (
            "2026-02-01T00:00:00Z",
            "docs: open the issues",
            vec![
                ("issues/alpha.md", Some("# alpha\n")),
                ("issues/beta.md", Some("# beta\n")),
                ("src/a.rs", Some("-- x0\n")),
            ],
        ),
        (
            "2026-02-02T00:00:00Z",
            "feat: alpha",
            vec![
                ("src/a.rs", Some("-- x0\nx1\nx2\nx3\n")),
                ("issues/alpha.md", Some(alpha)),
            ],
        ),
        (
            "2026-02-03T00:00:00Z",
            "docs: close alpha",
            vec![
                ("issues/alpha.md", None),
                ("issues/done/alpha.md", Some(alpha)),
            ],
        ),
        (
            "2026-02-04T00:00:00Z",
            "feat: beta",
            vec![
                ("src/a.rs", Some("-- x0\nx1\nx2\nx3\ny4\n")),
                ("src/c \"q\".rs", Some("c1\nc2\n")),
                ("issues/beta.md", Some(beta)),
            ],
        ),
        (
            "2026-02-05T00:00:00Z",
            "docs: close beta",
            vec![
                ("issues/beta.md", None),
                ("issues/done/beta.md", Some(beta)),
            ],
        ),
        (
            "2026-02-05T12:00:00Z",
            "fix: a line of alpha in a file of beta",
            vec![("src/a.rs", Some("x1\nw2\nx3\ny4\n"))],
        ),
        (
            "2026-02-06T00:00:00Z",
            "refactor: rename",
            vec![("src/a.rs", None), ("src/b.rs", Some("x1\nw2\nx3\ny4\n"))],
        ),
        (
            "2026-02-07T00:00:00Z",
            "fix: a line of alpha",
            vec![("src/b.rs", Some("z1\nw2\nx3\ny4\n"))],
        ),
        (
            "2026-02-08T00:00:00Z",
            "fix: insert below a line of beta",
            vec![("src/c \"q\".rs", Some("c1\nnew\nc2\n"))],
        ),
        (
            "2026-02-09T00:00:00Z",
            "fix: insert at the top",
            vec![("src/c \"q\".rs", Some("top\nc1\nnew\nc2\n"))],
        ),
    ];
    for (date, message, files) in steps {
        commit_at(&repo, date, message, &files);
    }
    repo
}

// spec: QUA-4
fn blame_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = blame_fixture(tmp);
    let counted = |out: &str| {
        let r = rows(out);
        [
            has(&r, "alpha", "fixes_after", "2"),
            has(&r, "beta", "fixes_after", "1"),
        ] == [true; 2]
    };
    let (_, out) = run(dir, &repo, &[]);
    // a user's configuration that changes the diff headers and makes blame fail in a repository without the file
    let config = tmp.join("gitconfig");
    std::fs::write(
        &config,
        "[diff]\n\tnoprefix = true\n[blame]\n\tignoreRevsFile = .no-such-file\n",
    )
    .unwrap();
    let mut cmd = tool_command(dir.join("delivered_quality.rs"));
    cmd.arg(&repo)
        .arg("--projects")
        .arg(repo.join("no-projects"))
        .env("GIT_CONFIG_GLOBAL", &config);
    let configured = String::from_utf8_lossy(&cmd.output().unwrap().stdout).into_owned();
    vec![
        (
            "a fix counts only against the issue that wrote its lines, across a rename, an insertion by the line above it"
                .to_string(),
            counted(&out),
        ),
        (
            "the user's git configuration leaves the count unchanged".to_string(),
            counted(&configured),
        ),
    ]
}

#[test]
fn delivered_quality() {
    let dir = std::path::PathBuf::from("/tools");
    let scratch = tempfile::tempdir().unwrap();
    let tmp = scratch.path().to_path_buf();
    let checks: Vec<_> = git_checks(&dir, &tmp)
        .into_iter()
        .chain(blame_checks(&dir, &tmp))
        .chain(review_checks(&dir, &tmp))
        .chain(fence_checks(&dir, &tmp))
        .chain(start_checks(&dir, &tmp))
        .chain(unread_checks(&dir, &tmp))
        .chain(traced_checks(&dir, &tmp))
        .chain(traced_marker_checks(&dir, &tmp))
        .chain(lines_checks(&dir, &tmp))
        .chain(session_checks(&dir, &tmp))
        .chain(summary_checks(&dir, &tmp))
        .chain(setup_checks(&dir, &tmp))
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
