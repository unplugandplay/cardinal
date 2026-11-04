---
priority: medium
depends_on: [turn_off_the_review_gate]
why: 'a project that turns the review gate off is still counted as breaking its rules, which skews the measure of the harness'
---
# Quiet the review rules of rule-breaches when the gate is off

# Goal
`rule-breaches` applies the rules of the review gate (`no-verify`, `stage-everything`, `review-waiver`,
`review-before-commit`) to every commit, even when the project turned the feature `review-gate` off. With the
feature off, staging every file or skipping the git hooks is legitimate, but it is still counted as a breach in
`cardinal-report` and `skill:evaluate-harness`. Expected: the rules carried by the feature are not applied to a commit
whose `.claude/cardinal.toml`, as of that commit, turns it off, as FEA-11 and FEA-22 do for the other features.

# Findings
* `check_bash` in `crates/cardinal/src/tools/rule_breaches.rs` applies the four rules with no look at `disabled`
  (found by the `high` review of `turn_off_the_review_gate`).
* The rules are not stated in `AGENTS.md`: unlike `user-language` (`language-hint`), no rule of the user stays loaded.

# Approach
1. Read how FEA-11 and FEA-22 judge the configuration as of a commit, and reuse it for `review-gate`.
2. Specify the requirement in `specs/features.md`, with the rules of BRE-9 it silences.

# Specs
* `specs/features.md` (a new FEA requirement), `specs/rule-breaches.md` if BRE-9 names the exception.

# Milestone
Optional features

# Resources
* `crates/cardinal/src/tools/rule_breaches.rs`, `specs/features.md` FEA-11 and FEA-22, `features.toml`.

# Tracking

Plan: validated

## Understanding
`rule-breaches` drops the four rules of the review gate for a session whose project turned `review-gate` off. These
rules read the session's calls, not its commits: the configuration judged is the one committed at the session's
`start` (as BRE-14 reads it), not the one "as of that commit" of FEA-11 and FEA-22. The refusals of the hook are out of scope.

## Plan
1. [x] Quiet the four rules: `Context` carries whether the configuration at `start` turns `review-gate` off, and the
   rules of `check_bash`, `check_edit` and the refusals read from the errors are skipped then; `features.toml` and the
   docs name it; verified by a test of `tests/rule_breaches.rs` (a session with `git add -A` and a commit skipping the
   hooks, its project turning the gate off); makes true FEA-26.

## Decisions
- Mode: autonomous
- Base commit: 806fb57eacbca9f6d86d0ee8faf5f67d47792a2e
- Configuration judged: the one committed at the session's `start` (1 a).

## Summary of deliverable 1
- done: `rule-breaches` skips the four rules of the review gate for a session whose configuration at `start` turns
  `review-gate` off; `features.toml` and `docs/gates.md` say it.
- red: FEA-26: `FAIL - a session of a project that turned off review-gate is not checked against its rules` (the
  session was reported `stage-everything`).
- Code review: 2 findings, none fixed: both feared that the skip of `check_bash` and of the `Refused:` errors silenced
  other rules; `check_bash` applies only the four rules and the error branch adds only `review-waiver` and
  `review-before-commit`.

## Summary of the close (update-docs and high review)
- done: `docs/usage.md` names the four rules in the row of `review-gate`; `docs/gates.md` and it say which sessions
  go quiet; the FEA-26 test checks that the session is read (`oversized-result` applies).
- red: none (a fix of the test and the docs).
- High review: 8 findings. Fixed: the test with no positive control, the two doc wordings. Not fixed: the
  configuration read at the session's `start` on the default branch (a worktree branch, a change mid-session) is
  decision 1 a, as BRE-14; a refusal of a gate a project turned off yet kept is a configuration it chose; the three
  call sites of the exception (a rule-to-feature map in `Verdict`) would be a refactor of FEA-11 and FEA-22 too,
  left out of this issue.
- Code review (low): no findings.
