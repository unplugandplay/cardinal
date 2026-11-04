# Handle squash and rebase merged issue branches in finish-issue and next-issue

# Goal
An issue branch merged by squash or rebase must be cleaned by `just finish-issue`, and a stale branch must not keep an
issue out of `just next-issue` without the user seeing why.

# Findings
* `issue_worktree.rs` `finish` requires `git merge-base --is-ancestor <stem> <trunk>`, which is false after a squash or
  rebase merge. It errors "not merged" and removes nothing.
* `next_issue.rs` marks an issue as taken when a local branch has the issue stem as name. A leftover branch (squash
  merge, `git worktree remove` without `branch -d`) keeps the issue taken, and the output can be "every issue of the
  queue is taken" with only the Taken lines to explain it. A branch named like a trunk (`main`) would match a stem too.

# Approach
1. Add failing tests for both: a squash-merged branch, a leftover branch with no worktree.
2. Candidates: detect a merge by patch equivalence (`git cherry`) or by the merged pull request, or accept a
   `--force` flag after a prompt; count a branch as taking the issue only when it has a worktree or commits not on trunk.

# Specs
* `specs/parallel-issues.md`: amend the requirements for finish and for taken issues.

# Resources
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/skills/issue-workflow/next_issue.rs`, `specs/parallel-issues.md`

# Hints
* `finish` must never delete work that is not on trunk.

# Tracking

1. [x] `finish-issue` accepts a rebased or one-commit squashed branch (patch equivalence) and `--force` for the rest (PAR-6, PAR-10)
   Done: `finish` merged test = ancestor or `git cherry` without `+`; `--force` (recipe `finish-issue stem *flags`) accepts the rest, still
   refuses a dirty worktree; `branch -D` when not an ancestor. Tests `finish_accepts_a_patch_equivalent_branch`,
   `finish_forces_a_squash_of_several_commits`. SKILL.md updated. Verified: `just check`, `just test-harness`, `just check-harness`.
   Not verified: a branch with no commit at all (empty `git cherry`) counts as merged, by design. Code review: 8 findings. Fixed: `--force` accepted before the stem too; this line. Not fixed, by design: spec ahead of code
   (PAR-3/PAR-4 land with deliverable 2, the old PAR-3 scenario is replaced there; spec stays `draft` until then); patch-equivalence
   is the approach chosen with the user (`-D` after `git cherry` can miss a reverted cherry-pick, `--force` stays the explicit escape);
   the checked-then-removed race is negligible for a single-user flow; the multi-commit test already covers the `+` refusal; unquoted
   `*flags` is the usual just idiom.
2. [x] `next-issue` takes an issue only for a worktree or commits off trunk, and reports stale branches (PAR-3, PAR-4)
   Done: `next_issue.rs` takes an issue for a worktree or `git cherry` `+` commits (true when git cannot tell), prints `Stale:` lines; existing
   tests now give taken branches a commit; new `stale_checks`. Spec set to `implemented`, old PAR-3 scenario reworded, architecture.md updated.
   Verified: `just check`, `just test-harness`, `just check-harness`. Not verified: a multi-commit squash left as a branch is still shown
   `Taken:` (its commits are not patch-equivalent), the user sees it and `--force`s. Code review: 8 findings. Fixed: `git cherry` on
   `refs/heads/<name>` (no ambiguity), git queried only for branches named after a queue entry, architecture.md wrapped. Not fixed, by design:
   an empty branch with no worktree is free (`start-issue` always makes a worktree, agreed with the user); trunk is the branch of the main
   worktree, as in `finish`; multi-commit squash stays `Taken:` and needs `--force` (documented limit); `Stale:` only covers queue entries.

## Decisions

* Mode: autonomous.
* No pull request detection (no network); `--force` is itself the confirmation, the skill asks the user before the agent uses it.
* The spec `specs/parallel-issues.md` stays `draft` until the last deliverable.
