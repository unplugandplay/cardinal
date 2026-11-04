//! The budget of the fixed context of `check`, `sync` and `update`, run as the binary on temp dirs (SYN-38, SYN-39).
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, Home, Run, CONFIG, FORMAT, SOURCE_FORMAT};
use tempfile::tempdir;

/// A skill whose front matter counts 1 + 9 = 10 bytes, with a body loaded on demand that counts for nothing.
const SKILL: &str = "---\nname: a\ndescription: \"012345678\"\n---\n\nA long body the session loads on demand only.\n";

/// The lines of stderr about the budget.
fn budget(run: &Run) -> Vec<&str> {
    run.stderr
        .lines()
        .filter(|l| l.contains("fixed_context_budget"))
        .collect()
}

fn over(total: u64, budget: u64, part: &str, bytes: u64) -> String {
    format!(
        "warning: the fixed context is {total} bytes, over the budget of {budget} (fixed_context_budget in .claude/cardinal.toml); the largest part is {part} ({bytes} bytes)"
    )
}

const NOT_A_NUMBER: &str =
    "warning: fixed_context_budget in .claude/cardinal.toml is not a whole number of bytes";

/// Sets `fixed_context_budget = <value>` in the `[project]` table of the project's configuration, replacing the
/// previous value.
fn set_budget(root: &Path, value: &str) {
    let config: String = get(root, CONFIG)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("fixed_context_budget"))
        .map(|l| l.to_string() + "\n")
        .collect();
    let line = format!("fixed_context_budget = {value}\n");
    let config = if config.contains("[project]\n") {
        config.replacen("[project]\n", &format!("[project]\n{line}"), 1)
    } else {
        format!("{config}\n[project]\n{line}")
    };
    put(root, CONFIG, &config);
}

/// A `CLAUDE.md` of `bytes` bytes that imports `AGENTS.md`.
fn importing(bytes: usize) -> String {
    format!("@AGENTS.md\n{}", "c".repeat(bytes - 11))
}

/// A synced project of 30 bytes of `CLAUDE.md`, 20 of the `AGENTS.md` it imports and 10 of skill descriptions.
fn measured(home: &Home) -> (tempfile::TempDir, tempfile::TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), "CLAUDE.md", &importing(30));
    put(root.path(), "AGENTS.md", &"a".repeat(20));
    put(root.path(), ".claude/skills/a/SKILL.md", SKILL);
    (source, root)
}

// spec: SYN-38
#[test]
fn check_sync_and_dry_run_warn_once_when_the_fixed_context_exceeds_the_budget() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "59");
    let warning = over(60, 59, "CLAUDE.md", 30);
    for args in [&["check"][..], &["sync"], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        assert_eq!(budget(&run), [warning.as_str()], "{args:?}: {run:?}");
    }
}

// spec: SYN-38
#[test]
fn no_warning_at_or_under_the_budget_nor_without_the_key() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    let run = home.ok(root.path(), &["check"]);
    assert!(budget(&run).is_empty(), "{run:?}");
    set_budget(root.path(), "60");
    let run = home.ok(root.path(), &["check"]);
    assert!(budget(&run).is_empty(), "{run:?}");
}

// spec: SYN-38
#[test]
fn the_largest_part_wins_its_ties_in_the_order_claude_agents_skills() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    put(root.path(), "AGENTS.md", &"a".repeat(30));
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(budget(&run), [over(70, 0, "CLAUDE.md", 30).as_str()]);
    std::fs::remove_file(root.path().join("CLAUDE.md")).unwrap();
    put(root.path(), ".claude/skills/b/SKILL.md", SKILL);
    put(root.path(), ".claude/skills/c/SKILL.md", SKILL);
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(
        budget(&run),
        [over(60, 0, "AGENTS.md", 30).as_str()],
        "a missing CLAUDE.md counts 0, a tie goes to AGENTS.md"
    );
    put(root.path(), ".claude/skills/d/SKILL.md", SKILL);
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(
        budget(&run),
        [over(70, 0, "skill descriptions", 40).as_str()]
    );
}

// spec: SYN-38
#[test]
fn a_symbolic_link_counts_as_the_file_it_points_to_once() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    let root = root.path();
    std::fs::remove_file(root.join("AGENTS.md")).unwrap();
    put(root, "docs/agents.md", &"a".repeat(25));
    std::os::unix::fs::symlink("docs/agents.md", root.join("AGENTS.md")).unwrap();
    // the same file as CLAUDE.md, loaded once
    std::os::unix::fs::symlink("../CLAUDE.md", root.join(".claude/CLAUDE.md")).unwrap();
    let run = home.ok(root, &["check"]);
    assert_eq!(
        budget(&run),
        [over(65, 0, "CLAUDE.md", 30).as_str()],
        "{run:?}"
    );
}

// spec: SYN-38
#[test]
fn a_nested_fence_hides_its_imports_and_an_open_front_matter_scopes_nothing() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    let root = root.path();
    let claude = "````markdown\n```\n@docs/big.md\n```\n````\n~~~\n@docs/big.md\n~~~~\n";
    put(root, "CLAUDE.md", claude);
    put(root, "docs/big.md", &"b".repeat(1000));
    let rule = "---\nnot a front matter\n\npaths: see below\n";
    put(root, ".claude/rules/open.md", rule);
    // AGENTS.md is not imported; 10 of skill descriptions
    let memory = (claude.len() + rule.len()) as u64;
    let run = home.ok(root, &["check"]);
    assert_eq!(
        budget(&run),
        [over(memory + 10, 0, "CLAUDE.md", memory).as_str()],
        "{run:?}"
    );
}

// spec: SYN-38
#[test]
fn update_warns_once_judging_the_work_tree_it_leaves() {
    let home = Home::new();
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/AGENTS.md", &"a".repeat(20));
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "[cardinal]\nsource = \"file://{}\"\n\n[project]\nfixed_context_budget = 25\n",
            repo.path().display()
        ),
    );
    put(root.path(), "CLAUDE.md", "@AGENTS.md");
    let run = home.ok(root.path(), &["update", "--dry-run"]);
    assert!(
        budget(&run).is_empty(),
        "the dry run judges the work tree as it is: {run:?}"
    );
    let run = home.ok(root.path(), &["update"]);
    assert_eq!(budget(&run), [over(30, 25, "AGENTS.md", 20).as_str()]);
}

// spec: SYN-38
#[test]
fn every_memory_file_a_session_loads_at_launch_counts_and_nothing_else() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    let root = root.path();
    // 100 bytes: two imports, one in a code span and one in a fenced block that import nothing, an e-mail address
    let claude = "@docs/a.md see `@docs/b.md` x@docs/b.md\n```\n@docs/b.md\n```\n";
    put(
        root,
        "CLAUDE.md",
        &format!("{claude}{}", "c".repeat(100 - claude.len())),
    );
    put(root, "AGENTS.md", &"a".repeat(1000));
    put(root, "CLAUDE.local.md", &"l".repeat(1000));
    put(root, "docs/b.md", &"b".repeat(1000));
    // imported twice, counted once; its relative import resolves against its own directory
    put(root, "docs/a.md", "@c.md @c.md\n");
    put(root, "docs/c.md", &"c".repeat(8));
    put(root, ".claude/CLAUDE.md", &"d".repeat(40));
    put(
        root,
        ".claude/rules/x/always.md",
        "---\ndescription: x\n---\n",
    );
    put(
        root,
        ".claude/rules/scoped.md",
        &format!("---\npaths:\n  - \"src/**\"\n---\n{}", "s".repeat(1000)),
    );
    put(root, ".claude/rules/notes.txt", &"t".repeat(1000));
    // 100 + 12 + 8 + 40 + 23, and 10 of skill descriptions
    let run = home.ok(root, &["check"]);
    assert_eq!(
        budget(&run),
        [over(193, 0, "CLAUDE.md", 183).as_str()],
        "{run:?}"
    );
}

// spec: SYN-38
#[test]
fn agents_md_counts_in_place_of_a_missing_claude_md_and_imports_stop_at_four_hops() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    let root = root.path();
    std::fs::remove_file(root.join("CLAUDE.md")).unwrap();
    put(
        root,
        "AGENTS.md",
        "@1.md @/etc/hostname @~/x.md @../outside.md\n",
    );
    put(root, ".claude/AGENTS.md", &"a".repeat(6));
    for hop in 1..=4 {
        put(root, &format!("{hop}.md"), &format!("@{}.md\n", hop + 1));
    }
    put(root, "5.md", &"f".repeat(1000));
    // AGENTS.md 44 + 6, four hops of 6 bytes, 10 of skill descriptions
    let run = home.ok(root, &["check"]);
    assert_eq!(
        budget(&run),
        [over(84, 0, "AGENTS.md", 50).as_str()],
        "{run:?}"
    );
}

// spec: SYN-39
#[test]
fn a_budget_that_is_not_a_whole_number_is_warned_of_once_without_failing() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    for value in ["\"big\"", "-1", "1.5", "true"] {
        set_budget(root.path(), value);
        for args in [&["check"][..], &["sync"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(0), "{value} {args:?}: {run:?}");
            assert_eq!(budget(&run), [NOT_A_NUMBER], "{value} {args:?}: {run:?}");
        }
    }
}

/// The shared rule of the reports to the user, managed in `files/`.
const REPORTS_RULE: &str = ".claude/rules/reports-to-the-user.md";

/// The bytes of the rules a session loads at launch, all of them managed: none of this harness has `paths`.
fn rules_bytes(root: &Path) -> usize {
    std::fs::read_dir(root.join(".claude/rules"))
        .unwrap()
        .map(|e| usize::try_from(std::fs::metadata(e.unwrap().path()).unwrap().len()).unwrap())
        .sum()
}

#[test]
fn a_project_synced_from_this_harness_loads_the_shared_rule_of_the_reports_at_launch() {
    let home = Home::new();
    let root = tempdir().unwrap();
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    home.ok(root.path(), &["init", harness.to_str().unwrap()]);
    home.ok(root.path(), &["sync"]);
    // a project's CLAUDE.md that names AGENTS.md without importing it, larger than the skill descriptions
    let claude_md = format!("See `AGENTS.md`.\n{}", "c".repeat(20_000));
    put(root.path(), "CLAUDE.md", &claude_md);
    set_budget(root.path(), "0");
    let rule = get(root.path(), REPORTS_RULE).expect("the rule is synced");
    assert!(rule.contains("Reports to the user"), "{rule}");
    let run = home.ok(root.path(), &["check"]);
    let bytes = claude_md.len() + rules_bytes(root.path());
    let part = format!("the largest part is CLAUDE.md ({bytes} bytes)");
    assert!(
        budget(&run).iter().any(|w| w.ends_with(&part)),
        "the rule loads at launch with no import: {run:?}"
    );
}
