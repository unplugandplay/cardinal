# Prove the sync specs through the binary

# Goal
Move the proofs of `specs/sync.md` for `init`, `sync`, `update`, `check` and the managed settings entries from
internal unit tests to tests that run the `harness` binary on temporary directories, so the spec and its tests
form an acceptance suite that survives a rewrite of the code.

# Findings
* Split from `prove_the_specs_with_black_box_tests` by size (2026-10-27).
* `crates/harness/src/tests.rs` holds 32 `// spec: SYN-n` citations and `crates/harness/src/tests/settings_entries.rs`
  20, all calling internal functions (`use super::*`).
* `crates/harness/tests/` already runs the built binary (`CARGO_BIN_EXE_harness`) in several files.

# Approach
1. Per group of requirements (init and sync, update, check, settings entries), write the boundary test that proves
   each one through the command line, files, exit code and output; move the `// spec:` citation to it.
2. Keep an internal test that adds detail as a unit test without a citation; drop the ones the boundary test covers.

# Specs
* `specs/sync.md`: content unchanged, only its proofs move.

# Resources
* `crates/harness/src/tests.rs`, `crates/harness/src/tests/settings_entries.rs`, `crates/harness/tests/`
* `files/specs/README.md` (the tracing rule)

# Hints
* After `prove_the_specs_with_black_box_tests`: it sets the rule and reports the internal-only proofs this issue
  removes.
* Black-box tests through the binary are slower; keep them on temp dirs with no network, per `CLAUDE.md`.

# Tracking

Plan: validated

## Understanding
Re-prove SYN-1 to SYN-17 and SYN-29 to SYN-36 by running the `harness` binary (`init`, `sync`, `update`, `check`) on
temp dirs, asserting files, exit code, stdout and stderr; the internal tests either lose their citation (kept as unit
detail: `set_rev`, `is_full_hash`) or are dropped. Out of scope: SYN-18 to SYN-28 (install gate, warnings, retention:
`prove_the_run_and_gate_specs_through_the_binary`), except the citation SYN-25, SYN-26, SYN-27 of the settings test
that moves with it. Offline: local or `file://` sources, `XDG_CACHE_HOME` in a temp dir, a fake failing `cargo` first
on the `PATH` so the prefetch of the tools binary (BIN-13) builds nothing.

## Plan
1. [x] Init, sync and check: shared helpers `crates/harness/tests/common/mod.rs` (run the binary, put/get, a source),
   `crates/harness/tests/sync_cli.rs`, the matching tests of `src/tests.rs` dropped; verified by `just check` and
   `check-specs` no longer listing them; makes true SYN-1 to SYN-10 (~650 lines).
2. [x] Update: `crates/harness/tests/update_cli.rs` on a local git source, internal update tests dropped, `set_rev`
   and `is_full_hash` kept uncited; makes true SYN-11 to SYN-17 (~650 lines).
3. [x] Settings entries: `crates/harness/tests/settings_entries_cli.rs`, `src/tests/settings_entries.rs` dropped;
   makes true SYN-29 to SYN-36 (~850 lines).
4. [x] Fixes of the `high` review: the proofs fail when the behaviour breaks, git isolated from the caller.

## Decisions
- Mode: autonomous
- Base commit: 9d11cef45c6773bc0af41451336fd0836f3a47c7

## Deliverable 1
- done: `tests/common/mod.rs` (binary in a temp home: cache, user settings, failing `cargo`), `tests/sync_cli.rs`
  proves SYN-1 to SYN-10 through `init`, `sync`, `check`; their internal tests of `src/tests.rs` dropped.
- red: none (the behaviour exists: the proofs move, no test can fail before the code)
- Code review: 1 findings; the possible dead helpers and imports of `src/tests.rs` after the drop: not a defect,
  `just check` (clippy `-D warnings`) passes, the remaining tests use them (`use super::*` raises no unused warning).
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `tests/update_cli.rs` proves SYN-11 to SYN-17 through `update` and `sync` on a `file://` git source; the
  internal update, resolve and pinned-hash tests dropped; `set_rev` (array lines, a name with a newline) and
  `is_full_hash` kept as uncited unit tests.
- red: none (the behaviour exists: the proofs move, no test can fail before the code)
- Code review: 2 findings; (1) `git_source` and `git_project` possibly dead: no, `src/tests/launcher.rs` calls them
  and clippy passes; (2) `set_rev` and `is_full_hash` tests lost their citation: by plan, they add detail, the
  boundary tests prove SYN-12 and SYN-17.
- not verified: none
- decisions needed: none

## Deliverable 3
- done: `tests/settings_entries_cli.rs` proves SYN-29 to SYN-36 (and carries the SYN-25, SYN-26, SYN-27 citation of
  the repository settings test) through `sync` and `check`; `src/tests/settings_entries.rs` dropped; the test home
  puts a `harness` that knows every subcommand first on the `PATH`, so `check` judges no hook by the installed one.
- red: none (the behaviour exists: the proofs move, no test can fail before the code)
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 4 (fixes of the `high` review)
- done: the 10 findings fixed: the update half of the fetch-by-hash test gets an empty cache; SYN-35 asserts the
  shape warning itself; the outside-path lock line carries the real hash of the file; the refusals of SYN-12 and
  SYN-13 assert their cause, and a dotted key is refused too; `snapshot` records directories and symlinks; git runs
  with the temp home, no system or global config and no `GIT_*` of the caller, for the fixtures and the binary; the
  `PATH` is joined portably and the files run on Unix only (the fakes are shell scripts); the `cat-file` probe no
  longer prints to the test output; the closure `pinned` renamed `rev_line`.
- red: none (the fixes harden the proofs of behaviour that exists)
- Code review: 1 findings; `GIT_CONFIG_PARAMETERS` (the `git -c` options of a calling hook) not removed: fixed.
- not verified: none
- decisions needed: none
