# Share the hash stamp check of the shared modules

# Goal
A script that includes a shared module by `#[path]` (ADR 0006) carries a `// <module> sha256: <hex>` line, so that
`rust-script` rebuilds it when the module changes. Each behaviour test of such a script copies the same check of
that line, and adds the `sha2` dependency. A change of the stamp rule must then be made in every copy, and a missed
copy keeps passing on a stale format. One check should cover every `#[path]` include of `.claude/`.

# Findings
* `/code-review high` of the issue `list_the_repository_files_through_git` (2026-10-27): the check is copied in
  six tests, `test_delivered_quality.rs`, `test_fixed_context.rs`, `test_rule_breaches.rs`,
  `test_session_metrics.rs` (module `transcripts.rs`), `test_close_issue.rs` and `test_check_specs.rs` (module
  `repo_files.rs`).

# Approach
1. One managed test (for example in `.claude/test_*.rs`, run by `just test-harness`) that finds every
   `#[path = "…"] mod` under `.claude/`, and checks the stamp line of the module it names; the per-script copies
   are dropped.
2. Check first how `ship_the_harness_scripts_as_prebuilt_binaries` changes the build of the scripts: if the
   scripts stop being built by `rust-script`, the stamp itself may go.

# Resources
* `docs/adr/0006-transcript-readers-are-managed-scripts.md`, `docs/adr/0009-scanners-list-the-repository-files-through-git.md`.
* `issues/done/ship_the_harness_scripts_as_prebuilt_binaries.md`.

# Tracking

Closed without work (2026-10-27, `skill:prioritize-issues`): `ship_the_harness_scripts_as_prebuilt_binaries` moved
the scripts into the `harness` binary, so no `#[path]` include, `// <module> sha256:` stamp or `rust-script` build
is left to check. Left: the doc comment of `crates/harness/src/tools/repo_files.rs` still cites a `#[path]` include.
