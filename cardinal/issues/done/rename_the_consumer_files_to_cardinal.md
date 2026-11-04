# Rename the consumer files to cardinal

# Goal
Rename the files the tool keeps in each consumer: `.claude/harness.toml` → `.claude/cardinal.toml` (and its
`[harness]` table), `.claude/harness.lock` → `.claude/cardinal.lock`, `harness.just` → `cardinal.just`, with `sync`
migrating a consumer that still has the old names.

# Findings
* `harness.toml` appears in 167 lines of 71 files, `harness.lock` in 14 lines, `harness.just` in 12, the `[harness]`
  table in 36 lines.
* The paths are constants in `crates/cardinal/src/lib.rs` (`CONFIG`, `LOCK`); `run.rs` finds the project by the
  nearest `.claude/harness.toml`; `install_cardinal.sh` and `gate.rs` read it too.
* Each consumer's `justfile` holds `import 'harness.just'`; that file is project-owned, not managed.

# Approach
1. `sync` (and `run`, `check`, `update`) read the old names when the new ones are missing, and `sync` renames them,
   rewriting the `import` line of the `justfile`.
2. Rename the paths in the code, `files/`, `seed/`, the specs, the skills and the docs.
3. Drop the fallback once every consumer has migrated (separate issue, or a dated note).

# Specs
* `specs/sync.md` (a new SYN-n for the migration of the old names) and the specs naming these files; amended at
  `issue-workflow` step 1.

# Resources
* `crates/cardinal/src/lib.rs`, `run.rs`, `gate.rs`, `budget.rs`
* `seed/.claude/harness.toml`, `files/harness.just`, `.github/install_cardinal.sh`
* ADR 0001, ADR 0002 (rev pins the resolved commit)

# Hints
* After `rename_the_binary_to_cardinal`.
* A consumer pinned to an old revision still runs the old binary, which knows only `harness.toml`: the migration
  must not leave a project that the old binary can no longer read before its `update`.

# Tracking

Plan: validated

## Understanding
Rename the consumer files to `cardinal.toml` (`[cardinal]`), `cardinal.lock`, `cardinal.just`, with `sync`/`update`
migrating an old project. Means: the names follow the source format, raised to 3 (ADR 0015): a project pinned to a
format-2 revision keeps the old names its tools read; readers without a source fall back to the old names.

## Plan
1. [x] Names by format and migration: `FORMAT` and `cardinal-format` 3, `init`/`sync`/`update` name and migrate,
   `check`/`run`/tools/gate/`install_cardinal.sh` read either, `git mv` of `files/harness.just` and
   `seed/.claude/harness.toml`, the `import` lines, `just sync` migrating this repository (~800 lines); verified by
   `sync_cli`, `update_cli`, `run_cli`, `gate_cli` tests; makes true SYN-44, SYN-45, SYN-46, SYN-47, BIN-1, SYN-18.
2. [x] History readers read both names at each commit: `transcripts`, `rule_breaches`, `delivered_quality`,
   `fixed_context` (~300 lines); verified by their CLI tests; makes true EVA-8, BRE-7, BRE-9, BRE-10, QUA-4, REP-2.
3. [x] Prose: the new names in `files/` (skills, `AGENTS.md`, `cardinal.just`), `seed/`, `docs/`, `README.md`,
   `CLAUDE.md`, then `just sync`; a follow-up issue to drop the old names with the format 2 (~300 lines); verified by
   `just check`, `just check-harness`, a grep for the old names.
4. [x] Fixes of the `high` review: refuse a source below the format 3 for a migrated project, migrate the
   justfile before moving the configuration, read `Justfile` and `.justfile`, name the old lock in `check`; verified by
   `names_cli` tests; makes true SYN-45, SYN-48.

## Decisions
- Mode: autonomous
- Base commit: c6b95b18aab6955bb847253767653cc75b7fbf3e

## Deliverable 1

- done: the names follow the source format (3 from now on), `sync`/`update` migrate an old project, `check`, `run`,
  the tools, the gate and `install_cardinal.sh` read either name; this repository migrated by its own `just sync`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass; `sync.md` back to `accepted` |
| docs | `docs/usage.md` (migration), ADR 0015 |
| code review | see below |

- red: SYN-44, SYN-45, SYN-46, SYN-47, BIN-1, SYN-18: `needs a cardinal that reads the format 3, this one reads up
  to 2`, `no .claude/harness.toml in . or above`, `review gate skipped`
- Code review: 3 findings
  - `update` writes the configuration before its sync, which then skips the migration: refuted, it writes it after
    (`update_to_a_revision_of_the_format_3_migrates_and_pins_it_in_the_new_configuration`).
  - the migration is applied before the conflicts are known: kept, `sync` writes past a conflict by contract (SYN-3),
    and `update` applies nothing on one (SYN-13).
  - the errors of `set_rev` no longer named the file: fixed, they name the configuration.
- Code review (again, after a commit that held only the `git mv`, amended): 1 finding
  - a migration interrupted after the move of the configuration left `.claude/harness.lock` for good: fixed, a sync
    writing the new lock drops the old one.
- not verified: the migration of a consumer other than this repository; the CI run of `install_cardinal.sh`.
- decisions needed: none

## Deliverable 2

- done: `transcripts` (`committed_file`, `shown`, `pinned_revision`) reads `.claude/cardinal.*` else
  `.claude/harness.*` at each commit; `rule_breaches`, `fixed_context`, `delivered_quality` go through it.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | n/a: no doc describes these readers' file names besides the specs amended at step 1 |
| code review | see below |

- red: EVA-8, QUA-4, BRE-7, BRE-9, BRE-10, REP-2: `FAIL - the new configuration pins after the migration`,
  `FAIL - a later fix of its files counts, docs, lock, side branch and window excluded`, the helpers missing
- Code review: no findings
- not verified: the readers on the history of a real consumer migrated later.
- decisions needed: none

## Deliverable 3

- done: the new names in `files/` (skills, `AGENTS.md`, `cardinal.just`, `specs/README.md`), `seed/`, `docs/`,
  `README.md`, `CLAUDE.md` and the doc comments, synced; follow-up
  `issues/done/drop_the_old_names_of_the_consumer_files.md`, queued after `drop_the_harness_alias`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | this deliverable; `just check-harness` passes |
| code review | see below |

- red: none
- Code review: no findings
- not verified: none
- decisions needed: none
- left: the old names stay where they are meant (the fallback code, `docs/usage.md`, ADR 0015, the specs, the tests
  of the old names).

## Deliverable 4

- done: the fixes of the `high` review of `c6b95b1..3ee89a0` (10 findings, below).

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass; SYN-48 added, `sync.md` back to `accepted` |
| docs | `docs/usage.md` |
| code review | see below |

- red: SYN-48, SYN-45, SYN-47: `Run { code: Some(0), stdout: "updated: a.txt\n" }` (no refusal), no
  `updated: .justfile`, `no .claude/cardinal.lock` for a project of the old names
- High review: 10 findings
  - a migrated project synced to a source of the format 2 broke its tools and recipes: fixed, refused (SYN-48).
  - the justfile was rewritten after the configuration moved, so a failure left it for good: fixed, it goes first.
  - `Justfile` and `.justfile` were not migrated: fixed.
  - `import './harness.just'` and other spellings are not migrated: kept, the seeded justfile writes
    `import 'harness.just'`; another spelling is the project's own.
  - a stray `.claude/cardinal.lock` beside the old configuration is read: kept, the lock is read on its own (SYN-47)
    so that a migration interrupted before its lock still finds it; the stray file arises only by hand.
  - the old lock is dropped beside two configurations: kept, an older `cardinal` pinned through the old
    configuration of a migrated project is not a supported setup.
  - `check` named the new lock for a project of the old names: fixed.
  - the language hint names `.claude/cardinal.toml`: kept, its binary runs only for a project pinned on a revision of
    the format 3, migrated by then.
  - `update` computes the migration besides its two syncs: kept, the reads are of the same files within one command.
  - the helper `budget::name` duplicated `Names::read`: fixed, removed.
- Code review: no findings
- not verified: none
- decisions needed: none
