---
priority: low
why: 'only an issue checked in `issues/done/` is affected, and `check-issue` runs mostly on open issues'
---
# Find the project root of an issue checked in issues/done

# Goal
`check-issue` takes the project root as the grandparent of the issue file. For `issues/done/foo.md` that is
`issues/`, so `features::is_off` finds no configuration and an attribution line is refused even when the project
turned off `no-ai-attribution` (FEA-9). The outcome: the root found wherever the issue file lies.

# Findings
* `crates/cardinal/src/tools/check_issue.rs`, the `root` computed with `path.parent().and_then(Path::parent)`.
* Found by the `low` review of `turn_off_conventional_commits`.

# Approach
1. Walk up from the issue file to the directory holding `.claude/cardinal.toml`.
2. Or use `repo_root()`, as `commit-check` does.

# Specs
* `specs/features.md` FEA-9.

# Milestone
Optional features

# Resources
* `crates/cardinal/src/tools/check_issue.rs`, `crates/cardinal/src/features.rs` (`is_off`).

# Tracking

Plan: validated

## Understanding
`check-issue` must find the project root (the nearest ancestor of the issue file holding `.claude/cardinal.toml`)
wherever the file lies, so FEA-9 holds for `issues/done/foo.md`. Out of scope: the other checks of the file name.
Means: approach 1 (walk up), no git call, so it works outside a repository like the tests' tempdirs.

## Plan
1. [x] Walk up to the root: `check-issue` takes the nearest ancestor holding `.claude/cardinal.toml` (the
   grandparent when none does); verified by a test running it on `issues/done/b.md` with the feature off; makes true FEA-9.
2. [x] Fixes of the `high` review: reuse `run::project_root` (canonical path, `CONFIG`, no dead fallback), a
   test from `issues/done/` itself and with the ban on; makes true FEA-9.

## Summary of deliverable 1
- done: `check-issue` takes as root the nearest ancestor of the file holding `.claude/cardinal.toml`.
- red: FEA-9: `issues/done/b.md: line 8: attribution line`
- Code review: no findings
- not verified: none
- decisions needed: none

## Summary of deliverable 2
- done: the `high` review findings fixed on the issue's change.
- red: FEA-9: `b.md` run from `issues/done/` was flagged (the old walk stopped at the cwd)
- Code review: no findings
- not fixed: no repository boundary on the walk up, the same as `run::project_root`, which resolves the project of
  every `cardinal run`; the `depends_on` and name checks of a done issue go to
  `issues/done/check_a_done_issue_against_its_issues_directory.md`.
- not verified: none
- decisions needed: none

## Decisions
- Mode: autonomous
- Base commit: 016c2f26d19227d74abb06030835db19bbedab2c
- Root: walk up to `.claude/cardinal.toml` (approach 1), the grandparent as fallback.
