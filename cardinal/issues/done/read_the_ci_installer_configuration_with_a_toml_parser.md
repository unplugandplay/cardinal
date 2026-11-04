# Read the CI installer configuration with a TOML parser

# Goal
`files/.github/install_cardinal.sh` reads `source` and `rev` from `.claude/cardinal.toml` with `sed` regexes in
`value()`, not as `cardinal` reads them (`[cardinal].source`, `[cardinal].rev` through a TOML parser). A valid
configuration in another form makes CI install from another source or revision than the one `cardinal` uses, or fail
with `no source read from .claude/cardinal.toml`. Make the installer read the same two values `cardinal` reads.

# Findings
* Found by the `high` review of `fail_the_ci_installer_when_the_source_is_empty` (2026-10-29). `value()` handles a
  basic or a literal string on one line, the key indented or not, and nothing else:
  * any table: a `source` or `rev` key in a table before `[cardinal]` wins (`head -n 1`);
  * an inline table `cardinal = { source = ... }`, a quoted key `"source" = ...`: not read, so the installer fails;
  * a multi-line literal `'''...'''` reads as the empty string, so it fails; a basic string with an escaped quote is
    cut at the escaped quote.
* `cardinal init` and `cardinal update` write the canonical one-line form, so only a hand-edited configuration hits
  these cases.

# Approach
1. Read the two keys with a TOML parser the runners already have, e.g.
   `python3 -c 'import tomllib, sys; ...'` (Python 3.11 or later); check first that every runner of `cardinal.yml`
   ships it (not verified).
2. Otherwise keep `sed`, but only inside `[cardinal]`: no stray key from another table, and a configuration it
   cannot read still fails with the no-source error.
In both cases keep the `no source read from` error and its tests in `crates/cardinal/tests/install_script.rs`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* `issues/done/fail_the_ci_installer_when_the_source_is_empty.md`
* ADR 0010 (the prebuilt binary of the CI)

# Hints
* The installer runs before `cardinal` exists, so it can't call `cardinal` to read its own configuration.

# Tracking

Plan: validated

## Understanding
The installer reads `[cardinal].source` and `[cardinal].rev` with Python's `tomllib` (`python3 -c`), the values
`cardinal` reads through `toml`; the `sed` reader goes. `ubuntu-latest` (24.04) ships Python 3.12; a runner without
Python 3.11+ fails with its own error. Out of scope: the rest of the installer (URL resolution, release fetch).

## Plan
1. [x] Read the configuration with `tomllib`: `value()` becomes a `python3 -c` over `[cardinal]`, a check of
   Python 3.11+ before it; verified by `install_script.rs` tests (another table's key, inline table, quoted key,
   multi-line and escaped strings, non-string `rev`, empty or non-string `source`, invalid TOML, a fake `python3`
   without `tomllib`); makes true SYN-56, SYN-57, SYN-58. ~150 lines.
2. [x] Fixes of the `high` review: one `python3` call printing `shlex`-quoted assignments (trailing newlines kept,
   bytes written whatever the locale), `ValueError` caught (a file not UTF-8); verified by `install_script.rs`;
   makes true SYN-56, SYN-57.

## Decisions
- Mode: autonomous
- No Python 3.11+: fail naming it, no `sed` fallback (one reader, the one `cardinal` matches).
- Base commit: 2f0fd164623f8087a6262e923729efe004709622

## Deliverable 1
- done: `install_cardinal.sh` reads `[cardinal].source` and `rev` through `python3 -I` and `tomllib`, after a check of
  Python 3.11+; SYN-56 to SYN-58 added to `specs/sync.md`, traced, spec `accepted`; `docs/architecture.md` names the need.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | `docs/architecture.md` |
| code review | see below |

- red: SYN-56, SYN-57, SYN-58: `rev 'other' is not a full commit hash` (a `source` of another table won), exit 0
  instead of 1 for an empty, non-string or invalid `source` and for a runner without `tomllib`.
- Code review: no findings
- not verified: a real `ubuntu-latest` run of the installer (Python 3.12 assumed from the runner image).
- decisions needed: none

## Deliverable 2
- done: the `high` review's findings fixed in `install_cardinal.sh`; the test module names its `python3` need.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | none (the comments of the script) |
| code review | see below |

- red: SYN-56, SYN-57: `source = "\\n"` read as no source; a byte 0xff gave a `UnicodeDecodeError` traceback, not the
  no-source error.
- High review: 6 findings: not UTF-8 (fixed), trailing newline (fixed; a NUL byte stays lost, the shell holds none),
  its missing test (added), locale encoding (fixed, `stdout.buffer`), tests need Python 3.11+ (stated in the test
  module doc; `ubuntu-latest` has it), three interpreter starts (one now).
- Code review: no findings
- not verified: a real `ubuntu-latest` run.
- decisions needed: none
