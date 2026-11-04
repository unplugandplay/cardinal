# Refuse a cardinal older than the source it syncs

# Goal
An installed `cardinal` older than the source revision it syncs, or than the lock it reads, misreads the managed
content without saying so. `update` and `sync` must tell the user to install the matching `cardinal` instead of
syncing wrongly or failing with an obscure error.

# Findings
* Found by the `high` review of `allow_cardinal_run_in_the_synced_projects` (ADR 0013): the managed entries now hold
  `permissions.allow` rules.
* A binary from before ADR 0013 reads only `permissions.deny` and hooks in `entries/.claude/settings.json`: syncing a
  newer source drops the allow rule `Bash(cardinal run *)` silently, and the seed no longer carries it.
* The same binary reading a lock written by a newer one fails on an `entry {"allow": ...}` line with
  `not a settings entry` (exit 3), without saying to upgrade.
* The guard of `update` against an outdated binary (`launcher::refused`) only checks that the installed launcher
  accepts the hook subcommands; it does not see a change of the fragment or lock format.

# Approach
1. A format version: the source declares the minimum `cardinal` version it needs (in `entries/` or a source file),
   the lock records it; `sync`, `update` and `check` refuse with the install command when the running binary is
   older.
2. Or narrower: an unknown key in the fragment, or an unknown entry line in the lock, is an error that names the
   upgrade, instead of being ignored or reported as a malformed entry.

# Specs
* `specs/sync.md`: a new requirement next to SYN-29 and SYN-30.

# Resources
* `crates/cardinal/src/entries.rs` (`of_source`, `from_lock`), `crates/cardinal/src/launcher.rs` (`refused`),
  `crates/cardinal/src/lib.rs` (`update`)
* ADR 0007, ADR 0013, `issues/done/install_the_new_harness_before_merging_a_hook_change.md`

# Hints
* A binary already released cannot be changed: only the binaries from this issue on can refuse a newer source.

# Tracking

Plan: validated

## Understanding
`sync`/`update` refuse a source whose declared format (`cardinal-format`, a whole number) exceeds the one compiled in
the binary, and `sync`/`update`/`check` a lock that records a higher one, naming the install command (approach 1 of
the issue, a format integer rather than the crate version, never raised). Out of scope: the binaries already released.

## Plan
1. [x] Source format: `cardinal-format` at the root (2), `FORMAT` in the binary kept equal by a test, `sync` and
   `update` refuse a higher one with the install command; verified by tests of `sync_from`, `update_in` and the
   format test; makes true SYN-40, SYN-43.
2. [x] Lock format: `write_lock` records `format <n>`, `read_lock` reads it, `check`, `sync` and `update` refuse a
   higher one with the install of the head of the source; ADR 0014 accepted, docs; verified by lock tests; makes
   true SYN-41, SYN-42.
3. [x] Fixes of the `high` review: the lock records the format of its source, read before any other line; `sync`
   names the commit it checked out; verified by `format_cli` tests; makes true SYN-40, SYN-41, SYN-42.

## Decisions
- Mode: autonomous
- Base commit: `896f6210b22ceb668c24dd7fbfb9255afc00130b`
- The format is declared in `cardinal-format` at the root of the source.
- The lock records the format of the binary that wrote it; its refusal installs the head of the source.
- Format 2 is the one of ADR 0013; no declaration means 1.
- `--dry-run` refuses too.
- After the `high` review (option a): the lock records the format of the source it was synced from, not of the
  binary, so a newer binary never makes a lock unreadable by the one pinned with it (CI, teammates); its refusal
  installs the pinned revision.

## Deliverable 1

- done: `cardinal-format` (2) at the root, `FORMAT` in the binary, `sync` and `update` refuse a newer source with
  the install command, `--dry-run` included.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | usage, architecture, CLAUDE.md, docs map |
| code review | low: no findings |

- red: SYN-40, SYN-43: `assertion left == right failed: left: Some(0) right: Some(3)` (sync accepted format 99);
  `cardinal-format` not found
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2

- done: the lock records `format <n>`; `check`, `sync` and `update` refuse a lock of a newer format with the install
  of the head of the source; ADR 0014 and `specs/sync.md` accepted; this repository's lock resynced.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | usage, architecture |
| code review | low: no findings |

- red: SYN-41, SYN-42: `assertion left == right failed` (no `format` line in the lock; `check` exited 0 on a newer
  lock)
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3

- done: the lock records the format of its source, as its first line, read before any other; its refusal installs
  the pinned revision; `sync` of a git source names the commit it checked out and reads `harness.toml` once.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | usage, architecture, ADR 0014 |
| code review | low: no findings |

- red: SYN-40, SYN-41, SYN-42: `assertion failed` (no `--rev <commit>` in the install of `sync`; lock line
  `format 2` instead of `format 1`; `not a settings entry` before `format 99`)
- `high` review, not fixed: `init` does not check the format (it writes only `harness.toml`, the next `sync`
  refuses); `FORMAT` read from `cardinal-format` by `include_str!` (the crate would read a file outside its
  directory; SYN-43 keeps them equal); duplicated test helpers.
- Code review: no findings
- not verified: none
- decisions needed: none
