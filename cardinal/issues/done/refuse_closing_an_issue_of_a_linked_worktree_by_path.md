# Refuse closing an issue of a linked worktree by path

# Goal
`close_issue.rs` checks for a linked worktree (PAR-8) in the caller's directory, not in the repository that holds
the issue. From the main worktree, `just close-issue ../repo-foo/issues/foo.md` passes the guard and edits the queue
and the references of the linked worktree, which PAR-8 forbids. The guard should check the worktree of the issue's
directory.

# Findings
* 2026-10-26, `/code-review high` of `move_tracked_issues_with_git_mv`: `linked_worktree()` calls
  `repo_root(Path::new("."))` and `git worktree list` in the current directory, before the issue path is read.

# Approach
1. Make `linked_worktree` take the directory to check: the issue's `issues/` directory for a close, `.` for
   `--relink`; validate the argument before the guard.
2. A case in `refuses_a_linked_worktree` of `test_close_issue.rs`: closing the linked worktree's issue by path from
   the main worktree is refused and leaves its queue untouched.

# Specs
* `specs/parallel-issues.md`, PAR-8 (its scenario may gain the by-path case).

# Resources
* `files/.claude/skills/issue-workflow/close_issue.rs` (`linked_worktree`, `main`)
* `files/.claude/skills/issue-workflow/test_close_issue.rs` (`refuses_a_linked_worktree`)

# Hints
* Change `files/`, then `just sync`; `just test-harness` runs the script tests.

# Tracking

Plan: validated

## Understanding
The PAR-8 guard of `close_issue.rs` must check the worktree holding the issue's `issues/` directory, not the caller's
directory; `--relink` keeps checking `.`. Out of scope: any other change to what a close edits.

## Plan
1. [x] Guard on the issue's worktree: `linked_worktree(dir)` runs `git` in the given directory, `main` validates the
   issue path before the guard; verified by a by-path case in `refuses_a_linked_worktree` of `test_close_issue.rs`;
   makes true PAR-8 (amended).
2. [x] Fixes of the `high` review: also refuse when the caller is in a linked worktree (closing the main worktree's
   issue by path from a linked one passed after deliverable 1), clearer refusal message; verified by a reverse case in
   `refuses_a_linked_worktree`; makes true PAR-8 (amended again).

## Decisions
- Mode: autonomous
- Base commit: 6b2825e320a9d07ff2af9157f0ebc284c221ac98

## Deliverable 1
- done: the PAR-8 guard runs `git` in the issue's `issues/` directory (`.` for `--relink`), after the path is validated.
- checks: `just check` ok, `just test-harness` ok, specs ok (PAR-8 amended), docs ok (architecture, details.md).
- red: `FAIL - the linked worktree's issue is refused by path from the main one`
- Code review: no findings
- not verified: none.

## Deliverable 2
- done: the guard checks both the caller's directory and the issue's directory; the message says to merge, then close
  in the main worktree.
- checks: `just check` ok, `just test-harness` ok, specs ok (PAR-8 and its scenarios), docs ok (architecture, details.md).
- red: `FAIL - the main worktree's issue is refused by path from a linked one`
- High review: 6 findings. Fixed: regression, missing reverse test, spec wording, message. Not fixed: the guard lets
  the close through when `git worktree list` or a canonicalize fails (predates this issue, rare); the extra
  `git rev-parse` per close (one process, keeps the guard independent of the relink).
- Code review: no findings
- not verified: none.
