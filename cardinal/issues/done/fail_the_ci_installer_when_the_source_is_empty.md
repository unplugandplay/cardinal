# Fail the CI installer when the source is empty

# Goal
`files/.github/install_cardinal.sh` now fails when `.claude/cardinal.toml` is missing, but a configuration it reads
no `source` from still takes the `*)` local-path branch, prints `source '' is a local path outside this repository:
cardinal not installed` and exits 0: the later jobs run without `cardinal` and the real cause is never named. Make an
empty source fail, naming the key and the file.

# Findings
* Found by the `high` review of `fail_the_ci_installer_when_the_configuration_is_missing` (2026-10-29), three ways to
  an empty `source` past the `[ -f "$config" ]` guard:
  * no `source` key in the configuration;
  * a TOML literal string, `source = '...'`: `value()` only matches double-quoted basic strings;
  * an unreadable configuration: `sed` fails, but `value()` returns the status of `head`, so `set -e` does not stop.
* The `*)` branch exits 0 on purpose for a real local path outside the repository; only the empty source is wrong.

# Approach
1. Guard right after `source=`: `[ -n "$source" ] || { echo "no source in $config: cardinal not installed" >&2; exit 1; }`
   (covers the three cases; `[ -r ]` alone would cover only the last).
2. Read single-quoted literal strings in `value()` too, so a valid `source = '...'` installs.
3. Tests in `crates/cardinal/tests/install_script.rs` (no `source` key; a single-quoted source), then `just sync`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* `issues/done/fail_the_ci_installer_when_the_configuration_is_missing.md`
* ADR 0010 (the prebuilt binary of the CI)

# Tracking

Plan: validated

## Understanding
`install_cardinal.sh` must exit 1, naming `source` and `.claude/cardinal.toml`, when it reads no source from the
configuration (no key, unreadable file), and must read a single-quoted literal `source = '...'`. Out of scope: the
`*)` branch for a real local path outside the repository (exit 0 on purpose), any other TOML form.

## Plan
1. [x] Guard the empty source and read literal strings in `value()` of `files/.github/install_cardinal.sh`, then
   `just sync`; verified by tests of `crates/cardinal/tests/install_script.rs` (no `source` key: status 1, stderr
   names the key and the file, `cargo` never called; a single-quoted local source installs).
2. [x] Fixes of the `high` review: commit the lock line `just sync` rewrote, read an indented key or one with tabs
   around `=`, word the error `no source read from` (an unparsed form is not a missing key), pin it in the test;
   verified by `crates/cardinal/tests/install_script.rs` and `just check-cardinal`.

## Decisions
- Mode: autonomous
- Base commit: 522f9650c5d1595323d0d420ac23ec48df83e838
- Spec: none (an error path of the CI installer, as its predecessor; only SYN-50 covers the installer, for its URL)

## Summary of deliverable 1
- done: `install_cardinal.sh` exits 1 with `no source in .claude/cardinal.toml: cardinal not installed` on stderr
  when it reads no source, and `value()` reads single-quoted literal strings; copy synced.

| check | result |
|---|---|
| tests | `just check` green |
| docs | n/a: no document describes the installer's error paths |
| code review | `low` run |

- red: none traced (no spec): no key, `left: Some(0) right: Some(1)`; literal source, `left: "" right: "install --quiet --force --git ssh://host/x.git --rev main --locked\n"`, both with stdout `source '' is a local path outside this repository: cardinal not installed`
- Code review: no findings
- not verified: a real GitHub Actions run; the unreadable configuration (covered by the same guard, not tested: a test run as root reads any file)
- decisions needed: none

## High review of the issue
- Code review: 7 findings
  - 1 (lock line of `.github/install_cardinal.sh` left out of 7c6f144): fixed in deliverable 2.
  - 2 (indented key, tab before `=` now fail): fixed in deliverable 2.
  - 3 (keys of any table read), 5 (sed instead of a TOML parser): filed as
    `issues/done/read_the_ci_installer_configuration_with_a_toml_parser.md`.
  - 4 (one message for three causes): wording made true for all three (`no source read from`); `'''` strings go to
    the new issue.
  - 6 (no test of a literal `rev` holding a full hash): not fixed, the release path needs a fake release, and the
    same `-e` reads `rev` and `source`, which the literal test covers.
  - 7 (the test pinned only `source`): fixed in deliverable 2.

## Summary of deliverable 2
- done: lock committed, indented keys read, error worded `no source read from .claude/cardinal.toml`, test pins it;
  follow-up issue filed and queued.

| check | result |
|---|---|
| tests | `just check` green |
| docs | n/a |
| code review | `low` run |

- red: none traced (no spec): indented key, `left: Some(1) right: Some(0)` with stderr `no source in .claude/cardinal.toml: cardinal not installed`; message test failed on the old wording
- Code review: no findings
- not verified: a real GitHub Actions run
- decisions needed: none
