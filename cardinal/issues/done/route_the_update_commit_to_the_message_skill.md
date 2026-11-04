---
priority: low
depends_on: [find_why_commits_skip_the_commit_message_skill]
why: 'a few skips in consumer projects; the close line of the parent issue is measured first'
---
# Route the update commit to the message skill

# Goal
After `cardinal update`, the agent commits the new revision with a message written inline, without
`skill:commit-message`: nothing in the update output or the instructions points it there. The outcome: the update
names the next step, committing the change with `skill:commit-message`, and the harness update commits stop being
`skill-routing-commit-message` breaches.

# Findings
* Measured 2026-11-03, after BRE-16 left out the throwaway and checked commits: 9 harness update commits
  (`chore(cardinal): update Cardinal to …`, `chore(harness): update the shared harness to …`) in 5 sessions of one
  consumer project skipped the skill, the last on 2026-10-30; none ran the message checker.
* `cardinal update` reports the previous and new revisions and the files changed (SYN-16), no next step.

# Approach
1. Print, after the report of SYN-16, one line naming the next step: commit the update with
   `skill:commit-message` (not under `--dry-run`, nor when nothing changed).
2. Re-measure the harness update commits after a few updates.

# Specs
* `specs/sync.md`: SYN-16 amended, or a new requirement.

# Milestone
Lean agent loop

# Resources
* `specs/sync.md`, `crates/cardinal/src/main.rs`.
* `issues/done/find_why_commits_skip_the_commit_message_skill.md`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
`cardinal update` ends with a `next:` line pointing to `skill:commit-message` when it wrote a change and the project has
that skill; nothing under `--dry-run`, on conflicts, or when nothing changed. The re-measure (step 2 of the Approach)
is left for after a few updates, out of this plan.

## Plan
1. [x] Next-step line of `update`: print it in `main.rs` after the report, tests in `tests/update_cli.rs` updated;
   verified by `update_cli` tests; makes true SYN-61. (autonomous)

2. [x] Fixes of the `high` review: `UpdateReport.changed` set where `update` writes (a skipped gate, a conflict or a
   kept file is no change; a renamed `rev` comment is one), test with a git project under `core.hooksPath`, docs
   rewrapped; verified by `update_cli`; makes true SYN-61. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: a5434425a2ee8851c37430d83c8b78c932e1ebec
- 1 a: the line only when the project has the skill; 2 a: on stdout, with the report.

## Deliverable 1 summary
- done: `update` ends on `next: commit the update with skill:commit-message` once it wrote a change and the project has the skill.
- checks: see the table of the final report (tests, lint, specs, docs, code review).
- red: SYN-61: `left: Some("updated: a.txt")`, `right: Some("next: commit the update with skill:commit-message")`.
- Code review: no findings
- not verified: the effect on the skips of `skill-routing-commit-message`, measured after a few updates.
- decisions needed: none.

## Deliverable 2 summary
- done: the `high` review findings fixed; the change is judged by the library where it writes.
- red: SYN-61: `next: commit the update with skill:commit-message` printed after `skipped (…): native review gate` at the same revision.
- Code review: no findings
- `high` review, not fixed: the binary names the path of the skill (`.claude/skills/commit-message/SKILL.md`); a move
  of the skill is a change of the managed content's layout, which ADR 0014 and 0017 already handle (format raised,
  migration), and the line only goes missing, nothing breaks.
- not verified: the effect on the skips of `skill-routing-commit-message`, measured after a few updates.
- decisions needed: none.
