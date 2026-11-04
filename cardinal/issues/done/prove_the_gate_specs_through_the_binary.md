# Prove the gate specs through the binary

# Goal
Move the proofs of the native review gate `sync` installs and of the settings warnings of `check`, `sync` and
`update` from internal unit tests to tests that run the `harness` binary, and the `pre-commit` it installs, on
temporary directories, so these specs and their tests form an acceptance suite that survives a rewrite of the code.

# Findings
* Split from `prove_the_run_and_gate_specs_through_the_binary` at its planning, by size (2026-10-27).
* Internal citations: `crates/harness/src/tests/install_gate.rs` 25 (24 `SYN`, 1 `REV`), plus BIN-9 and BIN-14
  cited with SYN-18.

# Approach
1. Gate install: run the `pre-commit` that `sync` writes with `/bin/sh` and fake `harness` or `rust-script` alone
   on the `PATH`; the move of a user's hook and its chaining (SYN-18, SYN-19, BIN-9).
2. Gate upkeep: reinstall, dry run, linked worktree, `core.hooksPath`, no repository, `check` of the gate (SYN-20 to
   SYN-23).
3. Settings warnings: deny rules, missing hooks, attribution, through `check`, `sync`, `update` and their dry runs;
   REV-19 on the shipped `entries/.claude/settings.json` (SYN-24 to SYN-27, REV-19). Then drop
   `src/tests/install_gate.rs`.

# Specs
* `specs/sync.md` (SYN-18 to SYN-27), `specs/review-gate.md` (REV-19), `specs/tool-binaries.md` (BIN-9): content
  unchanged, only their proofs move.

# Resources
* `crates/harness/src/tests/install_gate.rs`, `crates/harness/tests/common/mod.rs`,
  `crates/harness/tests/settings_entries_cli.rs`

# Hints
* After `prove_the_run_and_gate_specs_through_the_binary`: reuse the fake programs it adds to the test home.
* After `accept_an_internal_proof_with_its_reason`: move a proof to the binary only when that test stays fast and
  stable; otherwise keep the internal test and give its reason on the citation.
* Git runs isolated from the caller (`Home::isolated`): a test run from a git hook must not touch the real repository.

# Tracking

Plan: validated

## Understanding
Move the 25 internal proofs of `src/tests/install_gate.rs` to `tests/*_cli.rs` that run `harness` (and the
`pre-commit` it installs with `/bin/sh`) in the test home; specs unchanged. Out of scope: any change of behaviour.
REV-19 is a property of the shipped `entries/.claude/settings.json`: its proof reads that file against the three
rules the spec names (no binary involved, but no code either).

## Plan
1. [x] Gate install: `tests/gate_cli.rs` runs `sync` then the installed `pre-commit` with fake `harness` or
   `rust-script` alone on its `PATH`, plus the move and chaining of a user's hook; the matching internal tests
   dropped; verified by the new tests; makes true SYN-18, SYN-19, BIN-9, BIN-14 (~600 lines).
2. [x] Gate upkeep: reinstall, idempotence, dry run, linked worktree, `core.hooksPath`, no repository, `check` of the
   gate (with and without `CI`); the matching internal tests dropped; makes true SYN-20 to SYN-23 (~650 lines).
3. [x] Settings warnings: deny rules, missing hooks, attribution through `check`, `sync`, `update` and their dry
   runs; REV-19 on the shipped entries; `src/tests/install_gate.rs` and the helpers of `src/tests.rs` left unused
   dropped; makes true SYN-24 to SYN-27, REV-19 (~600 lines).
4. [x] Fixes of the `high` review: the proofs fail when the behaviour breaks, one fixture of the review gate.

## Decisions
- Mode: autonomous
- Base commit: 95d6a02ceb9ecb92448cdc54d065630a6cb90afb

## Deliverable 1
- done: `tests/gate_cli.rs` proves SYN-18, SYN-19, BIN-9 and BIN-14 through `init`, `sync` and the installed
  `pre-commit` run with `/bin/sh` and only fake `harness` or `rust-script` on its `PATH`; the 10 matching internal
  tests and their fakes dropped from `src/tests/install_gate.rs`.
- red: none (the behaviour exists: the proofs move); mutation checked: the hook ignoring the exit code of the review
  and not passing its arguments to the user's hook fails 3 proofs.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `tests/gate_cli.rs` proves SYN-20 to SYN-23 through `sync`, `sync --dry-run` (tree snapshot unchanged) and
  `check` (with and without `CI`): reinstall, linked worktree, `core.hooksPath`, no repository, nested project, a
  source of an older revision; the 7 matching internal tests and their helpers dropped.
- red: none (the behaviour exists: the proofs move); mutations checked: `core.hooksPath` ignored or a modified gate
  not reported fails 3 proofs.
- Code review: no findings; the `low` review read only the non-test files (the diff is all tests), so the test files
  were reread by hand: nothing found.
- not verified: none
- decisions needed: none

## Deliverable 3
- done: `tests/settings_warnings_cli.rs` proves SYN-24 to SYN-27 through `check`, `sync`, `update` and their dry
  runs, and REV-19 on the shipped `entries/.claude/settings.json` against the three rules the spec names (plus a
  `check` with no deny warning on a project synced on them); `src/tests/install_gate.rs` dropped, and `src/tests.rs`
  keeps only its two pure unit tests. No requirement of the three specs is proven only internally any more.
- red: none (the behaviour exists: the proofs move); mutations checked: the attribution warning off, and the sync
  judging the hooks on disk instead of as it would leave them, each fail a proof.
- Code review: no findings; the `low` review read only the non-test files (the diff is all tests), so the test files
  were reread by hand: nothing found.
- not verified: none
- decisions needed: none

## Deliverable 4 (fixes of the `high` review)
- done: 7 of the 9 findings fixed: the REV-19 `check` half seeds the settings, so a shipped deny rule missing fails
  it; the gate expected once out of the settings asserts the exact problems, with and without `CI`; SYN-24 also
  covers settings with no `deny` list; the `core.hooksPath` reason is asserted on the gate's own line; `gated`,
  `RECORD`, `record_hook`, `set_mode` and `Home::harness_env` moved to `tests/common/mod.rs`.
- not fixed: (4) an up-to-date gate is asserted as no line, not `unchanged`: SYN-20 says `sync` reports nothing then,
  and the same tests prove the gate still considered by its reinstall right after; (8) the deny rules of the REV-19
  proof are the spec's literal ones, not `settings::DENY_RULES`: a boundary test states what the spec promises, and
  its `check` half now catches a drift between the code and the shipped entries.
- red: none (the fixes harden the proofs); mutation checked: a rule removed from the shipped entries, the literal
  loop disabled, fails the `check` half of REV-19.
- Code review: no findings
- not verified: none
- decisions needed: none
- Status: every deliverable done; closing left to the user.
