# List the user-level instruction files in the instruction audit

# Goal
`harness run audit-sources` lists the instruction files of each project's working directories (AUD-1) but not the
user-level ones every session loads too: `CLAUDE.md` and the `rules/**/*.md` files of the Claude Code configuration
directory of the home directory. A rule the user keeps there is invisible to `audit-instructions`, which may propose
it again. List them once, apart from the projects.

# Findings
* 2026-10-28, step 2 of `issues/done/list_the_always_on_rules_in_the_instruction_audit.md`: left out of scope (decision
  `1 a`), AUD-1 lists them among its non-goals.

# Approach
1. Print them once before the projects (`instructions: <path>`), resolved with `memory.rs` from the configuration
   directory (`CLAUDE_CONFIG_DIR` when set), imports included.
2. Or name them in the "Collect" step of `audit-instructions` only, the skill reading them itself.

# Specs
* `specs/instruction-audit.md`: a new requirement, the non-goal withdrawn.

# Resources
* `crates/harness/src/tools/audit_sources.rs`, `crates/harness/src/memory.rs`
* `files/.claude/skills/audit-instructions/SKILL.md` (step "Collect")

# Tracking

Plan: validated

## Understanding
`audit-sources` prints the user-level instruction files (`CLAUDE.md`, `rules/**/*.md` of the configuration directory,
imports included) once, before the projects (approach 1). Out of scope: the managed policy files, imports leaving the
configuration directory.

## Plan
1. [x] User-level files: `memory.rs` reads a configuration directory's layout (`CLAUDE.md`, `rules/`), `audit-sources`
   prints them before the projects, the "Collect" step of `audit-instructions` names them; verified by the binary
   test of `tests/audit_sources.rs` (`CLAUDE_CONFIG_DIR` set, then unset with `HOME`); makes true AUD-2 (~150 lines).
2. [x] Fixes of the `high` review: AUD-1 leaves out the configuration directory rather than `~/.claude`, a relative
   `CLAUDE_CONFIG_DIR` printed absolute, `Layout.agents` as a list, the tests isolated from the machine's user files;
   verified by `tests/audit_sources.rs`; makes true AUD-1, AUD-2.

## Decisions
- Mode: autonomous
- Base commit: 700a7cd5a59c53e6cd9146fdbe5b1432180aab8e
- 1 a: an import leaving the configuration directory is not listed, as AUD-1 reads nothing outside the repository.
- 2 a: the rules scoped by `paths` are listed too.

## Deliverable 1
- done: `audit-sources` prints the user-level instruction files before the projects (`memory::Layout`, `config_dir`
  shared with the retention warnings); "Collect" of `audit-instructions` and `docs/architecture.md` name them.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (AUD-2 traced) |
| docs | skill, architecture |
| code review | low, see below |

- red: AUD-2: `assertion left == right failed, left: [] right: [<config>/CLAUDE.md, <config>/notes.md, <config>/rules/a.md, <config>/rules/sub/b.md]`
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: fixes of the `high` review (9 findings, below); AUD-1 and AUD-2 reworded; follow-up issue
  `issues/done/read_the_transcripts_from_the_configuration_directory.md`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | spec only |
| code review | low, see below |

- red: AUD-1, AUD-2: not observed, the assertions were written with the fixes (a relative `CLAUDE_CONFIG_DIR` printed `config/CLAUDE.md`; the `.claude/` of the home directory dropped under another configuration directory)
- Code review: no findings
- `high` review, 9 findings: fixed: the tests read the machine's user files, AUD-1 filtered `HOME` rather than the
  configuration directory, a relative `CLAUDE_CONFIG_DIR`, `Layout.agents` as a flag; spec reworded: `~` and absolute
  imports not listed (decision 1 a), imports of scoped rules not followed (as AUD-1); not fixed: the imports of a
  user file linked outside the configuration directory (outside it, as AUD-2 says), the second read of the rules (the
  existing pattern of AUD-1, a few small files); issue: `projects_root` ignores `CLAUDE_CONFIG_DIR`.
- not verified: the red of the fixes
- decisions needed: none
