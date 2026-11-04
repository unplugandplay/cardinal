# Stop exiting 2 outside a refusal

# Goal
`harness run` with no tool or a tool the pinned binary does not know exits 2 (BIN-7), which Claude Code reads as the
refusal of the hook: a hook command that names a mistyped or missing tool (written by the project, in
`settings.local.json`, or from a newer revision than the pinned one) blocks every command of the agent. An exit 3,
as BIN-14 and BIN-17 already give when the tool cannot be got or the launcher cannot parse its arguments, would tell
the user without locking the agent out. The same holds for the runtime errors of `init`, `sync`, `update` and
`check`, which `main` exits with 2: a hook that runs one of them and hits an error blocks the agent too.

# Findings
* `crates/harness/src/tools/mod.rs` (`run`, BIN-7) prints the list of the tools and returns 2 when the name is
  missing or unknown.
* Left out of `exit_3_when_the_launcher_cannot_parse_its_arguments` (2026-10-27): that issue only covers the parser of
  the launcher.
* `crates/harness/src/main.rs` (`main`) maps every `Err` of `init`, `sync`, `update` and `check` to exit 2; the
  `high` review of `exit_3_when_the_launcher_cannot_parse_its_arguments` (2026-10-27) named it.
* BIN-15 and BIN-16 do not look at the words after `harness run`: the pinned binary judges them.

# Approach
1. Amend BIN-7 to exit 3 (keep the list of the tools on stderr) and its test.
2. Exit 3 (or 1) instead of 2 on a runtime error of `main`, with a requirement in `specs/sync.md`.
3. Check the recipes and skills that may rely on exit 2 of an unknown tool or of an error.

# Specs
* `specs/tool-binaries.md`: amends BIN-7; `specs/sync.md`: the exit code of an error of `main`.

# Resources
* `crates/harness/src/tools/mod.rs`, `crates/harness/src/main.rs`, `specs/tool-binaries.md` (BIN-7, BIN-14, BIN-17)
* `issues/done/exit_3_when_the_launcher_cannot_parse_its_arguments.md`

# Hints
* Claude Code blocks on exit 2 only; any other non-zero code is a non-blocking error shown to the user.

# Tracking

Plan: validated

## Understanding
`harness` exits 2 only where a hook means to refuse: an unknown or missing tool of `run` (BIN-7) and a runtime error
of `init`, `sync`, `update` or `check` (SYN-37) exit 3 instead. The exit codes of the tools themselves (a gate that
refuses, `commit-check` on a broken setup) are out of scope: they are the tool's own verdict.

## Plan
1. [x] Unknown tool exits 3: `tools::run` returns 3, the list of the tools kept on stderr; verified by
   `no_tool_or_an_unknown_one_exits_3_listing_the_tools`; makes true BIN-7.
2. [x] Runtime error of `main` exits 3: the `Err` arm uses `CANNOT_RUN`; verified by a CLI test (`sync` with no
   `harness.toml`, `check` with no lock source) asserting exit 3 and `error:` on stderr; makes true SYN-37.
3. [x] Fixes of the `high` review: one shared `NOT_A_REFUSAL` constant, the caveat of a binary pinned before BIN-7,
   the exit code of `commit-check` in its skill, a real `init` error and a drift exiting 1 in the tests.

## Decisions
- Mode: autonomous
- Base commit: 71e615bb229f467b7b10280a8b7604a4f5197024
- Exit code of an error of `main`: 3 (1 stays drift or conflict).

## Summaries

### 1. Unknown tool exits 3
done: `harness tool` with no tool or an unknown one exits 3, the list of the tools still on stderr; BIN-7 and
`docs/architecture.md` say so. SYN-37 moves to deliverable 2, where its test is (`just check-specs` fails on an
untraced requirement).

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | `just check-specs` green |
| docs | `docs/architecture.md` |
| code review | `/code-review low` |

red: `assertion left == right failed: [] left: 2 right: 3`

Code review: no findings

not verified: none

decisions needed: none

### 2. Runtime error of `main` exits 3
done: an error of `init`, `sync`, `update` or `check` prints `error: ...` and exits 3 (SYN-37, new); 1 stays a drift
or a conflict; `docs/architecture.md` says so. No recipe or skill read exit 2 of an error.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | `just check-specs` green |
| docs | `docs/architecture.md` |
| code review | `/code-review low` |

red: `["sync"]: error: no .claude/harness.toml ... left: Some(2) right: Some(3)`

Code review: no findings

not verified: none

decisions needed: none

### 3. Fixes of the `high` review
done: `NOT_A_REFUSAL` (lib) replaces `CANNOT_RUN` and the literal 3 of `tools::run`; `docs/architecture.md` rewrapped,
says a binary pinned before BIN-7 still exits 2; the `commit-message` skill says `harness run commit-check` exits 3
with no `harness.toml`; the SYN-37 test makes `init` fail on an existing `harness.toml` and checks that a drift exits 1.

Findings of the `high` review not fixed:
- The pre-commit gate lets a commit through on exit 3, now also an unknown `review-gate`: `sync` installs the gate
  only from a source whose hooks run `review-gate` (SYN-22), and that revision's binary has it, so only a hand-edited
  pin hits it; SYN-18 already chose a warning over a locked commit there.
- The `harness` helper of `exit_codes_cli.rs` next to the `stderr` one of `retention_cli.rs`: two short helpers with
  different outputs; a shared module is not worth it.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | `just check-specs` green |
| docs | `docs/architecture.md`, `files/.claude/skills/commit-message/SKILL.md` |
| code review | `/code-review low` |

red: none (fixes of a review; the reworked `init` case was first red, `left: Some(0) right: Some(3)`, from a wrong setup of the test)

Code review: no findings

not verified: none

decisions needed: none
