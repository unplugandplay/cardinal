---
priority: low
depends_on: [turn_off_the_issue_files]
why: 'only a project with its own `issues/` code meets it, and only once it turns `issue-files` off'
---
# Leave the issues directory alone with the issue files off

# Goal
The binary reads `issues/` as the issue files even when the project turns off `issue-files`. A project tracking its
issues in another tool may hold its own code under `issues/`, which the gates then treat as issue files. The outcome:
with `issue-files` off, `issues/` is a directory like any other for the review gate, `deliverable-size` and
`rule-breaches`.

# Findings
* `review-gate` lets a commit through with no review when its diff touches `issues/` only (`ISSUES_DIR` of
  `crates/cardinal/src/tools/review_gate.rs`), and the close of an issue that `close-issue` rewrote.
* `deliverable-size` leaves `issues/` out of the count (`crates/cardinal/src/tools/deliverable_size.rs`, SIZ-4).
* `rule-breaches` counts `issues/` among the plan directories of rule `validation-stop` (`PLAN_DIRS`, BRE-8), and
  its trigger is a `Plan: proposed` written under `issues/`: a project's own workflow skill writing that marker there
  gets breaches of a workflow it turned off.
* Found by the `high` review of `issues/done/turn_off_the_issue_files.md`.

# Approach
1. Gate each of the three readers on `features::is_off(root, "issue-files")` (`rule-breaches`: on the configuration
   as of the session's start, as for `review-gate`, FEA-26).
2. Or declare the non-goal: a project with `issue-files` off names no directory `issues/`.

# Specs
* `specs/features.md` (new requirements), `specs/review-gate.md`, `specs/deliverable-size.md`,
  `specs/rule-breaches.md` if they change.

# Milestone
Optional features

# Resources
* `issues/done/turn_off_the_issue_files.md`, `specs/features.md` (FEA-26 for the session-start reading),
  `crates/cardinal/src/features.rs`.

# Tracking

Plan: validated

## Understanding
With `issue-files` off, `issues/` loses its exemptions: `review-gate` reviews it, `deliverable-size` counts it, rule
`validation-stop` of `rule-breaches` applies to no session (approach 1 of the issue). Out of scope: the issue tools
themselves, and rule `skill-routing-issue-workflow`, already gated on `skill:issue-workflow` being synced (BRE-17).

## Plan
1. [x] review-gate: drop the `issues/`-only and close exemptions when `features::is_off(root, "issue-files")`; verified
   by tests of `native` on a project with the feature off; makes true FEA-36.
2. [x] deliverable-size: count `issues/` when the feature is off; verified by a test through the binary; makes true FEA-37.
3. [x] rule-breaches: no `validation-stop` for a session whose config at `start` turns the feature off; verified by a
   transcript test; makes true FEA-38, spec `accepted`.

## Decisions
- Mode: autonomous
- Base commit: 75ae20c657cfd2754dfcecc55ae07e2629690aad

## Deliverable 1
done: `review-gate` drops its `issues/`-only and close exemptions when `issue-files` is off.
red: FEA-36: assertion `left == right` failed: a diff under issues/ only waits for a review (left: 0, right: 2)
Code review: no findings

## Deliverable 2
done: `deliverable-size` counts `issues/` when `issue-files` is off.
red: FEA-37: assertion `left == right` failed: left "1\tsrc/a.rs\n1 changed lines…", right "1\tissues/a.md\n1\tsrc/a.rs\n2 changed lines…"
Code review: no findings

## Deliverable 3
done: rule `validation-stop` of `rule-breaches` applies to no session whose config at `start` turns `issue-files`
off; spec `features` accepted.
red: FEA-38: FAIL - a session of a project that turned off issue-files is not checked against validation-stop
(`validation-stop	breach	s.jsonl:4	src/main.rs`)
Code review: no findings

## Deliverable 4: docs and `high` review of the issue
done: `docs/gates.md` notes FEA-36; REV-9, REV-20 and SIZ-4 point to their exception; tests and names tightened.
red: none (fixes of the review on green tests)
`high` review: 9 findings
- Fixed: REV-9 and REV-20 silent on FEA-36; SIZ-4 silent on FEA-37; the FEA-38 test had no positive control
  (now `oversized-result` applies); the FEA-36 test never recorded a review (now a reviewed diff, then a new
  `issues/x.md` refused as never seen); `staged` ran with the issue files off; the boolean `issues` renamed
  `issue_files`.
- Not fixed: the gate reads the work tree's configuration, so an unstaged edit of it restores the exemptions: the
  gate catches mistakes, not deliberate bypasses (module doc), and every other read of the hook is the work tree's.
- Not fixed: `deliverable-size <a>..<b>` reads the current configuration, as it already does for its limits.
- Not fixed: the configuration parsed twice by `deliverable-size` (`limit` and `is_off`): one small file, a shared
  reader kept.
Code review: no findings
