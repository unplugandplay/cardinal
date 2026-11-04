# Keep the review waiver until the commit really happens

# Goal
The user waives the review of the next commit by creating the waiver file in the git directory. The waiver must be
consumed by the commit it was meant for, not by a command that only looks like a commit.

# Findings
* `refusal` removes the waiver as soon as it sees one, in the `PreToolUse` check. If the command then does not commit
  (`git commit --dry-run`, a commit refused by `block_ai_attribution`, by another hook or by git itself), the waiver is
  gone and the next real commit is refused although the user waived it.

# Approach
1. Add a failing test: waiver present, the check lets a command through, a second check still finds the waiver
   when no commit happened.
2. Candidates: remove the waiver in a `PostToolUse` step once `HEAD` moved; or record `HEAD` at waiver use and drop
   the waiver only when it changed; or ignore `--dry-run`.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`refusal`, `main`), `files/.claude/hooks/test_require_code_review.rs`

# Hints
* Hooks of one event run in parallel or in order depending on the client: do not rely on the order of the two
  `PreToolUse` Bash hooks.
* The hook refuses any Bash command that names the waiver file: write the issue and the tests without that literal in a command.

# Tracking

1. [x] The waiver survives until a commit really happens: the check stamps the waiver with `HEAD` when it lets a commit
   through and drops it, without using it, once `HEAD` moved; tests first, then the hook, then its docs
   (hook header, `docs/architecture.md`, `just sync`; the `issue-workflow` skill does not describe the consumption).

## Decisions

* Mode: autonomous (local commit after the review, no push).
* No spec: the issue changes a shared hook, no capability of `specs/`.
* Approach: lazy consumption keyed on `HEAD` (no `PostToolUse` step, no reliance on hook order). The waiver may stay
  on disk until the first check after the commit; that has no effect on commits.

## Deliverable 1: summary

* Done: `refusal` stamps the waiver with `HEAD` (`no-head` without commit) when it lets a commit through, and drops it
  without using it once `HEAD` moved. Tests (`files/.claude/hooks/test_require_code_review.rs`): the waiver stays across
  checks while no commit happened, and is consumed once a commit moved `HEAD` (main and linked worktrees). Hook
  header and `docs/architecture.md` updated; `just sync` done.
* Verified: `just check`, `just test-harness`, `just check-harness` green.
* Not verified: a dry-run commit or a commit refused by another hook in a live session.
* Code review: 7 findings. Fixed: a stale stamp is dropped at any check, before the early return of an empty or
  issues-only diff, so a waiver created again after a commit is not mistaken for the spent one (the user creates it
  again after each commit); a stamp that cannot be written refuses; tests for both and for the `no-head` stamp; long
  lines rewrapped; the tracking no longer claims a change of the skill. Accepted: while no commit happened the waiver
  covers any retry, even with other files (the point of the issue: the user waived the next commit).
