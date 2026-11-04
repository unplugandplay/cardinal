# Drop the old names of the consumer files

# Goal
Once every consumer has migrated to `.claude/cardinal.toml`, `.claude/cardinal.lock` and `cardinal.just`, stop
reading and writing `.claude/harness.toml` (its `[harness]` table), `.claude/harness.lock` and `harness.just`, so the
tool loses the fallback and the migration code it keeps for them.

# Findings
* The old names live in `crates/cardinal/src/lib.rs` (`OLD_CONFIG`, `OLD_LOCK`, `Names` and its `OLD`, `Migration`,
  `table_renamed`, `imports_renamed`), `run.rs` (`project_root`), `gate.rs` (the `pre-commit` script),
  `files/.github/install_cardinal.sh` and `tools/transcripts.rs` (`CONFIGS`, `LOCKS`).
* They are written for a source below the format 3 (SYN-44): dropping them means `cardinal` stops reading the
  format 2, which ADR 0014 says it reads with every lower one.

# Approach
1. Check that every consumer is pinned to a revision of the format 3 or later, and migrated.
2. Decide whether this `cardinal` stops syncing sources below the format 3 (a new ADR superseding part of ADR 0014
   and ADR 0015) or keeps writing the old names for them only.
3. Keep the old names in the history readers (`transcripts.rs`): past commits still hold them.
4. Withdraw or amend SYN-44 to SYN-47 and the specs that cite them.

# Specs
* `specs/sync.md` (SYN-44, SYN-45, SYN-46, SYN-47), `specs/tool-binaries.md` (BIN-1, BIN-14).

# Resources
* ADR 0014, ADR 0015, `docs/usage.md` (the migration paragraph)
* `issues/done/rename_the_consumer_files_to_cardinal.md`

# Hints
* After `drop_the_harness_alias`: both drop what a consumer of before the rename still needs.

# Tracking

Plan: validated

## Understanding
Every consumer (cardinal, RetroDoc, Assistant) is pinned to `50e809d` (format 3) and migrated, checked 2026-10-29.
`cardinal` then refuses a source below the format 3 (new SYN-51, ADR 0016) and reads and writes the new names only;
the history readers (`transcripts.rs`, `delivered_quality.rs`) keep both. Out of scope: the `harness` release and
build of an old commit (`install_cardinal.sh`), the gate mark `# harness: native review gate`.

## Plan
1. [x] Test fixtures on the new names: `tests/common` names `.claude/cardinal.toml` and `.claude/cardinal.lock`, the
   fixture sources declare `cardinal-format` 3, the projects the tools and `run` read carry `[cardinal]`; the
   history fixtures (session metrics, delivered quality, rule breaches, fixed context) keep the old names; no change
   of behaviour; verified by `just check`. ~500 lines.
2. [x] Drop the old names: `init`, `sync` and `update` refuse a source below the format 3; `Names`, `OLD_CONFIG`,
   `OLD_LOCK`, `Migration`, `table_renamed`, `imports_renamed` go (the history readers keep the old names as their
   own constants); `run.rs`, the gate script and `install_cardinal.sh` read `.claude/cardinal.toml` only;
   `names_cli.rs` gives way to the SYN-51 tests; ADR 0016 accepted, ADR 0014 and 0015 marked partly superseded;
   `docs/usage.md` and `docs/architecture.md` updated; verified by the SYN-51 tests and `just check`; makes true
   SYN-51 (SYN-44 to SYN-48 withdrawn, SYN-9, SYN-11, SYN-18, SYN-42, BIN-1, LNG-3, TRC-10 amended). ~700 lines.
3. [x] Fixes of the `high` review: a source path that holds no `files/` is reported by SYN-10, not as the format 1;
   `init` names the resolved URL and the commit in its install hint; `readable` returns nothing; ADR 0016 says a
   project of the old names is not detected; the next issue no longer relies on SYN-45; verified by the SYN-10 test of
   `format_cli.rs`; makes true SYN-10, SYN-51. ~80 lines.

## Decisions
- Mode: autonomous
- Base commit: 70d7eb8222dfa3debc84d44774b249a6ae8ea0e0
- Answer 1 a: a source below the format 3 is refused (SYN-51, ADR 0016).

## Deliverable 1
- done: the test fixtures write `.claude/cardinal.toml` (`[cardinal]`) and `.claude/cardinal.lock` and their sources
  declare the format 3 (`common::source`, `SOURCE_FORMAT`, `FORMAT`); the history fixtures and the tests of the
  old names (`names_cli.rs`, the SYN-47 tests of `run_cli.rs` and `gate_cli.rs`) keep them until deliverable 2.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | none (tests only) |
| code review | `low`: no findings |

- red: none (tests only, no change of behaviour)
- Code review: no findings
- not verified: nothing
- decisions needed: none

## Deliverable 2
- done: `init`, `sync` and `update` refuse a source below the format 3; `check`, `run`, the tools, the gate and
  `install_cardinal.sh` read `.claude/cardinal.toml` and `.claude/cardinal.lock` only; the migration is gone; the
  history readers keep the old names (`transcripts::OLD_CONFIG`, `OLD_LOCK`); ADR 0016 accepted, ADR 0014 and 0015
  partly superseded; `names_cli.rs` and the two SYN-47 tests are removed.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (SYN-51 accepted with the spec; SYN-44 to SYN-48 withdrawn) |
| docs | `docs/usage.md`, `docs/architecture.md`, ADR 0014, 0015, 0016 |
| code review | `low`: no findings |

- red: SYN-51: `error: .claude/cardinal.toml needs a source of the format 3 or higher, <source> has the format 2`
  instead of `<source> has the format 2, this cardinal syncs the format 3 and higher`
- Code review: no findings (its two open points hold: `sync` and `update` call `readable` before `sync_from`, `transcripts::LOCKS` compiles)
- not verified: the CI run of `install_cardinal.sh` (a push only)
- decisions needed: none

## Deliverable 3
- done: fixes of the `high` review (9 findings).

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | ADR 0016, `issues/done/report_what_a_format_migration_leaves_to_the_project.md` |
| code review | `high`: 9 findings; `low` on the fixes: no findings |

- red: SYN-10: `error: <source>/gone has the format 1, this cardinal syncs the format 3 and higher` instead of `has no files/ directory`
- Code review: no findings (`low` on the fixes)
- `high` findings: 1 (old-name project not refused) ADR 0016 corrected, no code: detecting it would keep the old
  names; 2 (missing source read as format 1) fixed; 3 (`init` install hint) fixed; 4 (next issue cites SYN-45)
  fixed; 5 (CI installer exits 0 without a configuration) pre-existing, issue
  `fail_the_ci_installer_when_the_configuration_is_missing`; 6 (`readable` result unused) fixed; 7 (`config_of`
  twice) fixed; 8 (`write_lock` doc) fixed; 9 (test name claims an older format) renamed.
- not verified: the install hint of `init` for a git source of a too new format (no test, as for SYN-40 of `init`)
- decisions needed: none
