# Prove the run and gate specs through the binary

# Goal
Move the proofs of `harness run`, the launcher and the transcript retention from internal unit tests to tests that
run the `harness` binary on temporary directories, so these specs and their tests form an acceptance suite that
survives a rewrite of the code. The install gate and the settings warnings are
`prove_the_gate_specs_through_the_binary`.

# Findings
* Split from `prove_the_specs_with_black_box_tests` by size (2026-10-27); split again at planning, the gate going
  to `prove_the_gate_specs_through_the_binary` (2026-10-27).
* Internal citations: `crates/harness/src/tests/install_gate.rs` 25 (24 `SYN`, 1 `REV`),
  `crates/harness/src/tests/run.rs` 13 (`BIN`), `crates/harness/src/tests/retention.rs` 6 (`SYN`),
  `crates/harness/src/tests/launcher.rs` 4 (`BIN`).

# Approach
1. Per file of tests, write the boundary test that proves each requirement through the command line, files, exit
   code and output; move the `// spec:` citation to it.
2. Keep an internal test that adds detail as a unit test without a citation; drop the ones the boundary test covers.

# Specs
* `specs/tool-binaries.md`, `specs/sync.md` (SYN-28): content unchanged, only their proofs move.

# Resources
* `crates/harness/src/tests/`, `crates/harness/tests/` (`tools_cli.rs`, `exit_codes_cli.rs`, `retention_cli.rs`)

# Hints
* After `prove_the_sync_specs_through_the_binary`: same method, its helpers for running the binary can be reused.
* The launcher fetches revisions: a boundary test must stay offline (local source or a local clone), per `CLAUDE.md`.

# Tracking

Plan: validated

## Understanding
Re-prove BIN-1 to BIN-6, BIN-10 to BIN-13, BIN-15, BIN-16 and SYN-28 by running the `harness` binary (`run`,
`sync`, `update`, `check`) on temp dirs; the internal tests lose their citation (kept as
unit detail) or are dropped. Offline: fake `curl`, `cargo` and `harness` first on the `PATH` of the test home, local
or `file://` sources. Out of scope: the content of the specs, BIN-7, BIN-8, BIN-14, BIN-17 (already proven at the
boundary); the gate and the settings warnings (`prove_the_gate_specs_through_the_binary`).

## Plan
1. [x] Run: `Home` gains fake programs and a `run` in a working directory with stdin; `tests/run_cli.rs` proves the
   launcher through `harness run` (cached binary, GitHub asset and digest, `cargo install`, local source and its
   last good build, rev resolved in the cached clone) and `sync` (prefetch warning); `src/tests/run.rs` dropped;
   makes true BIN-1 to BIN-6, BIN-10 to BIN-13 (~900 lines).
2. [x] Installed harness: `tests/launcher_cli.rs` proves `check` and `update` against a fake `harness` that knows
   only `run`; `src/tests/launcher.rs` dropped; makes true BIN-15, BIN-16 (~350 lines).
3. [x] Retention: `tests/retention_cli.rs` gains each warning (user settings in `CLAUDE_CONFIG_DIR` or `HOME`,
   unreadable, unlocatable, project files, `CI`); `src/tests/retention.rs` dropped; makes true SYN-28 (~250 lines).

4. [x] Fixes of the `high` review: the proofs fail when the behaviour breaks, the test `PATH` holds no program of
   the caller but a few system ones.

## Decisions
- Mode: autonomous
- Base commit: e3d066c911a67a66956d33d53c58e0fc98f4774f
- Split at planning (1 a): the gate (SYN-18 to SYN-27, BIN-9, REV-19) goes to
  `prove_the_gate_specs_through_the_binary`.

## Deliverable 1
- done: `tests/common/mod.rs` gains fake programs (`curl` failing by default), `command`, `output` and `script`;
  `tests/run_cli.rs` proves BIN-1 to BIN-6 and BIN-10 to BIN-13 through `harness run`, `sync` and `update`;
  `src/tests/run.rs` dropped (its `script` helper copied into `src/tests/launcher.rs` until deliverable 2).
- red: none (the behaviour exists: the proofs move); mutations checked: the digest check off fails BIN-10, the last
  good build off fails BIN-12.
- Code review: no findings; the `low` review read only the non-test files (the diff is all tests), so the test files
  were reread by hand: nothing found.
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `tests/launcher_cli.rs` proves BIN-15 and BIN-16 through `check` and `update` with a fake installed
  `harness` that knows only `run` (and a `PATH` without any `harness` for the look-at-nothing case);
  `src/tests/launcher.rs` dropped.
- red: none (the behaviour exists: the proofs move); mutation checked: `launcher::refused` returning nothing fails
  the 3 proofs that expect a refusal.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3
- done: `tests/retention_cli.rs` proves SYN-28 through `init`, `sync`, `update` and `check`: each user-settings
  value, `CLAUDE_CONFIG_DIR` else `HOME` (unset or empty), unparsable, unlocatable, the project files, `CI`;
  `src/tests/retention.rs` dropped (`user_settings` is proven through the environment of the binary).
- red: none (the behaviour exists: the proofs move); mutation checked: an empty `CLAUDE_CONFIG_DIR` or `HOME` read as
  set fails 2 proofs.
- Code review: 1 findings; the removal of `config/settings.json` assumes it exists: it does, `Home::new` writes
  it, and the test passes.
- not verified: none
- decisions needed: none

## Deliverable 4 (fixes of the `high` review)
- done: 8 of the 10 findings fixed: the test home's `PATH` is its fakes plus links to `git`, `cat`, `cp`, `mkdir`,
  `chmod`, `pwd` (a removed fake is a missing program, whatever the caller installed; `path_without_harness` gone);
  BIN-11 also runs with no `cargo`; the fake `cargo install` no longer sets the executable bit, so BIN-3 and BIN-5
  prove the launcher does; `TOOL` prints its path, so BIN-6 asserts the fresh build ran, BIN-12 the cached one, and
  BIN-1 that the nearest project wins over `CLAUDE_PROJECT_DIR`; the `CI` proof asserts exit 0; the paths in the
  fakes are quoted; the BIN-2 test reuses `git_source`.
- not fixed: (6) `retention_cli.rs` is Unix-only: the test home's fakes are `sh` scripts and Windows is a non-goal of
  the hooks (`specs/tool-binaries.md`); (9) `target()` and `sha256` are not taken from the crate on purpose: a
  boundary test states the asset name and digest the spec promises, not the code's; the third copy of the git-source
  fixture (`launcher_cli.rs`, tag and two commits) has another shape and stays.
- red: none (the fixes harden the proofs); mutations checked: no `set_executable` in `Staging::install` fails BIN-3,
  BIN-5, BIN-13; the fallback before the nearest project fails BIN-1; running the last good build after a good build
  fails BIN-6.
- Code review: 1 findings; the fake `cargo install` no longer sets the executable bit: intended, the launcher
  must set it (BIN-3, BIN-5).
- Status: every deliverable done; closing left to the user.
- not verified: none
- decisions needed: none
