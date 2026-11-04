# Rename the binary to cardinal

# Goal
Rename the crate and the binary `harness` to `cardinal` (`cardinal sync`, `cardinal run <tool>`), with the hooks,
CI, release assets and cache following, without breaking a consumer that still has the old binary installed.

# Findings
* `harness run` appears in 174 lines of 71 files; `harness sync|check|init|update` in 67 lines of 32 files.
* The managed hooks of `entries/.claude/settings.json` call `harness run review-gate`, `format-rust`, `clippy-gate`,
  `language-hint`; the git hook of `crates/harness/src/gate.rs` calls `harness help run` and `harness run`.
* `release.yml` publishes `harness-<commit>` releases with `harness-<target>` assets; `run.rs` caches binaries under
  `<cache>/harness/bin/<commit>/<target>/harness`; `install_harness.sh` installs `crates/harness`.
* `justfile` recipes run `cargo run -p harness` and `cargo install --path crates/harness`.

# Approach
1. `git mv crates/harness crates/cardinal` alone in a first deliverable (renames only).
2. Rename the package and binary, the cache directory, the release and asset names, the install script and the
   workflow names.
3. Transition for the consumers: candidates (to decide) are a `harness` alias binary installed alongside for a
   while, the hooks calling `cardinal` only after `sync` checked it is on the path (as BIN-16 does for a new hook), or
   the launcher accepting both release names.

# Specs
* `specs/tool-binaries.md` (BIN-n naming the binary, its releases and cache) and every spec citing `harness run`;
  amended at `issue-workflow` step 1.

# Resources
* `crates/harness/src/run.rs`, `launcher.rs`, `gate.rs`, `entries.rs`, `entries/.claude/settings.json`
* `.github/install_harness.sh`, `.github/workflows/release.yml`, `harness.yml`, `ci.yml`, `justfile`
* ADR 0010 (the tools ship in the harness binary)

# Hints
* After `name_the_product_cardinal`.
* A move that also edits counts twice in the size limit: move first, edit after.
* Releases already published under `harness-<commit>` stay the ones old pinned revisions download.

# Tracking

Plan: validated

## Understanding
The crate and binary become `cardinal`; its cache, releases and install script follow. A `harness` binary installed
with it (BIN-18) and a launcher that still reads the `harness-<commit>` releases (BIN-3) keep the consumers running
through the switch. Out of scope: `harness.toml`, `harness.just`, `harness.yml` and the harness-named tools (their
own issues).

## Plan
1. [x] Move the crate: `git mv crates/harness crates/cardinal` and `git mv` of both `install_harness.sh` to
   `install_cardinal.sh`, with only the paths that name them (workspace member, `justfile`, `harness.yml`, the install
   line of `launcher.rs`, docs); verified by `just check` and `just check-harness`.
   - done: crate and install script moved (77 renames, 28 lines edited), copies synced; the BIN and SYN-23 spec
     drafts ride along.
   - check | result: tests, lint, specs pass (`just check`); `just check-harness` passes; docs: paths only.
   - red: none (a move, no behaviour).
   - Code review: no findings.
   - not verified: the CI jobs that run `install_cardinal.sh` (after the push).
   - decisions needed: none.
2. [x] Rename the package and binary: package `cardinal` with the bins `cardinal` and `harness`, cache
   `<cache>/cardinal/bin/<commit>/<target>/cardinal`, release `cardinal-<commit>` then `harness-<commit>`, a cargo
   build naming no crate, the local build of `cardinal`, the messages; verified by `run_cli.rs`, `launcher_cli.rs`;
   makes true BIN-1 to BIN-6, BIN-11, BIN-12, BIN-17, BIN-18.
   - done: package `cardinal` with the bins `cardinal` and `harness` (`src/bin/harness.rs` runs the same `main`),
     cache and clone under `<cache>/cardinal/`, releases `cardinal-<commit>` then `harness-<commit>`, `cargo install`
     naming no crate, local build `-p cardinal`, hints naming `cardinal`; `just install` takes `--force`.
   - check | result: tests, lint, specs pass (`just check`); docs: architecture, README, CLAUDE.md, justfile.
   - red: BIN-3: `a_commit_from_before_the_rename_downloads_the_harness_release` panicked, `NotFound` (no binary
     cached under `cardinal/bin`); the other `run_cli` tests failed on the cache path and the release names.
   - Code review: no findings.
   - not verified: a real `cargo install --git` of a pre-rename commit naming no crate (faked in the tests).
   - decisions needed: none. The lock header still says `harness sync` (consumer files issue).
3. [x] Hook transition: the git gate runs `cardinal`, else `harness`; the review gate is found under both commands;
   `check` and `update` judge the `cardinal` hooks and print the install with `--force`; verified by `gate_cli.rs`,
   `launcher_cli.rs`, `update_cli.rs`; makes true BIN-9, BIN-15, BIN-16, SYN-23.
   - done: the gate script runs `cardinal`, else `harness` (its mark line kept, so the gates of before stay ours);
     the gate is managed under both review commands; `check` and `update` judge the `cardinal` and `harness` hooks, a
     missing `cardinal` refusing when `harness` is there; install lines take `--force`; the gate of this repository
     reinstalled by `just sync`.
   - check | result: tests, lint, specs pass (`just check`); `just check-harness` passes; docs: architecture, usage.
   - red: BIN-9, BIN-15, BIN-16, SYN-23: `gate_cli` 18 failed, the gate not installed for a `cardinal run
     review-gate` hook (`sync_installs_an_executable_hook...` panicked at `gate_cli.rs:82`).
   - Code review: no findings.
   - not verified: none.
   - decisions needed: none.
4. [x] Release and CI: `release.yml` publishes `cardinal-<commit>` and the legacy `harness-<commit>`,
   `install_cardinal.sh` fetches and caches `cardinal`; verified by `just check`, the shell syntax (`sh -n`) and the
   CI run after the push.
   - done: `release.yml` builds `cardinal` and publishes it as `cardinal-<commit>` and `harness-<commit>`;
     `install_cardinal.sh` fetches `cardinal-<commit>`, else `harness-<commit>`, into `<cache>/cardinal/bin`, links
     `harness` to it, and builds naming no crate; `harness.yml` checks with `cardinal`; copies synced.
   - check | result: tests, lint, specs pass (`just check`); `sh -n` passes; smoke run of the script with a fake
     `curl` serving only the `harness` asset: installed `cardinal` and the `harness` link; docs: architecture.
   - red: none (CI files, no test; the release workflow is a non-goal of BIN).
   - Code review: 2 findings. Fixed: the build fallback moves `cardinal`, else `harness` (a pre-rename commit
     with no release). Not fixed: `cargo install` naming no crate needs a single package with binaries, which this
     workspace has and BIN-5 states.
   - not verified: `release.yml` and `harness.yml` on GitHub (after the push).
   - decisions needed: none.
5. [x] Switch the commands: the managed hooks, `files/harness.just`, the skills, `AGENTS.md`, the specs, the docs and
   the `justfile` call `cardinal`; `just install` then `just sync`; follow-up issue to drop the alias; verified by
   `just check`, `just check-harness`; makes true BIN-8; sets `tool-binaries` and `sync` `accepted`.
   - done: hooks, recipes, skills, specs, docs, usages and seed call `cardinal`; `session-metrics` names a refusal
     by `cardinal run` or `harness run` (EVA-5 amended); the BIN-8 test refuses `harness run` in what the source
     ships; the lock header says `cardinal sync`; copies synced; follow-up issue `drop_the_harness_alias` queued
     after the rename. The ADRs keep the names of their time (ADR 0012).
   - check | result: tests, lint, specs pass (`just check`); `just check-harness` passes; docs: README, CLAUDE.md,
     architecture, usage, workflow, update-docs-map.
   - red: EVA-5: `session_metrics` panicked at `session_metrics.rs:721`, "a check failed" (a `cardinal run`
     refusal not named after its tool); BIN-8: `settings_hygiene` failed, hooks `cardinal run ...` "point to a
     missing script" before the sync.
   - Code review: no findings.
   - not verified: a consumer synced to this revision (its hooks need `cardinal` installed).
   - decisions needed: none.
6. [x] Fixes of the `high` review (`428ec4a..c677a0f`, 9 findings): verified by `tools_cli.rs`, `just check`.
   - done: `install_cardinal.sh` installs with `--force` (local source, non-commit rev); the tool list says
     `cardinal:`; doc comments of `check`, `update` and the CLI help name the launcher; issue
     `allow_cardinal_run_in_the_synced_projects` queued second; cache cleanup noted in `drop_the_harness_alias`.
   - check | result: tests, lint, specs pass (`just check`); `sh -n` passes; docs: doc comments.
   - red: BIN-7: `no_tool_or_an_unknown_one_exits_3_listing_the_tools` panicked, stderr began `harness: name a
     tool`.
   - Code review: no findings.
   - High review: 9 findings. Fixed: 4 (`--force` in the CI install, `harness:` messages, doc comments, cache
     cleanup noted for the alias issue). Issue: 1 (the seeded allow rule, `allow_cardinal_run_in_the_synced_projects`).
     Not fixed: 4, as decided at the stop (1 a) or out of reach: hooks with no `harness` fallback (an older `update`
     writes them unchecked, a BIN non-goal; a missing command never blocks the agent); an old launcher in the gate
     fallback cannot build a post-rename commit off GitHub (BIN-9 warning, the gate is skipped); a `harness` hook
     with only `cardinal` installed is not refused (every install puts both, BIN-18); the alias bin includes
     `main.rs` by `#[path]` (it declares no module; dropped with the alias).
   - not verified: none.
   - decisions needed: none.

## Decisions
- Mode: autonomous
- Base commit: 428ec4ae5ddbb574760d8ef156a45e48c21b6437
- Transition: the `harness` alias bin, the launcher reading `cardinal-<commit>` then `harness-<commit>`, `check` and
  `update` judging the `cardinal` hooks (BIN-15, BIN-16), `--force` in the install lines.
- `release.yml` also publishes each new commit as `harness-<commit>` / `harness-<target>` until the alias is dropped,
  so a launcher installed before the rename keeps fetching new revisions.
