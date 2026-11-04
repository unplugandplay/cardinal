# Install the new harness before merging a change of the hook commands

# Goal
When an issue changes the commands the hooks run (a new `harness run` tool, a new subcommand of `harness`), the hooks
of the main worktree switch at the fast-forward of the merge. If the installed `harness` does not know the new
command, every hook fails and the agent is locked out. The merge procedure must install the new binary first.

# Findings
* Closing `ship_the_harness_scripts_as_prebuilt_binaries` (2026-10-27): after `git merge --ff-only`, the PreToolUse
  hook `harness run review-gate check` exited 2 (`unrecognized subcommand 'run'`), so Claude Code refused every Bash
  command, `just install` included; the Stop hook `harness run clippy-gate` refused each end of turn, so the agent
  repeated the same request for four turns. The user had to run the install by hand.
* The issue had noted the need for `just install`, but placed it after the fast-forward.
* `harness run` now exits 3 when it cannot get a tool (BIN-14), but an installed binary without `run` exits 2 through
  its argument parser, which no change of the new code can prevent.

# Approach
1. In `issue-workflow` "Merge back" (`details.md`): when the branch changes `entries/.claude/settings.json` or the
   `harness` crate, install the binary of the branch (`cargo install --path crates/harness --locked` from the
   worktree, or the recipe) before the fast-forward.
2. Or detect it: a check run before the merge that compares the commands of the managed hooks with what the installed
   `harness` accepts, and stops with the command to run.
3. For consumers: say in the README that a project moving to a revision with new hook commands needs the matching
   `harness` installed first, or have `harness update` refuse when the installed binary is older than the target.

# Resources
* `files/.claude/skills/issue-workflow/details.md` ("Merge back"), `entries/.claude/settings.json`
* `specs/tool-binaries.md` (BIN-14), `crates/harness/src/main.rs` (`run`)

# Hints
* The installed `harness` is the launcher of every project of the machine; an install is not reversible by the agent
  once the hooks refuse its commands.

# Specs
* `specs/tool-binaries.md` (BIN-15, BIN-16)

# Tracking

Plan: validated

## Understanding
Stop a hook change from reaching a project whose installed `harness` cannot parse the new hook commands: detect it
before the fast-forward (this repository) and before `update` writes (consumers). Out of scope: the words after the
subcommand (the pinned binary runs them), installed launchers older than the check. Means: approaches 1 + 2 + 3 of the
issue, the check being `harness help <subcommand>` run against the `harness` on the path.

## Plan
1. [x] Launcher check in `check`: a hook command `harness <sub> ...` of `.claude/settings.json` whose `<sub>` the
   `harness` on the path refuses fails `check`; verified by tests with a fake `harness`; makes true BIN-15.
2. [x] `update` refuses such a target, naming `cargo install --git <source> --rev <commit> --locked`, and warns under
   `--dry-run`; verified by tests with a fake `harness`; makes true BIN-16.
3. [x] Procedure and docs: `issue-workflow` "Merge back" runs `just check-harness` in the worktree before the
   fast-forward and installs the branch's `harness` (`just install` from the worktree) when it fails; the README says a
   consumer installs the matching `harness` when `update` refuses; verified by `just check-harness` and a reread.

## Decisions
- Mode: autonomous
- Base commit: 61cd2097e77756fcd581ca17d7de3d628cbcd552
- Questions: no override of the refusal of `update` (install, then run it again); `sync` not checked.

## Deliverable 1
done: `harness check` fails on each hook command `harness <sub>` whose `<sub>` the `harness` on the path refuses
(`harness help <sub>`, asked once per subcommand); no `harness` on the path, nothing looked at.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BIN-15 traced, spec `draft` until BIN-16) |
| docs | README, `docs/architecture.md`, `check` help |
| code review | low, see below |

red: `assertion left == right failed: [] left: 0 right: 2` (no hook command refused before the code)

Code review: no findings

not verified: a real old launcher (one without `run`) against these settings; the fake `harness` stands for it.

## Deliverable 2
done: `harness update` reads the hooks of the target revision (`entries/.claude/settings.json`) and, when the
`harness` on the path refuses one, writes nothing and fails with one line per hook naming
`cargo install --git <source> --rev <commit> --locked`; under `--dry-run` the same lines are warnings.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BIN-16 traced, `specs/tool-binaries.md` back to `accepted`) |
| docs | README, `docs/architecture.md`, `update` help and doc comment |
| code review | low, see below |

red: `called Result::unwrap_err() on an Ok value: UpdateReport { ..., applied: true, ... }`

Code review: 1 finding
- An unreadable or invalid target fragment skips the check: not fixed, no defect, since `entries::of_source` fails the
  dry-run sync on it before anything is written; a comment says so.

not verified: a real `harness update` from GitHub (network); the tests use a `file://` source and a fake `harness`.

## Deliverable 3
done: `issue-workflow` "Merge back" step 1 runs `just check-harness` in the worktree after the rebase and installs the
branch's `harness` before leaving it when a hook is refused; the README explains the launcher and the refusal of
`update`; `docs/architecture.md` names the step.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | README, `docs/architecture.md`, `details.md` (synced) |
| code review | low, see below |

red: none (docs and procedure only)

Code review: 1 finding
- The shared "Merge back" step named `cargo install --path crates/harness`, which only this repository has: fixed, the
  step names that install for the local source `.` and `cargo install --git <source> --rev <rev> --locked` elsewhere.

not verified: a real merge of a branch that adds a hook subcommand (no such branch here).

## Deliverable 4 (fixes of the high review)
Plan line: 4. [x] Fixes of the `high` review of `61cd209..f3a7882`; verified by the launcher tests; amends BIN-15, BIN-16.

done: `check` names the exact install (`cargo install --git <cargo url> --rev <rev> --locked`, or `--path
<source>/crates/harness`), and "Merge back" runs the install it prints (finding 1, 7); quoted subcommands are
unquoted (3); a failed run of `harness` keeps the refusals found so far (4); BIN-16 judges the managed entries of the
target, as the code does (6); `update` prints the install once (8); the settings text no longer shadows the
`settings` module (9); the fake's log path is quoted (10).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (`specs/tool-binaries.md` BIN-15, BIN-16 reworded) |
| docs | `docs/architecture.md`, `details.md` (synced) |
| code review | high on the issue, low on this fix |

red: `left: 3 right: 2` (the quoted `harness 'run' quoted` refused) and the install printed twice by `update`

High review: 10 findings
- 1, 3 (quotes), 4, 6, 7, 8, 9, 10: fixed as above.
- 2 (`sync` does not probe): not fixed, `sync` was left out at validation.
- 3 (a leading global flag such as `harness --root . x` is skipped): not fixed, no hook of the harness has one.
- 5 (exit 3 on a parse error of the launcher): out of the requirements, opened as
  `issues/done/exit_3_when_the_launcher_cannot_parse_its_arguments.md`.

Code review: no findings

not verified: an scp-style source (`git@host:path`) printed by `check`; `cargo_url` converts it and has its own tests.

Status: done (4 deliverables, `3e35fc8`, `7ef83e8`, `f3a7882`, `d6df29e`).
