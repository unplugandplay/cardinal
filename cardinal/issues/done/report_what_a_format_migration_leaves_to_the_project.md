# Report what a format migration leaves to the project

# Goal
When `update` migrates a project to a new format (as SYN-45 did, withdrawn by ADR 0016), it renames the managed names, but the project-owned files
still cite the old ones, and nothing tells the agent the migration exists before it runs `update`. In the two
migrations to `cardinal`, the agent of each consumer spent its session finding and fixing those leftovers by hand.
Make the next format migration discoverable and make `update` say, or fix, what it leaves behind.

# Findings
* Measured on 2026-10-29, the sessions that migrated the two consumers from `harness` to `cardinal` (Claude Code,
  Opus): each found that `update` migrates by reading ADR 0015 and `docs/usage.md`; one cloned the source into a
  temporary directory to read them. Nothing printed by `check` or `sync` on a project with `.claude/harness.toml`
  pointed at `cardinal update`.
* The `source` URL still named the repository before its rename; it was edited by hand in both (`sed` by the agent
  in one, by the user in the other). The old URL still resolves through the forge redirect.
* `update` rewrote the `import` line of the justfile (SYN-45) but not the `check: check-harness` dependency of the
  seeded `check` recipe, nor the header comments of `.claude/cardinal.toml`, still the old seed's text word for word
  (`the shared harness reads … harness sync`, `besides docs and harness`).
* About six `grep`/`sed` calls per consumer went to the old names left in project-owned files: `CLAUDE.md`,
  `README.md`, `docs/`, `.claude/update-docs-map.md` (`harness.toml`, `harness run`, `check-harness`,
  `cargo install … harness --locked`, `install_harness.sh`, `harness.yml`).

# Approach
1. `update`, after a migration, warns with the lines of the project-owned files (tracked by git, outside the
   managed files, `issues/done/` and `docs/adr/`) that still cite an old managed name, `path:line`, without
   rewriting them.
2. Extend the migration to the seeded text it can recognise exactly: the `check-harness` dependency of the
   justfile, the header comments of the configuration when they are byte-identical to the old seed.
3. `check` and `sync` on a project below the current format with a source of a newer one print a hint to run
   `cardinal update`.
4. Optionally, `update` rewrites a `source` URL the forge redirects (a renamed repository), or only reports it.

# Specs
* `specs/sync.md`: new requirements for the migration, the leftover report and the migration hint.

# Resources
* ADR 0014, ADR 0015, ADR 0016
* `crates/cardinal/src/lib.rs` before `faea050` (`Migration`, `imports_renamed`), the migration it dropped
* `issues/done/rename_the_consumer_files_to_cardinal.md`

# Hints
* No consumer is left to migrate to `cardinal`: the value is for the next change of `FORMAT`, so the old names to
  look for come from the format change, not hard-coded `harness` strings.
* The format 2 migration code is gone (`drop_the_old_names_of_the_consumer_files`): the next migration brings its
  own, and this issue builds on it.

# Tracking

Plan: validated

## Understanding
Make the next format change discoverable and self-reporting: the source declares its migration as data
(`migrations/<n>.toml`: old/new names, exact seeded texts), `sync`/`update` rewrite those texts and warn `path:line`
of each leftover citation; `check`/`sync` hint at `update` below the binary's format. Out of scope: the `source` URL
of a renamed repository (forge redirect), a migration that renames the configuration or the lock (its own code).

## Plan
1. [x] Migration declarations and seeded rewrites: read `migrations/<n>.toml` of the applying formats, rewrite the
   whole-line texts, `rewritten:` lines, ADR 0017; verified by binary tests (lock of format 1 or 2, source 3);
   makes true SYN-52, SYN-53. ~400 lines.
2. [x] Leftover report: scan the git-tracked project-owned files for the old names after the rewrites, warn
   `path:line`; verified by binary tests; makes true SYN-54. ~300 lines.
3. [x] Format hint in `check` and `sync`; verified by binary tests (lock `format 2`); makes true SYN-55, spec
   `accepted`. ~150 lines.
4. [x] Fixes of the `high` review: `--dry-run` judges the files as the run leaves them (removed, merged settings),
   CRLF line ends, old names as words, chained renames named by the last one, no hint for a local source, `update`
   always edits the configuration as the sync left it, the hint from the sync's own lock format; verified by binary
   tests; SYN-53, SYN-54, SYN-55 amended.

## Decisions
- Mode: autonomous
- Base commit: 62fdd8ddc14eb39638abf0baf46f9ba420ee229f
- Migration declared as data in the source, `migrations/<n>.toml` (1 a).
- Hint in `check` and `sync` (2 a).
- Leftovers reported once, by the run that applies the migration (3 a).
- The `source` URL of a renamed repository is out of scope (4 a).

## Deliverable 1

- done: `migrations/<n>.toml` read above the format of the lock (`migration.rs`), whole-line seeded rewrites with
  `rewritten:` lines, `update` keeps a rewritten configuration under its new `rev`; ADR 0017, usage, architecture.
- red: SYN-52, SYN-53: `assertion left == right failed: left: None, right: Some("rewritten")` (no outcome for the
  justfile).
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2

- done: after the rewrites, `sync` and `update` warn `path:line` of each tracked project-owned line citing an old
  name (managed files, lock, `issues/done/`, `docs/adr/` skipped), judged on the rewritten texts under `--dry-run`.
- red: SYN-54: `assertion left == right failed: left: [], right: ["warning: README.md:1 cites check-old, …"]`.
- Code review: 1 findings: an old name contained in its new one reported every migrated line; fixed (`migration::cites` skips the old name inside the new one, unit test and SYN-54 scenario).
- not verified: none
- decisions needed: none

## Deliverable 3

- done: `check` and `sync` print the `cardinal update` hint on stderr while the lock is below the binary's format
  (`sync` judges the lock it leaves); `update` does not; spec `accepted`.
- red: SYN-55: `assertion left == right failed: ["check"] … left: 0, right: 1` (no hint printed).
- Code review: no findings
- not verified: the hint of a `sync` that leaves a lock below the format (no source below 3 is synced any more).
- decisions needed: none

## Deliverable 4

- done: eight of the ten `high` review findings fixed, spec amended (scenarios of SYN-53, SYN-54, SYN-55).
- red: SYN-53, SYN-54, SYN-55: `left: "check: check-old\r\n…"`, `cites x-old, named check-old since the format 2`,
  `.claude/settings.json:8 cites check-old` under `--dry-run`, `hint:` printed for a local source.
- `high` review (`62fdd8d..86df30f`): 10 findings. Fixed: dry-run on removed files, dry-run on merged settings,
  CRLF, chained renames, word boundary, hint for a local source, `update` re-reads the configuration, `sync` reads
  the lock format from its own run. Not fixed: the preview of a conflicting `update` prints `rewritten:` and the
  leftovers, as it prints every other outcome it would apply, under `conflicts: nothing written`; the double read of
  the tracked files by `update` (preview, then sync) is the price of SYN-13's preview, `update` is rare.
- Code review: no findings
- not verified: none
- decisions needed: none
