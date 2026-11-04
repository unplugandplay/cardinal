---
priority: low
depends_on: [reuse_a_worktree_across_issues]
why: 'a measure that drifts only for the sessions of a removed worktree named apart from its branches'
---
# Count the sessions of a closed worktree with its repository

# Goal
`session-metrics` gives each session the project of its first `cwd` (EVA-7). When that `cwd` is gone, a directory
`<repo>-<x>` next to an existing `<repo>` counts with `<repo>` only when `<x>` is a `gitBranch` of the session, the
naming of the former `start-issue`. Since `open-worktree` (ADR 0023) a worktree is named apart from its branch
(`--branch`) and serves several issues on other branches, so once it is closed its sessions count as a project of
their own (`repo-<x>`), and `evaluate-harness` splits one project in two. The outcome: the sessions of a removed
worktree `<repo>-<x>` count with `<repo>`, whatever the branches.

# Findings
* High review of `reuse_a_worktree_across_issues`, 2026-11-01: `repository()` of
  `crates/cardinal/src/tools/transcripts.rs` matches the suffix `-<branch>` against the session's branches only.
* Scenario: `just open-worktree feat --branch issue-a`, then issue `issue-b` there, then `close-worktree feat`: the
  sessions of `../repo-feat` count as project `repo-feat`.

# Approach
1. Link a missing `<repo>-<x>` to `<repo>` when `<repo>` is an existing git repository, whatever `<x>`; or
2. read the git directory of the main repository for a record of the removed worktree (none is left after
   `git worktree remove`: measure first), or
3. keep the branch match and add the worktree name given to `open-worktree` (no record of it either).

# Specs
* `specs/evaluation.md`: EVA-7 amended.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/transcripts.rs` (`repository`), `specs/evaluation.md` (EVA-7).
* `docs/adr/0023-worktrees-open-next-to-the-main-one-with-no-issue.md`.

# Hints
* A directory `<repo>-<x>` next to `<repo>` may be an unrelated clone: approach 1 then misattributes it.

# Tracking

Plan: validated

## Understanding
A removed `<repo>-<x>` next to an existing git repository `<repo>` counts with `<repo>` whatever `<x>` (approach 1);
the branch match goes, and with it the `branches` argument of `repository()`. Approach 2 measured: `git worktree
remove` leaves no record in the main git directory (no `worktrees/` entry, no path in `.git`), so it is out.

## Plan
1. [x] Link a removed `<repo>-<x>` to `<repo>` whatever `<x>`: `repository()` tries the prefixes of the name, the
   longest first, drops its branch argument (5 callers, restored by deliverable 2); verified by a `session_metrics.rs` session in
   `myrepo-feat` on branch `issue-b` counted as `myrepo`; makes true EVA-7 (amended).

2. [x] Fixes of the `high` review of the issue: the branch match comes first again, a prefix must be the top level
   of a work tree, a missing ancestor is matched too, the empty `cwd` guard moves into `repository()`; verified by
   three more sessions of `session_metrics.rs`; makes true EVA-7 (amended again).

## Decisions
- Mode: autonomous
- Base commit: 0162c22a03e49f8430270648133677068484a6e6
- 1 a: an unrelated clone `<repo>-<x>` removed since is accepted as a misattribution, stated in EVA-7.
- 2 a: the longest existing prefix wins.

## Deliverable 1

- done: a removed `<repo>-<x>` counts with the longest existing git repository among the prefixes of its name;
  `repository()` no longer takes the session's branches, so its per-`cwd` cache in the four readers is now exact.
- red: EVA-7: `FAIL - a removed worktree named apart from its branches counts with its repository`
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2

- done: review fixes of `0162c22..d2ca9df` (high): branch suffix first then the longest prefix, a prefix counts only
  as the top level of a work tree, missing ancestors are matched, an empty prefix and an empty `cwd` give nothing.
- red: none (the three new checks guard the review's scenarios: branch with a dash, plain directory in a repository,
  session started in a subdirectory)
- Code review: 1 findings: a missing prefix passed the top-level check (None == None), fixed by requiring both sides.
- High review, not fixed: an unrelated removed clone `<repo>-<x>` counts with `<repo>` in delivered-quality too
  (decision 1 a); `rule-breaches` resolves the repository per session without the per-`cwd` cache of the others
  (unchanged by this issue, a few `git` calls per session).
- not verified: none
- decisions needed: none
