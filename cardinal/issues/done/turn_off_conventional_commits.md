---
priority: medium
depends_on: [split_the_managed_recipes_by_feature, let_a_feature_need_another]
why: 'a team with its own commit convention cannot adopt Cardinal while `commit-check` and the CI refuse its messages'
---
# Turn off the commit message convention

# Goal
The commit messages follow Conventional Commits: the section of `AGENTS.md`, `skill:commit-message`, the
`check-commit` and `check-commits` recipes, the CI job and rule `commit-format` of `rule-breaches`. A team with
another convention cannot keep its own. The outcome: a feature `conventional-commits` whose removal leaves the commit
format to the project.

# Findings
* `AGENTS.md` is mostly the commit convention; ADR 0025 moves a feature's text to a file of its own.
* `issue-workflow` and `create-issue` call `skill:commit-message`: a dependency (`let_a_feature_need_another`).
* The recipes live in `cardinal.just` (`split_the_managed_recipes_by_feature`).

# Approach
1. Move the commit section of `AGENTS.md` into a file of the feature, `AGENTS.md` keeping what is shared.
2. Gate `commit-check`, `check-commits`, the CI job and rule `commit-format` on the feature (FEA-n each), as the
   pilot did for the attribution.
3. Decide what `issue-workflow` does at its commit step without `commit-message` (a plain message, or the feature
   needed by the issue feature).

# Specs
* `specs/features.md`, `specs/rule-breaches.md` BRE-9 narrowed.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* `files/AGENTS.md`, `files/.claude/skills/commit-message/`, `crates/cardinal/src/tools/commit_check.rs`.

# Hints
* `Expected-effect:` footers are read by `commit-check`: they belong to the measure, not to the convention.

# Tracking

Plan: validated

## Understanding
A feature `conventional-commits` holding the commit convention, which a project turns off to keep its own: the
convention leaves `AGENTS.md` for a file of the feature, and `commit-check`, `check-commits` and rule `commit-format`
go quiet on it. Means differing from the issue: the recipes, the CI job and `skill:commit-message` stay (they carry
the attribution and `Expected-effect:` checks); only their convention part is gated, so no `needs` is required.

## Plan
1. [x] Move the convention: `files/.claude/skills/commit-message/conventional-commits.md` gets the commit section of
   `files/AGENTS.md`; `commit-message` and `issue-workflow` point to it (else to the project's own convention);
   `features.toml` declares `conventional-commits`; `just sync`; verified by a test in `tests/skills.rs`; makes true
   FEA-20.
2. [x] Gate the checks: `commit-check` (hence `check-commits`) keeps only the attribution and `Expected-effect:` lines
   when the feature is off, `rule-breaches` drops `commit-format`; verified by tests of `tools_cli.rs` and
   `rule_breaches.rs`; makes true FEA-21, FEA-22; spec `accepted`.
3. [x] Close the issue: the docs of `base..HEAD` and the fixes of its `high` review.

## Decisions
- Mode: autonomous
- Base commit: f6423729ff8c1b86fbafc47e9b1e3cb3e4776a54
- The convention lives beside the skill (`commit-message/conventional-commits.md`), loaded on demand, not in a rule
  loaded by every session.
- With the feature off, `commit-check` keeps only the attribution and `Expected-effect:` lines: the style rules
  (wrap at 72, emoji, markdown) belong to the convention.

## Deliverable 1

done: the convention moved from `files/AGENTS.md` to `commit-message/conventional-commits.md`, declared as the
feature `conventional-commits`; `commit-message`, `issue-workflow`, `CLAUDE.md` and the references to the types point
to it.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (FEA-19 warnings predate the issue) |
| docs | `CLAUDE.md`, QUA-3, BRE-9 |
| code review | see below |

red: FEA-20: `the_commit_convention_lives_in_its_feature_alone` panicked at the index `features["conventional-commits"]`
(no such feature).

Code review: 1 finding
- The comment of `features.toml` described the gating of deliverable 2 ahead of its code: cut to the files; the
  checks join it with deliverable 2.

not verified: none

decisions needed: none

## Deliverable 2

done: with `conventional-commits` off, `commit-check` and `check-commits` read no `commit_scopes` and report only the
attribution and `Expected-effect:` lines; `rule-breaches` no longer applies `commit-format`; spec `accepted`; docs
(`usage.md`, `README.md`, `features.toml`). Follow-up from the review of deliverable 1:
`issues/done/find_the_root_of_a_done_issue.md`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass, `features.md` accepted |
| docs | `docs/usage.md`, `README.md` |
| code review | see below |

red: FEA-21, FEA-22: `commit-check` reported `format must be <type>(<scope>): <description>`, the blank line, the wrap
and the heading on `Added the parser.`; `rule-breaches` applied `commit-format` to the commit of a project that turned
the convention off.

Code review: no findings

not verified: none

decisions needed: none

## Deliverable 3

done: the fixes of the `high` review of `base..HEAD`; `update-docs` found no other stale reference.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `cardinal-commits.just`, the `about` of `commit-check` |
| code review | see below |

red: none (refactor and docs)

Code review (high): 9 findings
1. The project's own convention pointed to `AGENTS.md`, a managed file: fixed, `CLAUDE.md` only.
2. `release` reads the Conventional Commits types: non-goal of `specs/features.md`, follow-up
   `issues/done/release_a_project_without_the_commit_convention.md`.
3. "Describe only what is in the diff" and the language left with the convention: back in `commit-message`.
4. The `about` of `commit-check` named `AGENTS.md`: fixed.
5. The comment of `check-commit` named `AGENTS.md`: fixed.
6. No migration for the texts of a project citing `AGENTS.md`: not fixed, `AGENTS.md` stays a managed file under the
   same name and a project's prose is not a seeded text (ADR 0017).
7. `commit-check` resolves the root and reads the configuration more than once: not fixed, a few milliseconds for a
   command run once per commit.
8. The `convention &&` guard repeated in `violations`: fixed, the convention checks in `convention_violations`.
9. `COMMIT_TYPES` of `rule-breaches` duplicated `TYPES`: fixed, shared.
