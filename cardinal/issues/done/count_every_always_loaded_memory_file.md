# Count every always-loaded memory file in the fixed context

# Goal
The fixed context measure counts only the root `CLAUDE.md` and `AGENTS.md` (plus the skill descriptions), both in
`fixed-context` (REP-2, on a revision) and in the budget warning of `check`, `sync` and `update` (SYN-38, on the work
tree). Claude Code loads other project memory in every session too, so a project keeping its instructions there is
under-counted and its budget never warns. Make the shared measure count what a session actually loads.

# Findings
* Raised by the `high` review of `issues/done/budget_the_fixed_context_of_a_project.md`: a project whose instructions sit
  in `.claude/CLAUDE.md` measures 0 for `CLAUDE.md`.
* Claude Code documents `.claude/CLAUDE.md` and `CLAUDE.local.md` as project memory, and `@path` imports in a memory
  file; not yet checked which of them this repository's consumers use.
* `AGENTS.md` is counted although Claude Code loads it only through an import from `CLAUDE.md`; the measure assumes it.

# Approach
1. Measure first: which memory files and imports the local consumers use, and their sizes.
2. Count `.claude/CLAUDE.md` with the root `CLAUDE.md` in one shared function used by `tools/fixed_context.rs` and
   `budget.rs`, so REP-2 and SYN-38 stay one measure.
3. Candidate: follow the `@path` imports of the memory files (once each), instead of counting `AGENTS.md` blindly.
4. `CLAUDE.local.md` is personal and kept out of git: count it in the work tree only, or not at all.

# Specs
* `specs/harness-report.md` REP-2 and `specs/sync.md` SYN-38, amended together.

# Resources
* `crates/harness/src/tools/fixed_context.rs`, `crates/harness/src/budget.rs`
* `specs/harness-report.md`, `specs/sync.md`

# Hints
* Keep the two measures one: the budget is only useful if it agrees with what `evaluate-harness` reports.

# Tracking

Plan: validated

## Understanding
Measure the memory files a session loads at launch (`CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/` without
`paths`, their `@` imports; `AGENTS.md` only when imported or when no `CLAUDE.md` exists) with one function shared by
REP-2 (git tree) and SYN-38 (work tree). Out of scope: memory outside the repository, `CLAUDE.local.md`.

Measured on 2026-10-28 (both local consumers): `CLAUDE.md` 2120 and 4891 bytes, `AGENTS.md` 2545 in each, no
`.claude/CLAUDE.md`, `CLAUDE.local.md`, `.claude/rules/` nor `@` import. Claude Code (2.1.312, default setting) reads
`AGENTS.md` only when no `CLAUDE.md` exists or one imports it: the current measure counts 2545 bytes no session loads.

## Plan
1. [x] Shared memory measure + budget: a module reading the memory files through a reader (work tree or git tree),
   following imports; `budget.rs` uses it; verified by `tests/budget_cli.rs` (`.claude/CLAUDE.md`, rules with and
   without `paths`, import, `AGENTS.md` not imported, fallback); makes true SYN-38 (~500 lines).
2. [x] `fixed-context` on the shared measure: the git-tree reader (symbolic links as today); verified by
   `tests/fixed_context.rs`; makes true REP-2 (~300 lines).
3. [x] Fixes of the `high` review: links read in a commit, a file counted once whatever path names it, nested
   fences, an open front matter, one `ls-tree` per commit; verified by `tests/budget_cli.rs` and
   `tests/fixed_context.rs`; makes true SYN-38, REP-2.

## Decisions
- Mode: autonomous
- Base commit: 360b947
- Columns and budget parts kept (`claude_md_bytes`, `agents_md_bytes`; `CLAUDE.md`, `AGENTS.md`), their meaning widened.
- `CLAUDE.local.md` never counted: one measure for the budget and `evaluate-harness`.
- The shared `AGENTS.md` no session loads goes to a separate issue.

## Deliverable 1

- done: `memory.rs` measures the memory files loaded at launch (work tree reader, `Tree` trait for the commit reader of
  deliverable 2); the budget counts through it; docs and the `harness.toml` comments say what is counted.
- red: SYN-38: `the fixed context is 1110 bytes ... the largest part is AGENTS.md (1000 bytes)` instead of 193 and
  `CLAUDE.md` (183): `.claude/CLAUDE.md`, rules and imports not counted, an `AGENTS.md` no session loads counted.
- Code review: no findings
- not verified: the measure against `/context` of a real session (the doc of Claude Code is the reference).
- decisions needed: none.

## Deliverable 2

- done: `fixed-context` reads the memory files of a commit through the shared measure (`CommitTree`), so REP-2 and
  SYN-38 are one count; usage docs say what the columns hold.
- red: REP-2: row `r1` read `10 4` instead of `44 4`, `r2` `0 10` instead of `5 10`, the harness row `2 2` instead of
  `2 0`: `.claude/CLAUDE.md`, rules and imports not counted, an `AGENTS.md` no session loads counted.
- Code review: no findings
- not verified: none.
- decisions needed: none.

## Deliverable 3 (fixes of the `high` review)

- done: fixes 1, 3, 4, 6, 7, 8, 9 of the `high` review; not fixed: 2 (a linked rules directory points outside the
  repository, a non-goal of REP-2), 5 (the Claude Code doc says four hops), 10 (an unreadable `CLAUDE.md` is absent
  for Claude Code too; the commit case is fix 1), the code span over two lines of 6 (rare).
- red: SYN-38, REP-2: `95 bytes ... CLAUDE.md (60 bytes)` instead of 65 and 30 (a linked `.claude/CLAUDE.md` counted
  twice); `1071 bytes` instead of 112 (an import in a nested fence counted, an open front matter read as scoped); row
  `r1` `44` instead of `51` (a rule linked with `..` read as 0 in the commit).
- Code review: no findings
- not verified: none.
- decisions needed: none.
