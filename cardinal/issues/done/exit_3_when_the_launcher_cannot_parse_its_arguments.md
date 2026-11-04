# Exit 3 when the launcher cannot parse its arguments

# Goal
A hook command the installed `harness` does not know (a new subcommand, a new global flag) exits 2 through the clap
parser, which Claude Code reads as the refusal of the hook: every hook then blocks and the agent is locked out. BIN-15
and BIN-16 probe the hooks before they take effect, but only through `check` and `update`; any other way a hook
command reaches a project still locks it out. A launcher that exits 3 on a parse error, as `run` already does when it
cannot get a tool (BIN-14), would never block the agent, whatever the command.

# Findings
* The `high` review of `install_the_new_harness_before_merging_a_hook_change` (2026-10-27) named it as the cause the
  probes work around: `main` uses `Cli::parse`, which exits 2 on an unknown subcommand.
* Paths the probes do not cover: `sync` of a local source, a hook of `settings.local.json`, a hook written by the
  project, a hook command that starts with an environment variable or an absolute path.
* Only launchers installed after the change benefit: an older one still exits 2.

# Approach
1. `main` parses with `Cli::try_parse`; on a parse error, it prints clap's message and exits 3, except for
   `--help` and `--version`, which keep exit 0.
2. Decide whether a mistyped command typed by a person (not a hook) may exit 3 rather than 2, or limit it to the
   commands a hook runs (`run`, the hidden `tool`).

# Specs
* `specs/tool-binaries.md`: a new requirement next to BIN-14.

# Resources
* `crates/harness/src/main.rs` (`main`, `CANNOT_RUN`), `specs/tool-binaries.md` (BIN-14, BIN-15, BIN-16)
* `issues/done/install_the_new_harness_before_merging_a_hook_change.md`

# Hints
* Claude Code blocks on exit 2 only; any other non-zero code is a non-blocking error shown to the user.

# Tracking

Plan: validated

## Understanding
Every argument the launcher's parser refuses exits 3 instead of clap's 2, so no hook command, however it reached the
project, blocks the agent. Out of scope: the exit 2 of a tool the pinned binary does not know (BIN-7), and launchers
installed before the change.

## Plan
1. [x] Exit 3 on a parse error: `main` uses `Cli::try_parse`, prints clap's error and exits 3 unless it is help or
   version (exit 0); docs (`docs/architecture.md`) updated; verified by a test running the binary with an unknown
   subcommand, an unknown flag and `--help`; makes true BIN-17.

## Decisions
- Mode: autonomous
- Every parse error of the launcher exits 3, whatever the subcommand (not only `run` and `tool`).
- The exit 2 of an unknown tool (BIN-7) becomes its own issue.
- Base commit: `882a70cfd61d3f324839509c62596597d89b406a`

## Deliverable 1
done: `main` parses with `Cli::try_parse`; a parse error prints clap's message and exits 3, help and version exit 0.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BIN-17 traced) |
| docs | `docs/architecture.md` updated |
| code review | low, see below |

red: `assertion left == right failed: ["newcmd"] left: Some(2) right: Some(3)`

Code review: no findings

not verified: a real Claude Code hook session with an unknown subcommand.

decisions needed: none. Follow-up: `issues/done/stop_exiting_2_outside_a_refusal.md` (BIN-7, runtime errors of `main`).

2. [x] Fixes of the `high` review: BIN-17 limited to what the parser refuses, stale doc of `launcher.rs`, a single
   error arm in `main`, the test checks clap's usage, paragraph re-wrapped; the runtime errors of `main` that exit 2
   went to `issues/done/stop_exiting_2_outside_a_refusal.md`.

## Deliverable 2
done: the six findings of the `high` review settled (five fixed, one moved to the follow-up issue).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `docs/architecture.md`, `launcher.rs` doc comment |
| code review | low, see below |

red: none (a stricter assertion on an existing test, no new behaviour)

Code review: no findings

not verified: none.

decisions needed: none.
