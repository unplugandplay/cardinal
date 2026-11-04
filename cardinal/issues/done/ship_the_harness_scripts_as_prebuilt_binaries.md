# Ship the harness scripts as prebuilt binaries

# Goal
Every hook, check and transcript reader runs through `rust-script`, so a consumer project needs a Rust toolchain,
`rust-script` and a first compile of each script, even when the project is not written in Rust. Ship them as prebuilt
binaries downloaded by `harness`, so the only prerequisites are the `harness` binary and `just`.

# Findings
* `files/harness.just` and the hook declarations of `seed/.claude/settings.json` call `rust-script` on about fifteen
  scripts (hooks, `check_specs.rs`, `check_issue.rs`, `commit_check.rs`, `close_issue.rs`, `next_issue.rs`,
  `issue_worktree.rs`, the transcript readers).
* The transcript readers carry the SHA-256 of `transcripts.rs` because `rust-script` would otherwise keep running a
  stale build (ADR 0006): a cost of the script model.
* The CI already installs `harness` with `cargo install --git --rev <hash>`.

# Approach
1. Measure: the cold and warm start time of a hook under `rust-script` against a compiled binary, and what a
   non-Rust consumer must install today.
2. Fold the scripts into subcommands of `harness` (one binary, versioned by the pinned `rev`), or into a second
   binary; release builds per platform attached to a tag, fetched by `harness sync` into a cache.
3. Keep `rust-script` as a fallback for the repository itself, where the scripts are developed.

# Resources
* `files/harness.just`, `seed/.claude/settings.json`, `files/.claude/hooks/`, `files/.claude/transcripts/`
* `docs/adr/0001-shared-harness-synced-by-a-rust-tool.md`, `docs/adr/0006-transcript-readers-are-managed-scripts.md`

# Hints
* Depends on `propagate_new_hook_declarations_on_sync`: the hook commands change in every project's settings.
* The pinned `rev` must select the binary too, or a project runs hooks of another revision than its files.

# Specs
* [`specs/tool-binaries.md`](../specs/tool-binaries.md) (BIN); the ported tools keep their specs (REV, CLG, PAR, CLR,
  EVA, RER, QUA, REP, BRE), whose traces move to cargo tests.

# Tracking

Plan: validated

## Understanding
The scripts become modules of the `harness` crate, run as `harness run <tool>`, which executes the binary of the pinned
commit from a cache, fetched from a release (SHA-256 checked) or built with `cargo` when none exists. Measured: start
time is not the cost (warm 3-6 ms vs 1 ms, cold 2-3.5 s once per script, 0.4 s per new worktree); the prerequisites are.
Means differ from the issue: no `rust-script` fallback, this repository builds its own crate (`source = "."`, BIN-6).

## Plan
1. [x] Launcher: `harness run` (registry, local build, cache, curl fetch with digest, atomic install, cargo fallback),
   prefetch in `sync`/`update`, first tool ported (`commit-check`, `check-commits`); verified by cargo tests with fake
   `curl`/`cargo` on `PATH`; makes true BIN-1 to BIN-7, BIN-10, BIN-11.
2. [x] Release workflow: build `harness` for each platform on each push to the main branch, publish the asset and its
   SHA-256 in the release of the commit; the CI of `files/` installs `harness` from it; verified by its first run
   after a push (not testable offline).
3. [x] Hooks: `require_code_review`, `clippy_gate`, `format_rust` ported with their tests (REV, CLG traces), entries
   commands and the `pre-commit` gate on `harness run`; makes true BIN-9 and part of BIN-8.
4. [x] Issue workflow: `next_issue`, `issue_worktree`, `close_issue`, `repo_files` ported with their tests (PAR).
5. [x] Checks: `check_specs`, `check_issue` ported with their tests (CLR); this repository's `check-specs` keeps working.
6. [x] Transcript readers and `audit_sources` ported with their tests (EVA, RER, QUA, REP, BRE); stamps removed;
   ADR 0006 marked superseded.
7. [x] No `rust-script` left: recipes, skills, seed allow rule, CI, `test-harness` (the content tests `test_skills`,
   `test_recipes`, `test_settings` become cargo tests of this repository), README and architecture; a test fails on
   any `rust-script` in `files/`, `entries/`, `seed/`; makes true BIN-8, spec `accepted`.
8. [x] Fixes of the `high` review of the issue: the launcher exits 3 (no block) when it cannot run a tool, finds the
   project from `CLAUDE_PROJECT_DIR` when the working directory has none, resolves a non-hash `rev` in the cached
   clone without fetching, builds a scp-style source as `ssh://`; Linux assets are `musl`; the gate recognizes and
   runs the review hook of an older revision too; `install_harness.sh` installs into the cache and reads a missing
   digest as no asset; the CI steps are guarded; the ported tests use `tempfile`; verified by tests (BIN-1, BIN-12,
   BIN-14, SYN-18, SYN-22) and an offline run of the install script.

## Decisions
- Mode: autonomous
- Base commit: 95113afff8717c5944eee5964c10c04acdbddbc0
- The tools are subcommands of `harness` (one artifact per commit), not a second binary.
- A binary is published for every push to the main branch, in a release named after the commit.
- Platforms: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`. Changed by deliverable
  8 to the `musl` targets on Linux (same platforms): a `gnu` binary built on a recent runner does not run on an older
  glibc, and a cached binary is never fetched again.
- The script tests become `cargo test` of this repository; consumers lose `just test-harness`.
- Release asset naming: release `harness-<commit>`, assets `harness-<target>` and `harness-<target>.sha256` (raw
  binary, no archive: nothing to unpack). A source not on GitHub is built with `cargo install`.
- Spec amended while designing the launcher: BIN-5 builds with `cargo install --git`, BIN-12 (a local source that
  does not compile runs its last good build, else the hooks would all block), BIN-13 (a failed prefetch of
  `sync`/`update` is a warning).
- This machine's `harness` (`~/.cargo/bin`) predates `run`: `just check-commit` needs `just install` once the
  deliverable is merged (before that, `cargo run -q -p harness -- run commit-check`).

## Deliverable 1
- done: `harness run`/`harness tool`, launcher with cache, download + digest, `cargo install` fallback, local build
  with last good copy, prefetch in `sync`/`update`; `commit-check` and `check-commits` ported, their scripts removed,
  recipes, skill and CI job switched.

| check | result |
|---|---|
| tests (`just check`) | pass |
| lint | pass |
| specs | pass (BIN draft, BIN-1 to 7, 10 to 13 traced) |
| `just test-harness` | pass |
| docs | README, architecture (Tools and their binary), CLAUDE.md, update-docs map, ADR 0010 accepted |

- red: none obtained before the code (the code was written first); the first run of the launcher tests failed on the
  fixture (fake tool exit argument, then `ETXTBSY` on a script written in-process), not on the code.
- Code review: no findings (its one doubt, a prefetch under `update --dry-run`, cannot happen: `applied` is false then).
- not verified: a real download from GitHub (no release yet, deliverable 2); the CI commits job.
- decisions needed: none.

## Deliverable 2
- done: `.github/workflows/release.yml` (3 targets, release `harness-<commit>` per push to main); managed
  `.github/install_harness.sh` used by both jobs of `harness.yml` (asset into the cache of `harness run` and on the
  path, digest mismatch fails, otherwise `cargo install`; a local source of this repository installs its crate).

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness` | pass |
| install script | `sh -n`, and run offline with a fake `curl`/`cargo`: asset installed, mismatch exits 1, no asset builds |
| workflows | parsed as YAML; not run |
| docs | architecture (Tools and their binary, Sources, what is shared) |

- red: none (no test for CI glue, as planned).
- Code review: 2 findings, both fixed: a re-run of the release workflow failed on the existing release (now
  uploads with `--clobber`); a `rev` that is not a full hash fell back to `cargo install` silently (now says so).
- not verified: the release workflow and the CI jobs on GitHub: they run on the first push.
- decisions needed: none.

## Deliverable 3
- done: `review-gate`, `clippy-gate`, `format-rust` ported (code moved as is, entry points return the exit code),
  their tests turned into cargo tests (`tests/review_gate.rs`, `tests/rust_hooks.rs`, 146 checks, traces kept); the
  managed hook commands and the `pre-commit` gate run `harness run`; the gate is installed when the source manages a
  hook running `harness run review-gate` (`check`: when the lock records one) instead of when the hook file is managed;
  SYN-18, 22, 23, 24, REV intro and REV-9, CLG intro and PAR-12 reworded; a test checks every `harness run <tool>` of
  `files/` and `entries/` names a tool (BIN-8, part).

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness` | pass |
| docs | architecture (Tools, Hooks and their declaration, gate, parallel issues), issue-workflow skill and details |

- red: none before the code (the hook tests were ported with the code, the gate tests were rewritten with it).
- Code review: no findings (run twice: the first did not see the new, untracked files).
- not verified: the hooks run by Claude Code through `harness run` (this session runs the declarations of the main
  worktree, still `rust-script`).
- decisions needed: none.
- Follow-ups:
  - Merge: `sync` installs the new gate in the common git directory, shared by every worktree, and the gate needs a
    `harness` with `run`. After the merge, run `just install` then `just sync` in the main worktree, in that order.
    Until then the gate of the main branch must stay: this session restored it by hand after each `just sync`.
  - Deliverable 6: `session_metrics.rs` names a hook refusal by the `.rs` file its text cites; a refusal of
    `harness run review-gate` cites none. Map the tool names (`review-gate` to `require_code_review`, `clippy-gate`
    to `clippy_gate`) so the counts stay comparable across revisions.

## Deliverable 4
- done: `next-issue`, `issue-worktree`, `close-issue` and the `repo_files` module ported (code moved as is, usage
  lines name `harness run`), their tests turned into cargo tests (`tests/next_issue.rs`, `close_issue.rs`,
  `issue_worktree.rs`, 76 checks, PAR traces kept; the hash stamp check of `repo_files.rs` dropped, the crate needs
  none); the recipes call `harness run`. `files/.claude/repo_files.rs` stays for `check_specs.rs` until deliverable 5.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness` | pass |
| smoke | `cargo run -- run next-issue` in this worktree prints the queue |
| docs | architecture (Tools, Parallel issues) |

- red: none before the code (tests ported with the code).
- Code review: no findings; it saw part of the moved modules only (73 kB diff), so each was diffed against its original through rustfmt: only the entry point and usage lines differ.
- not verified: `just start-issue`, `finish-issue`, `close-issue` through an installed `harness` (this machine's
  predates `run`).
- decisions needed: none.
- Follow-up: the merge itself needs the new `harness` first: `just close-issue` and `just finish-issue` of the main
  branch call `harness run` once fast-forwarded, so run `just install` before them.

## Deliverable 5
- done: `check-specs` and `check-issue` ported with their tests (`tests/check_specs.rs`, `check_issue.rs`, 87 checks,
  CLR and PAR traces kept, the hash stamp check dropped); the last copy of `repo_files.rs` under `files/` removed;
  recipes, `write-spec`, `create-issue` and `specs/README.md` name `harness run`. This repository's `justfile` runs
  `check-specs` with `cargo run`, so `just check` (and CI, which installs no `harness` here) uses the checker of the
  working tree; `just check` no longer needs `rust-script`.

| check | result |
|---|---|
| tests, lint, specs (`just check`, through the ported checker) | pass |
| `just test-harness` | pass |
| docs | architecture, CLAUDE.md, `specs/README.md`, write-spec and create-issue skills |

- red: none before the code (tests ported with the code).
- Code review: no findings (its one doubt, a duplicate `check-specs` recipe, is allowed by `set allow-duplicate-recipes`; `just check` ran through it).
- not verified: none beyond the earlier ones.
- decisions needed: none.

## Deliverable 6
- done: `session-metrics`, `rule-breaches`, `delivered-quality`, `fixed-context`, `harness-report`, `audit-sources`
  and the `transcripts` module ported (`chrono` joins the crate), their tests cargo tests (125 checks); the hash
  stamps and their checks gone: EVA-13 and BRE-12 withdrawn, the stamp clause of QUA-10 and REP-8 dropped, EVA-12
  checked on the sources of the crate; ADR 0006 superseded by 0010; EVA-5 names a refusal of `harness run <tool>` by
  the script it was (`review-gate` as `require_code_review`), so the counts compare across revisions; the skills
  `evaluate-harness`, `audit-instructions`, `issue-workflow` and `test_skills.rs` name the tools.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness` | pass |
| smoke | `run session-metrics` reads 246 sessions of this machine, `run rule-breaches --summary` prints its table |
| docs | architecture (instruction audit, evaluation), README, ADR 0006, specs EVA, BRE, QUA, REP |

- red: `hook refusals are counted by hook` failed on the two new `harness run` refusals before `hook_name` knew them.
- Code review: no findings (run twice: the first did not see the new files, after a refused `git add -N`).
- not verified: none beyond the earlier ones.
- decisions needed: none.

## Deliverable 7
- done: `test_skills.rs` and `test_settings.rs` became cargo tests of this repository (`tests/skills.rs`,
  `tests/settings_hygiene.rs`; the allowlist must now cover `harness run`, the declaration check of `.claude/hooks/`
  scripts went with the scripts); `test_recipes.rs` and the `test-harness` recipe removed (consumers run no harness
  test); the seeded allowlist (and this repository's) allows `Bash(harness run *)` instead of `rust-script`; the shared
  CI installs `just` and `harness` only; `issue-workflow` no longer asks for `just test-harness` nor `rust-script`; a
  test fails when anything under `files/`, `entries/` or `seed/` names `rust-script` (BIN-8); spec BIN accepted.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `git grep rust-script -- files entries seed` | nothing |
| docs | architecture (Tools, cost table), README, CLAUDE.md, clippy-gate non-goals |

- red: not observed: the BIN-8 scan was written after the last `rust-script` left `files/`.
- Code review: no findings.
- not verified: the shared CI on GitHub (first push).
- decisions needed: none.

## Deliverable 8
- done: the 10 findings of the `high` review of `95113af..1948b5b`:
  1. `harness run` exits 3 when it cannot get the tool (BIN-14, BIN-12 now 3): a hook that cannot run no longer
     blocks the agent, and the gate lets the commit through with a warning on 3;
  2. a `rev` that is no hash resolves in the cached clone, with no fetch per hook;
  3. the launcher falls back to `CLAUDE_PROJECT_DIR` when the working directory is outside any project (BIN-1);
  4. a `git@host:path` source is built from `ssh://git@host/path` (launcher and CI script);
  5. Linux assets are `musl`, static (decision amended);
  6. `install_harness.sh` builds into the cache `harness run` reads, so a job builds once;
  7. a missing `.sha256` reads as no asset (CI script), as in the launcher; a source off GitHub ending in `.git` was
     taken for GitHub (found by the offline run), fixed;
  8. the gate recognizes and runs the review hook script of an older revision (SYN-18, 22, 23), and skips with a
     warning a `harness` without `run`: a new binary keeps gating a project pinned before this issue, and the shared
     gate no longer needs restoring by hand in this repository;
  9. the CI steps that need `harness` are skipped like `harness check` when it is not installed;
  10. the ported tests take a `tempfile::tempdir()` each.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| install script, offline (fake `curl`, `cargo`) | asset, bad digest (exit 1), no asset, no digest, scp source off GitHub: as expected |
| workflows | parsed as YAML |
| docs | architecture (Tools, gate), specs BIN (back to accepted) and SYN |

- red: the launcher tests did not compile against the old `command` signature; the three new gate tests failed
  (legacy script not run, exit 3 not let through, legacy source not gated).
- Code review: 1 finding, fixed: a name the cached clone did not know (pushed after it, a short hash) failed every hook; the launcher now fetches once then. A branch moved since the last `sync` still resolves to the commit of that sync, on purpose: the revision of the synced files.
- not verified: the `musl` builds and the CI on GitHub (first push).
- decisions needed: none.
- Merge, updated: the gate `sync` installs now works with the main branch as it is, so no hand restore; `just install`
  is still needed once merged, before `just close-issue` and `just finish-issue`, which call `harness run`.
