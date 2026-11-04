---
priority: low
depends_on: [split_the_managed_recipes_by_feature, let_a_feature_need_another]
why: 'a team tracking its issues elsewhere can still use the rest; the feature is the largest to cut out'
---
# Turn off the issue files

# Goal
The issues live in `issues/`, driven by `issue-workflow`, `create-issue`, `prioritize-issues`, the recipes
`next-issue`, `close-issue`, `check-issue` and the roadmap. A team tracking its issues in another tool cannot drop
them. The outcome: a feature `issue-files` that leaves out the skills, the recipes and the seeded roadmap guidance.

# Findings
* `issue-workflow` is the main loop of the harness: without it a project keeps the gates, the specs and the
  commit checks.
* `next-issue` and `issue-claim` read `issues/` from the binary.

# Approach
1. List every managed file and check that names `issues/`, and what each becomes without it.
2. Declare the feature, with its `needs`, and gate the binary's issue tools if they are called by other features.
3. Decide whether a lighter workflow skill (an issue given in the prompt) replaces `issue-workflow`, or whether it
   is left to the project.

# Specs
* `specs/features.md`, `specs/next-issue.md`, `specs/issue-claim.md`, `specs/close-issue.md` if they change.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* `files/.claude/skills/issue-workflow/`, `docs/workflow.md`, ADR 0021.

# Hints
* The seeded `ROADMAP.md` is the project's (SYN-4): the feature cannot remove it.

# Tracking

Plan: validated

## Understanding
A feature `issue-files` leaves out the three issue skills and `cardinal-issues.just`; the rest of the harness keeps
working without `issues/`. Out of scope: gating the binary's issue tools (nothing left on calls them), a replacement
workflow skill, the seeded `ROADMAP.md` (the project's), rewording the skills that cite the issue skills.

## Plan
1. [x] Declare `issue-files` in `features.toml` (files only, no `needs`), its row in the test of the file-only
   features, the README and the docs; verified by `the_file_only_features_are_their_files`; makes true FEA-30.
2. [x] Fixes of the `high` review of the whole issue; verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: 32d3f8328b274c154e7f5659943905488ec24746
- No replacement workflow skill: a project tracking its issues elsewhere writes its own.
- The seeded `ROADMAP.md` stays as it is (SYN-4; `release` reads its milestones).
- No `needs` either way: `harness-measure` keeps its per-session measures without issues.

## Summary of deliverable 1
done: feature `issue-files` declared (3 skills, `cardinal-issues.just`), FEA-30 and its non-goals, usage and README.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `docs/usage.md`, `README.md` |
| code review | low, see below |

red: FEA-30: `features.toml declares no issue-files`
Code review: no findings
not verified: none
decisions needed: none

## Summary of deliverable 2 (high review of the issue)
done: the refusal of the review gate no longer names `issue-workflow`; the test of the file-only features sorts its
expected list and checks `needs` for `issue-files` only; follow-up issue for the readers of `issues/`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a (no behaviour documented changed) |
| code review | low, see below |

red: none
Code review: no findings (low); high: 9 findings
- fixed: the refusal of the review gate cited `issue-workflow, step 3.5`, a skill the feature leaves out.
- fixed: the expected files of the test were not sorted after the concat; the `needs` check ran for FEA-27 too.
- issue `issues/done/leave_the_issues_directory_alone_with_the_issue_files_off.md`: `review-gate` and `deliverable-size`
  leave out `issues/`, `validation-stop` reads it, with the feature off.
- not fixed, decided at the plan: the seeded `ROADMAP.md` stays (its milestones then go `done` by hand), the skills
  citing `skill:create-issue` stay worded (non-goals of FEA).
- not fixed: `sync` leaves the emptied skill directories, as for every feature; a directory with no `SKILL.md`
  loads nothing.
not verified: none
decisions needed: none
