# Budget the fixed context of a project

# Goal
`harness run fixed-context` measures the context every session pays before any work (`CLAUDE.md`, `AGENTS.md`,
skill descriptions) but nothing bounds it, so it can only grow. Let a project set a budget and have
`harness check`, `sync` and `update` warn when the project goes over it.

# Findings
* `specs/harness-report.md` REP-1: `fixed-context` prints `claude_md_bytes`, `agents_md_bytes`,
  `skill_description_bytes` per project and revision, read from the transcripts' revisions; REP-7 reports the
  means per group. No requirement compares them to a limit.
* `harness check` already prints `warning:` lines that do not fail it (`crates/harness/src/settings.rs`).
* `google-research/harness2` (`harness2/core/tree.py`, `validate`) rejects a harness whose always-on text exceeds a
  character cap, before any run, and its prompts require paying for added always-on text by cutting some.

# Approach
1. Measure first: the current fixed context of this repository and of a few consumers, to pick a default.
2. A `fixed_context_budget` value in `[project]` of `.claude/harness.toml` (bytes; absent means no check).
3. `harness check`, `sync` and `update` print a `warning:` line naming the total, the budget and the largest part,
   reusing the measure of `fixed-context` on the work tree rather than on a past revision.
4. Candidate alternative: a test of this repository only, bounding what `files/` ships (`AGENTS.md`, skill
   descriptions), with no consumer setting.

# Specs
* `specs/sync.md` (the warnings of `check`, `sync`, `update`) or `specs/harness-report.md`; new requirements ids
  decided at `issue-workflow` step 1.

# Resources
* `crates/harness/src/tools/fixed_context.rs`, `crates/harness/src/settings.rs`, `specs/harness-report.md`
* `https://github.com/google-research/harness2`, `harness2/core/tree.py`

# Hints
* A warning, not a failure: the budget informs the user, it does not block a sync.
* The measure must not count what Claude Code loads on demand (skill bodies).

# Tracking

Plan: validated

## Understanding
A `fixed_context_budget` (bytes) in `[project]` of `.claude/harness.toml`; `check`, `sync` and `update` warn, never
fail, when the work tree's `CLAUDE.md` + `AGENTS.md` + skill descriptions exceed it (SYN-38, SYN-39), measured as
`fixed-context` measures a revision. Out of scope: the repository-only test of approach 4, the report tools.
Measured on 2026-10-28: this repository 7,667 bytes (CLAUDE.md 2,120, AGENTS.md 2,545, 8 skills ~3,000); the other
local consumer ~10,700 bytes (CLAUDE.md 4,891, 10 skills ~3,300).

## Plan
1. [x] Budget warning: share the description measure of `tools/fixed_context.rs` with a work-tree measure, print the
   warnings after `check`, `sync` and `update` (as the retention ones), document the key in `seed/.claude/harness.toml`
   and the docs, set this repository's budget; verified by `tests/budget_cli.rs` through the binary (~350 lines);
   makes true SYN-38, SYN-39.
2. [x] Fixes of the `high` review: `budget.rs` reads the configuration through `read_config`; the finding outside
   this issue's requirements filed as `issues/done/count_every_always_loaded_memory_file.md`.

## Decisions
- Mode: autonomous
- Base commit: aeecae964dfff6f6ff8c49bf6b729aca43d1ded8
- Budget of this repository: 10000 bytes (~30% over the 7,667 measured).
- Seed: a commented `# fixed_context_budget = 12000` line in `seed/.claude/harness.toml`, no check by default.
- No repository-only test bounding `files/` (approach 4): this repository's own budget covers what it ships.

## Deliverable 1 summary
- done: `budget.rs` measures the work tree (sharing `description_bytes` of `fixed-context`) and `check`, `sync`,
  `update` print its warning after the retention ones; budget 10000 here, commented line in the seed; docs updated.

| check | result |
|---|---|
| tests (`tests/budget_cli.rs`, 6) | pass |
| `just check` (fmt, clippy, tests, specs --strict) | pass |
| docs | `docs/architecture.md`, `docs/usage.md`, `seed/.claude/harness.toml` |
| code review (low) | no findings |

- red: SYN-38, SYN-39: `left: []` (no budget warning printed) against the expected warning line.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2 summary
- done: fixes of the `high` review of `aeecae9..HEAD` (5 findings).

| check | result |
|---|---|
| tests (`tests/budget_cli.rs`) | pass, unchanged |
| `just check` | pass |
| code review (low) | no findings |

- red: none (a refactor under the green tests)
- Code review: 5 findings (high review of the issue)
  1. Symbolic links to skill directories are followed, unlike `git ls-tree`: kept, Claude Code loads them, so the
     work tree measure is the truer one; REP-2's counting of the front matter is what SYN-38 shares.
  2. The key outside `[project]` is ignored silently: kept, SYN-38 names `[project]` like the other project keys.
  3. `.claude/CLAUDE.md` and `CLAUDE.local.md` not counted: outside this issue, filed as
     `issues/done/count_every_always_loaded_memory_file.md` (both measures must change together).
  4. Second parser of `harness.toml`: fixed, `read_config` reused.
  5. A SKILL.md that is not UTF-8 counts 0: kept, as `fixed-context` does; Claude Code would not parse it either.
- not verified: none
- decisions needed: none

