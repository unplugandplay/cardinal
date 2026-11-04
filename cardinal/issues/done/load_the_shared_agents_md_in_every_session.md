# Load the shared AGENTS.md in every session

# Goal
`files/AGENTS.md` holds rules every session should follow (the commit message format, the output rule "Reports to the
user"), but Claude Code does not load it in the consumers: by default it reads `AGENTS.md` only when the project has
no `CLAUDE.md` or a `CLAUDE.md` imports it. Its rules reach the context only when a skill sends the agent to read the
file. Make the shared rules part of what every session loads, or decide where they belong instead.

# Findings
* 2026-10-28, measured for `issues/done/count_every_always_loaded_memory_file.md`: both local consumers (this repository
  included) have a root `CLAUDE.md` that names `AGENTS.md` in backticks only, no `@AGENTS.md` import; neither has a
  `.claude/CLAUDE.md`, `CLAUDE.local.md` nor `.claude/rules/`.
* Claude Code documentation (memory page, read with v2.1.312): with the default `instructionFiles` setting
  `claude-md-or-agents-md`, a `CLAUDE.md` in the working directory or above it makes Claude read the `CLAUDE.md` files
  only; `AGENTS.md` loads through an import, a symbolic link, or the user setting `claude-md-and-agents-md` (ignored in
  project settings).
* The session context of this repository shows `CLAUDE.md` and no `AGENTS.md`.
* `CLAUDE.md` is owned by each project (neither managed nor seeded), so `sync` cannot add the import itself.

# Approach
1. Measure first: in the transcripts, how often the agent followed the "Reports to the user" format and the commit
   format without a skill that sends it to `AGENTS.md` (`rule-breaches` may already count it).
2. Candidates (can be combined):
   1. `check` warns when a project's `CLAUDE.md` neither imports `AGENTS.md` nor is one with it.
   2. Move the shared always-on rules to a managed `.claude/rules/<name>.md` without `paths`, which Claude Code loads
      at launch with no edit of the project's `CLAUDE.md`; keep `AGENTS.md` for other agents.
   3. `init` seeds a `CLAUDE.md` holding the import for a new project.
3. Weigh the fixed context each candidate adds against the budget (`fixed_context_budget`).

# Specs
* `specs/sync.md`: a new requirement for the warning or the managed rule file, if chosen.

# Resources
* `files/AGENTS.md`, `entries/.claude/settings.json`, `docs/architecture.md` (managed and seeded content)
* `specs/sync.md`, `specs/harness-report.md` REP-2 (what the fixed context measure counts as loaded)
* Claude Code documentation, memory page, section "AGENTS.md"

# Hints
* The fixed context measure counts `AGENTS.md` only when a session loads it: whichever candidate is chosen, the budget
  will report it.

# Tracking

Plan: validated

## Understanding
Make the always-on rule "Reports to the user" part of every session's launch context in every consumer, without
touching the project-owned `CLAUDE.md`: a managed `.claude/rules/reports-to-the-user.md` with no `paths` (candidate
2). The commit format stays in `AGENTS.md`: `commit-message` and the `check-commit` gate already carry it. Out of scope:
candidates 1 and 3, a measure of the report format (no program can check it).

## Measure (2026-10-28, `rule-breaches --summary`, all revisions)

| rule | sessions applied | breached |
|---|---|---|
| `commit-format` | 143 | 2 |
| `commit-attribution` | 143 | 0 |
| "Reports to the user" | not measurable by a program | - |

Fixed context of this repository today: `CLAUDE.md` 2,120 + skill descriptions ~2,830 bytes, budget 10,000. Importing
the whole `AGENTS.md` would add 2,545 bytes for a commit format already enforced; the rule file adds ~450.

## Plan
1. [x] Managed rule file: move the section "Reports to the user" from `files/AGENTS.md` to
   `files/.claude/rules/reports-to-the-user.md` (no front matter), leave a one-line pointer in `AGENTS.md`, repoint the
   skills that cite the section, `CLAUDE.md`, `docs/architecture.md`; `just sync`; verified by a test that syncs a
   project from this harness and finds the rule in the fixed context its `check` reports, and by `just check-harness`.

2. [x] Fixes of the high review: `issue-workflow` points to the rule instead of copying it; the budget test computes
   its bytes from what it wrote and every synced rule; verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: 4c9d16fc6c751da2bf4f848a9772c1163579a140
- `AGENTS.md` keeps a one-line pointer to the rule file, no copy (other agents follow the link).
- The skills that cite the section point to the rule file; the reminder stays.

## Deliverable 1

done: the rule "Reports to the user" is a managed `.claude/rules/reports-to-the-user.md`, loaded at launch in every
consumer; `AGENTS.md` points to it, the skills cite it, `skills.rs` checks `.claude/rules/` references.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `README.md`, `CLAUDE.md`, `docs/architecture.md` |
| code review | low, see below |

red: none traced (managed content, no requirement): the new budget test failed with `the rule is synced`.

Code review: no findings

not verified: a live Claude Code session in another consumer showing the rule in its context (Claude Code documents
the loading of `.claude/rules/` files without `paths`; the measure counts it).

decisions needed: none

## Deliverable 2 (high review of `4c9d16f..151a4fe`)

done: `issue-workflow` cites the rule file instead of its own copy; the budget test derives its expected bytes; the
plan line states the test as written.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | none needed |
| code review | low, see below |

red: none (fixes of a review, no new behaviour).

High review: 7 findings.
1. `audit-sources` misses `.claude/rules/`: outside this issue's requirements, opened as
   `issues/done/list_the_always_on_rules_in_the_instruction_audit.md`.
2. `issue-workflow` copy of the rule: fixed (pointer).
3. Expected bytes assume one rule: fixed (sum of the synced rules).
4. `AGENTS.md` exclusion not asserted: not fixed, REP-2 and SYN-38 tests already prove a named, unimported
   `AGENTS.md` counts nothing (`specs/harness-report.md`, scenario of REP-2).
5. Other agents lose the format: not fixed, decided at validation (pointer, no copy).
6. Plan states another test: fixed (plan line reworded).
7. Magic number `19_983`: fixed (length of the written text).

Code review: no findings

not verified: none

decisions needed: none
