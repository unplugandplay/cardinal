# Harden finish-issue and next-issue around merged and current worktrees

# Goal
`just finish-issue` must never remove the directory the user stands in, and `just next-issue` must free an issue
whose branch was squash-merged, whatever the number of commits.

# Findings (from a code review of the synced revision, to reproduce first)
* `finish` is documented as runnable from any worktree. Run inside the worktree of the issue, it removes the
  directory under the shell and later relative paths and `git` calls of that session fail.
* `next_issue.rs` `has_own_commits` is true as soon as `git cherry` shows a `+` line. A branch squash-merged from
  several commits keeps `+` lines, so the issue stays "taken" until `finish-issue --force`.

# Approach
1. Add failing tests for the two cases.
2. Candidates: refuse (or `cd` out first) when the cwd is inside the worktree to remove; detect a squash merge by
   comparing the tree diff of the branch against trunk.

# Specs
* `specs/parallel-issues.md`: amend the requirements for finish and taken issues.

# Resources
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/skills/issue-workflow/next_issue.rs`,
  `files/.claude/skills/issue-workflow/close_issue.rs`, `specs/parallel-issues.md`

# Hints
* `finish` must never delete work that is not on trunk.

# Tracking
1. [x] `finish-issue` refuses, with or without `--force`, from inside the worktree to remove (PAR-11).
2. [x] A squash of any number of commits counts as merged for `finish-issue` and frees the issue in `next-issue`, by comparing trees (PAR-3, PAR-6, PAR-10); docs of the skill updated.

## Decisions
* Mode: autonomous.
* Refuse (do not `cd`) when the cwd is the worktree or inside it; `--force` refuses too; the message names the main worktree.
* Squash detection: `git merge-tree --write-tree trunk branch` gives trunk's tree (needs git >= 2.38); a git error counts as not merged.

## Deliverable 1 summary
* Done: `finish` refuses from inside the worktree to remove (canonical cwd under the target), `--force` or not, naming the main worktree; test `finish_refuses_from_inside_the_worktree` (`// spec: PAR-11`) failed first (the worktree was removed under the shell); docs of the skill, architecture and script header updated; spec set `accepted`.
* Verified: `just check`, `just test-harness`, `just check-harness`.
* Not verified: a cwd whose symlinks `canonicalize` cannot resolve.
* Code review: 6 findings. Fixed: the guard now runs before the merged test (an unmerged branch gets the same refusal, tested); the architecture sentence reworded; the spec goes back to `draft` (it was set `accepted` while PAR-3/6/10 are still to be implemented: deliverable 2 sets it). Not fixed: the guard compares only with the sibling path (a branch checked out elsewhere is already refused by the existing `found != dir` check); the test helper's early return for a missing cwd is kept, the `kept` check catches a removal.

## Deliverable 2 summary
* Done: `finish` and `next_issue.rs` treat a branch as merged when `git merge-tree --write-tree trunk branch` yields trunk's tree (any git error counts as not merged); tests for a squash of several commits (finish accepts it, next-issue reports it `Stale:`), a squashed branch whose file trunk rewrote afterwards (kept, taken), and `--force` for work not on trunk (all failed first for the squash); skill doc and architecture updated; spec set `accepted` then `implemented`.
* Verified: `just check`, `just test-harness`, `just check-harness`.
* Not verified: git older than 2.38 (no `--write-tree`: counts as not merged, the safe side). ADR 0003 left as is (its "not a squash" wording was already historical).
* Code review: 5 findings. Fixed: the PAR-10 test starts its own worktree; stale comment; "keep in sync" notes (the scripts are standalone, so the helper stays duplicated); the refusal message names the git 2.38 requirement. Not fixed: `merge-tree --write-tree` writes loose tree objects on each `next-issue` run (only for branches not patch-equivalent; `gc` collects them, a cheaper check would not cover a squash).
