---
priority: medium
why: 'the commit, issue and spec features each own recipes of the one `cardinal.just`, which a feature cannot leave out'
---
# Split the managed recipes by feature

# Goal
`cardinal.just` is one managed file holding the recipes of every feature (`check-commit`, `next-issue`,
`check-specs`…). A feature can leave out whole files only (ADR 0025 rejected marked blocks), so the recipes of a
feature turned off stay. The outcome: the recipes of each feature in a file of their own, imported so that a missing
one is no error, and `cardinal.just` keeping the recipes of the core.

# Findings
* `cardinal.just` holds `waive-review`, `check-commit`, `check-commits`, `close-issue`, `next-issue`,
  `open-worktree`, `close-worktree`, `release`, `check-issue`, `check-specs`, `spec-coverage`, `check-cardinal`.
* The seeded `justfile` imports `cardinal.just`; the installed `just` is 1.58.

# Approach
1. Check that `just` has an optional import (`import?`) and how a seeded `justfile` of an older project picks the
   new files up (a migration of its seeded text, ADR 0017, or `cardinal.just` importing the others itself).
2. Move the recipes into one file per feature, `cardinal.just` importing each optionally.
3. A raised format if an older `cardinal` would misread the layout (ADR 0014).

# Milestone
Optional features

# Resources
* `files/cardinal.just`, `seed/justfile`, ADR 0014, ADR 0017, ADR 0025.

# Hints
* `cardinal.just` importing the others keeps the seeded `justfile` untouched: no migration of a project-owned file.

# Specs
* `specs/features.md` FEA-19; `specs/review-gate.md` REV-17 and `specs/tool-binaries.md` BIN-8 renamed.

# Tracking

Plan: validated

## Understanding
Move the recipes of the commit convention, the issue files, the specs and the review gate out of `cardinal.just` into
four managed files it imports with `import?`; declaring the features themselves stays with their own issues. No
migration (the seeded `justfile` is untouched) and no raised format: an older `cardinal` copies the new files as any.

## Plan
1. [x] Split the recipes: the four files and `import?` lines in `files/cardinal.just`, `just sync`, REV-17 test pointed
   at `cardinal-review.just`, docs (README, architecture, usage); verified by a test running `just --list` on
   `cardinal.just` with all, and with each one missing; makes true FEA-19.
2. [x] Fixes of the `high` review of the issue: FEA-19 narrowed to `cardinal.just` (the seeded `check: check-specs`
   and the CI job go to their features' issues, `turn_off_the_specs` notes them), the test reads `just --summary` as
   sets, the docs name the `cardinal-<part>.just` files; verified by the FEA-19 test and `just check`.

## Decisions
- Mode: autonomous
- Files at the root next to `cardinal.just` (`cardinal-<part>.just`); `waive-review` moves to `cardinal-review.just` (answers 1 a, 2 a).
- `just` 1.58 accepts a nested `import?` and a missing file; the root `justfile` of this repository still overrides
  `check-specs` and `check-cardinal` (`allow-duplicate-recipes`), checked in a scratch directory.
- The seeded `check: check-specs` still needs `cardinal-specs.just`: left to `turn_off_the_specs`.
- Base commit: b2215631c23341a35b81a0f213c032202b3ae863

## Deliverable 1
- done: four `cardinal-<part>.just` files imported by `import?`; REV-17 test reads `cardinal-review.just`; README,
  architecture, CLAUDE.md and the docs map name them.
- red: FEA-19: `fs::copy` of `files/cardinal-commits.just` failed (no such file).
- checks: `just check` pass (fmt, clippy, tests, specs), `just check-cardinal` pass, docs updated.
- Code review: no findings
- not verified: a consumer synced with an older `just` lacking `import?` (CI installs the latest).

## Deliverable 2
- done: the `high` review findings settled: FEA-19 and its scenario narrowed, a non-goal for the project's own
  recipes and the CI, `turn_off_the_specs` findings, test on `just --summary`, `docs/usage.md`, `update-docs` and
  the comment of `cardinal.just` reworded.
- red: none (review fixes, the FEA-19 test rewritten green).
- not fixed: the REV-17 test copies `cardinal-review.just` alone (the import is FEA-19's test); no minimum `just`
  version enforced (CI installs the latest).
- Code review: no findings
