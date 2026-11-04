---
priority: low
why: 'a comfort: these files cost few tokens (skill descriptions) and no hook call'
---
# Declare the features made of managed files only

# Goal
Some managed files serve only part of the users: the skills `evaluate-harness` and `audit-instructions` (measuring
the harness), the rule `.claude/rules/reports-to-the-user.md` (a report style), and the GitHub CI
(`.github/workflows/cardinal.yml`, `.github/install_cardinal.sh`) for a project not on GitHub. The outcome: features
a project turns off, the fixed context shrinking by the descriptions of the skills it drops.

# Findings
* The mechanism copies or leaves out whole files (FEA-3): no code in `sync`.
* The retention warning (SYN-28) only matters to the measure of the harness: it belongs to the same feature.

# Approach
1. Declare `harness-measure` (the two skills), `report-style` (the rule) and `github-ci` (the workflow and its
   installer) in `features.toml`.
2. Gate the SYN-28 warning on `harness-measure` (an FEA-n, as FEA-10 does for the attribution warning).
3. Check that no other managed file cites a file of these features as required (`issue-workflow` names
   `skill:evaluate-harness` in `Expected-effect:` guidance): a citation of a skill turned off must stay harmless.

# Specs
* `specs/features.md` (the SYN-28 gate), `specs/sync.md` SYN-28 untouched.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* `files/.claude/skills/evaluate-harness/`, `files/.claude/skills/audit-instructions/`, `files/.github/`.

# Hints
* The fixed context budget (SYN-38) shows the gain: compare `cardinal check` before and after on a consumer.

# Tracking

Plan: validated

## Understanding
Declare `harness-measure`, `report-style` and `github-ci` as file-only features (FEA-27) and quiet the SYN-28 warning
with `harness-measure` off (FEA-28). The rule holds the language rule too: it moves to `.claude/rules/user-language.md`,
in no feature (FEA-29), so turning off the report style keeps the language. No rewording of the citations (non-goal).

## Plan
1. [x] Split the rule: `user-language.md` takes the language paragraph, `reports-to-the-user.md` keeps the format,
   `AGENTS.md` names both; verified by a test of `skills.rs`; makes true FEA-29.
2. [x] Declare the three features in `features.toml` and gate the retention warning on `harness-measure`; verified by a
   test of `skills.rs` (the declared files) and one of `retention_cli.rs` (no warning with the feature off); makes true
   FEA-27, FEA-28.

## Decisions
- Mode: autonomous
- Base commit: c5e4fc54f321bddb2fb5dd3566260139c3a7a873
- Q1: a, the language rule moves to `.claude/rules/user-language.md`, in no feature.

## Summary of deliverable 1
- done: the language rule lives in `.claude/rules/user-language.md`, `reports-to-the-user.md` keeps the format only,
  `AGENTS.md` names both.
- red: FEA-29: `the_user_language_is_said_once_in_the_always_on_rule` panicked, `files/.claude/rules/user-language.md`
  not found.
- Code review: no findings

## Summary of deliverable 2
- done: `harness-measure`, `report-style` and `github-ci` declared in `features.toml`; the retention warning is quiet
  with `harness-measure` off; `docs/usage.md` lists the three features.
- red: FEA-27, FEA-28: `features.toml declares no harness-measure`; the retention warning printed after `sync` with
  the feature off.
- Code review: no findings
- not verified: the fixed context gained on a consumer (the hint of the issue).

## Summary of the high review (deliverable 3)
- done: with `harness-measure` off only the warnings on the user settings go, those on the project's settings stay
  (FEA-28 narrowed: they sweep every project's transcripts); `AGENTS.md` names the report rule as optional; README
  lists the three features; the test helper fails on an unreadable entry.
- red: FEA-28: the project's `cleanupPeriodDays: 7` not warned of with the feature off.
- Code review (high): 7 findings: 4 fixed (above); not fixed: FEA-28 reads `disabled` from the configuration as
  FEA-10 does (the project's intent, a sync on an older source fails anyway); `update` untested (a local source
  fails it, the same `main.rs` path prints for the three commands); the substring check of FEA-29 is enough for a
  path the project writes as is.
- Code review: no findings
