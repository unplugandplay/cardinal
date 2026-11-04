# List the always-on rules in the instruction audit

# Goal
`harness run audit-sources` lists as instruction files only `CLAUDE.md` and `AGENTS.md` of each project's working
directory. Since the rule "Reports to the user" moved to the managed `.claude/rules/reports-to-the-user.md`, the
audit no longer sees it, nor any rule a project keeps under `.claude/rules/`, so `audit-instructions` may propose a
rule or a skill that already exists. List every memory file a session loads at launch instead.

# Findings
* 2026-10-28, high review of `issues/done/load_the_shared_agents_md_in_every_session.md`: `INSTRUCTION_FILES` in
  `crates/harness/src/tools/audit_sources.rs` holds `CLAUDE.md` and `AGENTS.md` only.
* `crates/harness/src/memory.rs` already resolves the memory files loaded at launch (REP-2): `CLAUDE.md`,
  `.claude/CLAUDE.md`, the `.claude/rules/**/*.md` files without `paths`, their imports.

# Approach
1. List the instruction files from the resolution of `memory.rs` (it would need to return the paths, not only the
   bytes), plus `AGENTS.md` even when not loaded, since the audit reads what the user wrote.
2. Or add `.claude/CLAUDE.md` and the `.claude/rules/*.md` files to the fixed list, the scoped rules included.

# Resources
* `crates/harness/src/tools/audit_sources.rs`, `crates/harness/src/memory.rs`
* `files/.claude/skills/audit-instructions/SKILL.md` (step "Collect")
* `specs/harness-report.md` REP-2

# Specs

* `specs/instruction-audit.md` (AUD-1)

# Tracking

Plan: validated

## Understanding
`audit-sources` lists, per working directory, the memory files of REP-2 (resolved by `memory.rs`, which returns their
paths) plus `AGENTS.md`, `.claude/AGENTS.md` and the scoped rules: every file the user wrote. The user-level
files (home directory) stay out of scope. Means: the issue's approach 1, with approach 2's scoped rules.

## Plan
1. [x] List the instruction files: `memory.rs` returns the paths of the loaded files (`measure` sums them),
   `audit_sources.rs` prints them with the extra files, `audit-instructions` names them in "Collect"; verified by
   `tests/audit_sources.rs`; makes true AUD-1.
2. [x] Fixes of the `high` review; verified by `tests/audit_sources.rs`; makes true AUD-1 (amended).

## Decisions
- Mode: autonomous
- 1 a: the files of the home directory stay out of scope (follow-up issue); 2 a: the scoped rules are listed.
- Base commit: 54d60018e2895e938e6555923df93bdcedf3c85d

## Deliverable 1

done: `audit-sources` lists every instruction file of a working directory through `memory::instruction_files`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | AUD-1 traced, spec accepted |
| docs | audit-instructions "Collect", architecture link |
| code review | `low`: no findings |

red: AUD-1: `left: ["CLAUDE.md", "AGENTS.md"]`, expected the five files of the scenario.

Code review: no findings

not verified: none. decisions needed: none.

## Deliverable 2: fixes of the `high` review

done: the `high` review of `54d6001..933fa06` (9 findings) settled; a rule linked outside the directory stays listed by
its absolute path (AUD-1 amended), and the test checks every `instructions:` line.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | AUD-1 amended and traced |
| docs | audit-instructions "Collect" names `.claude/AGENTS.md`; doc comments wrapped |
| code review | `low`: no findings |

red: none (the code already printed the absolute path; the test filtered it out).

Code review: no findings

High review: 9 findings
1. absolute path of a file linked outside: kept, the user wrote it; AUD-1 states the form.
2. test filtered the other lines: fixed.
3. ancestors of the working directory: `issues/done/resolve_the_instruction_files_as_a_session_loads_them.md`.
4. imports of a linked file: same issue (behaviour of REP-2 before this issue).
5. rules read twice: not fixed, a few files per directory.
6. `Loaded.agents` against `path`: documented on the fields.
7. `.claude/AGENTS.md` missing from the skill: fixed.
8. long doc comment lines: fixed.
9. code review twice in the summary: the format of issue-workflow asks for both.

Follow-up: `issues/done/list_the_user_level_instruction_files_in_the_audit.md` (decision `1 a`).
