# Judge the repository that really commits in the review hook

# Goal
When the command is `cd <dir> && git commit`, `require_code_review.rs` must compute the snapshot and read the marker
of the repository that commits, not of the cwd of the event.

# Findings (from a code review of the synced revision, to reproduce first)
* `commit_dirs` follows `git -C <dir>` but not a preceding `cd <dir>`. In the parallel-worktree flow the agent
  can run `cd ../<project>-<stem> && git commit` from the main worktree: the hook judges the main worktree.
* A review done in the main worktree can then wave through an unreviewed commit in the other worktree, and an
  unrelated unreviewed main worktree can block a reviewed commit. Only the native pre-commit gate, which may not be
  installed, catches it.

# Approach
1. Add failing tests with `cd` before `git commit`, relative and absolute, with `&&`, `;` and in a subshell.
2. Track the directory changes of the segments before the commit, as `-C` is tracked; fall back to the event cwd.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dirs`), `files/.claude/hooks/test_require_code_review.rs`,
  `specs/parallel-issues.md`

# Hints
* An unparsable `cd` (variable, command substitution) should refuse or fall back safely, not pass.
