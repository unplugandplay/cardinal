---
priority: low
why: 'a conflict on pre-commit.user is rare, but check then sends the user to a sync that always fails'
---
# Name the chained hook conflict in check

# Goal
When the project has a `pre-commit` of the user and a `pre-commit.user` (a dangling link included), `sync` refuses to
install the gate (SYN-19), yet `check` only reports `missing: native review gate`, which `sync` cannot fix. Expected:
`check` names the conflict and what to merge by hand.

# Findings
* `crates/cardinal/src/gate.rs`, `problem`: `State::Missing | State::Foreign` both print `missing: {GATE}`; `plan`
  bails on `Foreign` with a `pre-commit.user` (found by the `high` review of
  `guard_a_dangling_chained_hook_at_the_gate_install`).

# Approach
1. Write the failing test of SYN-23 with both hooks present, then share the test of `plan` with `problem`.

# Specs
* `specs/sync.md` SYN-23 (amend: the line printed on the conflict), SYN-19.

# Resources
* `crates/cardinal/src/gate.rs`, `crates/cardinal/tests/gate_cli.rs`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
`check` prints `conflict: native review gate (...)` instead of `missing: native review gate` when a foreign `pre-commit`
and a `pre-commit.user` (a dangling link included) coexist, naming the manual merge; `plan` and `problem` share one
test of the conflict so they never disagree. Out of scope: changing what `sync` does (SYN-19 stays).

## Plan
1. [x] Conflict line in `check`: a `State`-level conflict test shared by `plan` and `problem`, a test in
   `gate_cli.rs` with a regular and a dangling `pre-commit.user`; verified by that test; makes true SYN-23 (amended). (autonomous)
2. [x] Docs and `high` review fixes: the conflict text shared by `sync` and `check` with the paths of the hooks
   directory, FEA-24, `docs/usage.md`, `docs/gates.md`; verified by the gate tests; makes true SYN-23, SYN-19.
   (autonomous)

## Deliverable 1
- done: `check` prints the conflict line of SYN-23; `gate::conflicts` is the one test `plan` and `problem` share.
- red: SYN-23: `left: ["missing: native review gate"] right: ["conflict: native review gate (...)"]`
- checks: `just check` ok; deliverable size 53 lines, within.
- Code review: no findings
- not verified: none.


## Decisions
- Mode: autonomous
- Base commit: 599d3e26eae163abd185f1964695f1193406d0b4
- Conflict line: the text of SYN-23 (1 a), so the next `sync` installs the gate and chains `pre-commit.user`, moving nothing.

## Deliverable 2
- done: one text `merge_by_hand` for the error of `sync` and the line of `check`, with the full paths and the dangling
  link case; the test checks the reason of the refusal of `sync` and an unchanged hooks directory; docs and FEA-24.
- red: none (the fixes of the `high` review, under green tests).
- Code review: no findings (`low`, the fixes)
- Code review (`high`, whole issue): 10 findings
  1. no hooks directory in the line: fixed (full paths).
  2. dangling link advice: fixed (`replace it if it is a dangling link`).
  3. interrupted `swap` leaves two hard links: out of scope, `issues/done/finish_an_interrupted_gate_install.md`.
  4. `unwrap_or(false)` on an I/O error: kept, an unreadable hooks directory is no conflict; `sync` reports the error.
  5. test checks only the exit code: fixed (reason and snapshot).
  6. two texts of the conflict: fixed (`merge_by_hand`).
  7. `Foreign` tested twice: fixed (`_ if conflicts`).
  8. FEA-24: fixed.
  9. `docs/usage.md`: fixed.
  10. line wrap of `docs/gates.md`: fixed.
- not verified: none.
