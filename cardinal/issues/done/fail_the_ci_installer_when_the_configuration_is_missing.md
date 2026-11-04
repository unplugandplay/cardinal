# Fail the CI installer when the configuration is missing

# Goal
`files/.github/install_cardinal.sh` puts on the CI path the `cardinal` the project pins. When the work tree has no
`.claude/cardinal.toml`, it says the source is a local path outside the repository and exits 0: the later jobs run
without `cardinal` and the real cause is never named. Make it fail naming the missing configuration.

# Findings
* Found by the `high` review of `drop_the_old_names_of_the_consumer_files` (2026-10-29): `value` reads the file with
  `sed | head`, whose status is `head`'s, so `set -e` does not stop on a missing file; `source` is empty, the `*)`
  branch prints `source '' is a local path outside this repository: cardinal not installed` and exits 0.
* Not new: before that issue, the fallback to `.claude/harness.toml` reached the same branch when neither existed.

# Approach
1. Guard right after `config=`: `[ -f "$config" ] || { echo "no $config: cardinal not installed" >&2; exit 1; }`.
2. A test of the missing configuration in `crates/cardinal/tests/install_script.rs`, then `just sync`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* ADR 0010 (the prebuilt binary of the CI)

# Tracking

Plan: validated

## Understanding
`install_cardinal.sh` must exit non-zero, naming `.claude/cardinal.toml`, when the work tree lacks it, instead of
reaching the local-path branch and exiting 0. Out of scope: any other check of the configuration's content.

## Plan
1. [x] Guard the missing configuration: `[ -f "$config" ] || …; exit 1` after `config=` in
   `files/.github/install_cardinal.sh`, then `just sync`; verified by a test of `crates/cardinal/tests/install_script.rs`
   (no configuration: status 1, stderr names it, `cargo` never called).

## Decisions
- Mode: autonomous
- Base commit: b0ef0c27c28292662e9356b07d42d6389869f9fd
- Spec: none (a fix of an error path of the CI installer; only SYN-50 covers the installer, for its URL)

## Summary of deliverable 1
- done: `install_cardinal.sh` exits 1 with `no .claude/cardinal.toml: cardinal not installed` on stderr when the
  configuration is missing; copy synced.

| check | result |
|---|---|
| tests | `just check` green |
| docs | n/a: no document describes the installer's missing-configuration path |
| code review | `low` run |

- red: none traced (no spec): `left: Some(0) right: Some(1)`, stdout `source '' is a local path outside this repository: cardinal not installed`
- Code review: no findings
- not verified: a real GitHub Actions run
- decisions needed: none

## High review of the issue
- Code review: 6 findings
  - 1-4 (empty `source` past the guard: no key, single-quoted literal, unreadable file; no test of it): outside this
    issue's requirement, filed as `issues/done/fail_the_ci_installer_when_the_source_is_empty.md`.
  - 5 (the `*)` branch still exits 0 on stdout): not fixed, intended for a real local path outside the repository.
  - 6 (no fix hint in the message): not fixed, the message names the project-owned file to restore.
- Done: no code change left.
