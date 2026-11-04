//! `update`, and `sync` on a pinned commit, run as the binary on a local git source (SYN-11 to SYN-17).
#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::Command;

use common::{get, put, record_hook, snapshot, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

/// A git repository with `v1` on its first commit and `main` on its second; returns both hashes.
fn git_source(home: &Home) -> (TempDir, String, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/a.txt", "a1");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    home.git(repo.path(), &["tag", "v1"]);
    let first = home.git(repo.path(), &["rev-parse", "HEAD"]);
    put(repo.path(), "files/a.txt", "a2");
    home.git(repo.path(), &["commit", "--quiet", "-am", "two"]);
    let second = home.git(repo.path(), &["rev-parse", "HEAD"]);
    (repo, first, second)
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

/// A project whose configuration points at `repo` under a comment of its own.
fn git_project(repo: &Path) -> TempDir {
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("# mine\n[cardinal]\nsource = \"{}\"\n", url(repo)),
    );
    root
}

/// A project moved to `v1` by `update`, with its repository and the hash of `v1`.
fn updated_to_v1(home: &Home) -> (TempDir, String, TempDir) {
    let (repo, first, _second) = git_source(home);
    let root = git_project(repo.path());
    home.ok(root.path(), &["update", "--rev", "v1"]);
    (repo, first, root)
}

// spec: SYN-11, SYN-16
#[test]
fn update_pins_the_resolved_commit_then_syncs_and_reports_both_revisions() {
    let home = Home::new();
    let (repo, first, second) = git_source(&home);
    let root = git_project(repo.path());

    let run = home.ok(root.path(), &["update", "--rev", "v1"]);
    assert_eq!(
        run.lines(),
        [format!("rev: (none) -> {first}"), "added: a.txt".into()]
    );
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    let config = get(root.path(), CONFIG).unwrap();
    assert!(config.starts_with("# mine\n"), "{config}");
    assert!(
        config.contains(&format!("rev = \"{first}\"  # v1\n")),
        "{config}"
    );

    let run = home.ok(root.path(), &["update"]);
    assert_eq!(
        run.lines(),
        [format!("rev: {first} -> {second}"), "updated: a.txt".into()]
    );
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    let config = get(root.path(), CONFIG).unwrap();
    assert!(
        config.contains(&format!("rev = \"{second}\"  # default branch\n")),
        "{config}"
    );

    // already there: `rev` is rewritten as it was and nothing else happens
    let before = snapshot(root.path());
    let run = home.ok(root.path(), &["update"]);
    assert_eq!(run.lines(), [format!("rev: {second} -> {second}")]);
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-11
#[test]
fn update_resolves_a_branch_a_tag_and_a_commit_to_a_full_hash() {
    let home = Home::new();
    let (repo, first, second) = git_source(&home);
    let root = git_project(repo.path());
    let rev_line = |hash: &str, comment: &str| format!("rev = \"{hash}\"{comment}\n");
    for (rev, line) in [
        ("main", rev_line(&second, "  # main")),
        ("v1", rev_line(&first, "  # v1")),
        // a commit needs no comment naming it again
        (first.as_str(), rev_line(&first, "")),
    ] {
        home.ok(root.path(), &["update", "--rev", rev]);
        let config = get(root.path(), CONFIG).unwrap();
        assert!(config.contains(&line), "{rev}: {config}");
    }
}

// spec: SYN-12
#[test]
fn update_changes_only_the_rev_line() {
    let home = Home::new();
    let (repo, first, _second) = git_source(&home);
    let root = tempdir().unwrap();
    let source = url(repo.path());
    let text = |rev: &str| {
        format!("# mine\n[cardinal]\nsource = \"{source}\" # why\nrev = {rev}\n\n[project]\nrev = \"keep\"\n")
    };
    put(root.path(), CONFIG, &text("\"old\"  # v0"));
    home.ok(root.path(), &["update", "--rev", "v1"]);
    assert_eq!(
        get(root.path(), CONFIG),
        Some(text(&format!("\"{first}\"  # v1")))
    );
}

// spec: SYN-12
#[test]
fn update_inserts_a_missing_rev_right_after_source() {
    let home = Home::new();
    let (repo, first, _second) = git_source(&home);
    let root = tempdir().unwrap();
    let source = url(repo.path());
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{source}\"\n\n[project]\na = 1\n"),
    );
    home.ok(root.path(), &["update", "--rev", "v1"]);
    assert_eq!(
        get(root.path(), CONFIG),
        Some(format!(
            "[cardinal]\nsource = \"{source}\"\nrev = \"{first}\"  # v1\n\n[project]\na = 1\n"
        ))
    );
}

// spec: SYN-12
#[test]
fn update_fails_changing_nothing_on_a_quoted_or_dotted_key() {
    let home = Home::new();
    let (repo, _first, _second) = git_source(&home);
    let source = url(repo.path());
    for (config, error) in [
        (
            format!("[cardinal]\nsource = \"{source}\"\n\"rev\" = \"old\"\n"),
            "cannot rewrite `rev`",
        ),
        (
            format!("harness.source = \"{source}\"\nharness.rev = \"old\"\n"),
            "missing `source`",
        ),
    ] {
        let root = tempdir().unwrap();
        put(root.path(), CONFIG, &config);
        let before = snapshot(root.path());
        let run = home.harness(root.path(), &["update", "--rev", "v1"]);
        assert_eq!(run.code, Some(3), "{config}: {run:?}");
        assert!(run.stderr.contains(error), "{config}: {run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-13
#[test]
fn update_stops_on_a_conflict_unless_forced_and_writes_nothing() {
    let home = Home::new();
    let (_repo, first, root) = updated_to_v1(&home);
    put(root.path(), "a.txt", "local");
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.outcome("a.txt"), Some("conflict"));
    assert!(run.stdout.contains("conflicts: nothing written"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);

    home.ok(root.path(), &["update", "--force"]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert!(!get(root.path(), CONFIG).unwrap().contains(&first));
}

// spec: SYN-13
#[test]
fn update_to_an_unknown_revision_or_an_unreadable_source_changes_nothing() {
    let home = Home::new();
    let (repo, _first, root) = updated_to_v1(&home);
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update", "--rev", "nope"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("unknown revision"), "{run:?}");

    // a revision without files/: the source cannot be read as a harness
    home.git(repo.path(), &["rm", "-rq", "files"]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "empty"]);
    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("no files/ directory"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);

    let gone = git_project(&tempdir().unwrap().path().join("gone"));
    let gone_before = snapshot(gone.path());
    let run = home.harness(gone.path(), &["update"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert_eq!(snapshot(gone.path()), gone_before);
}

// spec: SYN-14
#[test]
fn update_refuses_a_local_directory_source_and_changes_nothing() {
    let home = Home::new();
    let source = tempdir().unwrap();
    put(source.path(), SOURCE_FORMAT, FORMAT);
    put(source.path(), "files/a.txt", "a1");
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    let before = snapshot(root.path());
    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("local directory"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
    assert!(get(root.path(), LOCK).is_none());
}

// spec: SYN-49
#[test]
fn a_source_ending_in_dot_git_is_cloned_and_any_other_ending_is_a_local_directory() {
    let home = Home::new();
    let (repo, _first, second) = git_source(&home);
    let source_at = |source: &str| {
        let root = tempdir().unwrap();
        put(
            root.path(),
            CONFIG,
            &format!("[cardinal]\nsource = \"{source}\"\n"),
        );
        root
    };

    let dot_git = format!("{}/.git", repo.path().display());
    let cloned = source_at(&dot_git);
    let run = home.ok(cloned.path(), &["update"]);
    assert_eq!(
        run.lines()[0],
        format!("rev: (none) -> {second}"),
        "{run:?}"
    );
    let initialized = tempdir().unwrap();
    home.ok(initialized.path(), &["init", &dot_git, "--rev", "v1"]);

    let dirs = tempdir().unwrap();
    for local in ["x.git/", "X.GIT"] {
        put(&dirs.path().join(local), "files/a.txt", "a1");
        let root = source_at(&format!("{}/{local}", dirs.path().display()));
        let run = home.harness(root.path(), &["update"]);
        assert_eq!(run.code, Some(3), "{local}: {run:?}");
        assert!(run.stderr.contains("local directory"), "{local}: {run:?}");
    }
}

// spec: SYN-50
#[test]
fn a_relative_git_source_is_the_repository_next_to_its_own_project() {
    let home = Home::new();
    let mut projects = Vec::new();
    for text in ["one", "two"] {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("x.git");
        home.git(dir.path(), &["init", "--quiet", "-b", "main", "x.git"]);
        put(&repo, "files/a.txt", text);
        put(&repo, SOURCE_FORMAT, FORMAT);
        home.git(&repo, &["add", "."]);
        home.git(&repo, &["commit", "--quiet", "-m", text]);
        let root = dir.path().join("project");
        home.ok(&root, &["init", "../x.git"]);
        projects.push((dir, root, text));
    }
    // the second project fetches its own repository, not the clone of the first
    for (_dir, root, text) in &projects {
        let run = home.ok(root, &["sync"]);
        assert_eq!(run.outcome("a.txt"), Some("added"), "{run:?}");
        assert_eq!(get(root, "a.txt").as_deref(), Some(*text));
    }
}

// spec: SYN-15
#[test]
fn update_dry_run_reports_the_target_and_writes_nothing() {
    let home = Home::new();
    let (_repo, first, root) = updated_to_v1(&home);
    let before = snapshot(root.path());
    let run = home.ok(root.path(), &["update", "--dry-run"]);
    let target = run.lines()[0].to_string();
    assert!(target.starts_with(&format!("rev: {first} -> ")), "{run:?}");
    assert!(!target.ends_with(&first), "{run:?}");
    assert_eq!(run.outcome("a.txt"), Some("updated"));
    assert!(run.stdout.contains("dry run: nothing written"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-61
#[test]
fn update_names_the_commit_skill_as_the_next_step_once_it_wrote_a_change() {
    const NEXT: &str = "next: commit the update with skill:commit-message";
    const SKILL: &str = "files/.claude/skills/commit-message/SKILL.md";
    let home = Home::new();
    let (repo, first, _second) = git_source(&home);
    put(repo.path(), SKILL, "skill");
    put(repo.path(), "files/a.txt", "a3");
    // a source managing the review gate, which `sync` reports skipped at each run under `core.hooksPath`
    let fragment = serde_json::json!({"hooks": record_hook()});
    put(
        repo.path(),
        "entries/.claude/settings.json",
        &fragment.to_string(),
    );
    put(repo.path(), "seed/.claude/settings.json", "{}");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "three"]);
    home.git(repo.path(), &["tag", "v3"]);
    let root = git_project(repo.path());
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.git(root.path(), &["config", "core.hooksPath", "hooks"]);

    // the first revision has no skill: no next step
    let run = home.ok(root.path(), &["update", "--rev", &first]);
    assert!(!run.stdout.contains(NEXT), "{run:?}");

    let run = home.ok(root.path(), &["update", "--dry-run"]);
    assert!(!run.stdout.contains(NEXT), "{run:?}");

    let run = home.ok(root.path(), &["update"]);
    assert_eq!(run.lines().last(), Some(&NEXT), "{run:?}");

    // already there, nothing changed but the skipped gate
    let run = home.ok(root.path(), &["update"]);
    assert_eq!(
        run.outcome("native review gate"),
        Some("skipped"),
        "{run:?}"
    );
    assert!(!run.stdout.contains(NEXT), "{run:?}");

    // the same commit under another name: only the comment of `rev` changes
    let run = home.ok(root.path(), &["update", "--rev", "v3"]);
    assert_eq!(run.lines().last(), Some(&NEXT), "{run:?}");

    // a conflict writes nothing
    put(root.path(), "a.txt", "mine");
    put(repo.path(), "files/a.txt", "a4");
    home.git(repo.path(), &["commit", "--quiet", "-am", "four"]);
    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(run.outcome("a.txt"), Some("conflict"));
    assert!(!run.stdout.contains(NEXT), "{run:?}");
}

/// A source whose pinned commit `first` no branch or tag points to any more; `kept` tells whether the
/// source still holds it (else it was pruned).
fn rewritten_source(home: &Home, kept: bool) -> (TempDir, String) {
    let (repo, first, _second) = git_source(home);
    home.git(repo.path(), &["tag", "-d", "v1"]);
    home.git(
        repo.path(),
        &["checkout", "--quiet", "--orphan", "rewritten"],
    );
    put(repo.path(), "files/a.txt", "a3");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "three"]);
    home.git(repo.path(), &["branch", "-M", "main"]);
    home.git(repo.path(), &["reflog", "expire", "--expire=now", "--all"]);
    if !kept {
        home.git(repo.path(), &["gc", "--quiet", "--prune=now"]);
        let mut cat = Command::new("git");
        cat.args(["cat-file", "-e", &format!("{first}^{{commit}}")])
            .current_dir(repo.path());
        let still = home.isolated(&mut cat).output().unwrap().status;
        // CI served the commit after the gc alone: fail here, not in the test, if it is still there
        assert!(!still.success(), "the commit is still in the source");
    }
    (repo, first)
}

/// A project pinned to `rev` of `repo` that was never synced.
fn pinned(repo: &Path, rev: &str) -> TempDir {
    let root = git_project(repo);
    let config = get(root.path(), CONFIG).unwrap();
    put(root.path(), CONFIG, &format!("{config}rev = \"{rev}\"\n"));
    root
}

// spec: SYN-17
#[test]
fn a_pinned_hash_no_ref_points_to_any_more_is_fetched_by_hash() {
    let home = Home::new();
    let (repo, first) = rewritten_source(&home, true);
    let root = pinned(repo.path(), &first);
    home.ok(root.path(), &["sync"]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    // a home of its own: the cache of the sync above holds the commit already, `update` must fetch it
    let home = Home::new();
    let root = git_project(repo.path());
    home.ok(root.path(), &["update", "--rev", &first]);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

// spec: SYN-17
#[test]
fn a_pinned_hash_the_source_does_not_serve_fails_naming_update_and_changes_nothing() {
    let home = Home::new();
    let (repo, first) = rewritten_source(&home, false);
    let root = pinned(repo.path(), &first);
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["update", "--rev", &first]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?}: {run:?}");
        assert!(
            run.stderr.contains("gone from the source"),
            "{args:?}: {run:?}"
        );
        assert!(run.stderr.contains("cardinal update"), "{args:?}: {run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
    // an abbreviated hash or a name is not fetched by hash: the error stays the plain one
    for rev in [&first[..12], "nope"] {
        let run = home.harness(root.path(), &["update", "--rev", rev]);
        assert_eq!(run.code, Some(3), "{rev}: {run:?}");
        assert!(run.stderr.contains("unknown revision"), "{rev}: {run:?}");
        assert!(
            !run.stderr.contains("gone from the source"),
            "{rev}: {run:?}"
        );
    }
}
