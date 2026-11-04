# Propagate new hook declarations on sync

# Goal
A hook added to `files/.claude/hooks/` reaches a project that already adopted the harness only when its declaration
is added by hand to the project's `.claude/settings.json`, which is seeded once. Make `harness sync` (or `update`)
able to add the managed hook declarations itself, without taking ownership of the rest of the settings.

# Findings
* `docs/architecture.md`, "Hooks and their declaration": `settings.json` is seeded, so a new hook needs a manual
  edit in each project; `harness check` and `sync` only warn (issue `warn_of_the_settings_at_sync`, done).
* The same applies to the `permissions.deny` rules of the review waiver (`DENY_RULES` of
  `crates/harness/src/settings.rs`): a project synced before they existed lacks them.
* Switching the hooks from `rust-script` to prebuilt binaries (issue `ship_the_harness_scripts_as_prebuilt_binaries`)
  would change every hook command, so it needs this first.

# Approach
1. Measure: list what differs today between the seeded settings and the settings of the consumer projects
   (hooks, deny rules, attribution), to size the need.
2. A managed fragment: the harness owns a set of entries (hook declarations marked as its own, deny rules) and
   `sync` adds, updates or removes only those, leaving every other key as the project wrote it.
3. Or Claude Code's own layering: put the managed declarations in a separate settings file Claude Code reads
   (check which files it merges) and keep the project's file untouched.

# Specs
* `specs/sync.md`: new requirements for the managed settings entries.

# Resources
* `crates/harness/src/settings.rs`, `seed/.claude/settings.json`, `files/.claude/test_settings.rs`
* `docs/adr/0001-shared-harness-synced-by-a-rust-tool.md` (no merge of managed files)

# Hints
* ADR 0001 rejected merging managed files; a JSON entry merge is a different decision and needs its own ADR.
* JSON written back must keep the project's key order and formatting, or every sync makes a diff.

# Tracking

Plan: validated

## Understanding
`sync` owns a fragment of `.claude/settings.json` (deny rules and hook objects declared in the source's
`entries/.claude/settings.json`, recorded in the lock) and adds, replaces or removes only those. Measured: the one
other consumer differs from the seed only by the order of its two `PostToolUse` groups and one allow rule of its own.
Out of scope: the attribution (stays a warning), the allow list. Claude Code has no second shared settings file per
project, so approach 3 is out (ADR 0007).

## Plan
1. [x] Add the missing entries: source fragment, merge into the settings, lock record, report and `--dry-run`, invalid
   settings skipped; the seed drops its hooks and deny rules (moved to the fragment); verified by sync tests; makes
   true SYN-29, SYN-30, SYN-32, SYN-34, SYN-35.

   done: `entries.rs` merges `entries/.claude/settings.json` into the project's settings after the seeds, the lock
   records the entries (`entry  <json>` lines), warnings judged on the merged settings; seed and this repository's
   settings reformatted, REV-19 now reads the fragment.

   | check | result |
   |---|---|
   | tests | `just check` ok (70 + 1) |
   | lint | clippy clean |
   | specs | ok (SYN-31, SYN-33, SYN-36 untraced until deliverable 2, spec `draft`) |
   | docs | deferred to deliverable 3 (plan) |
   | harness | `just test-harness` ok |

   red: `adds_each_managed_entry_the_settings_lack_and_keeps_the_rest`: deny was `["Bash(rm *)"]`, expected the
   managed rule appended.
   Code review: 1 finding — `merge` ignores the recorded entries (a dropped entry stays, a deleted one is re-added):
   the scope of deliverable 2 (SYN-31, SYN-33), not fixed here.
   not verified: an old `harness` binary reading a lock with `entry` lines (it would report them as missing files).
   decisions needed: none.
2. [x] Remove and replace: entries the source dropped or changed, an entry the project edited or removed, `check`
   reporting; verified by sync and check tests; makes true SYN-31, SYN-33, SYN-36.

   done: `merge` removes the recorded entries the source dropped (empty group and event dropped, the other events in
   order), reports a recorded entry no longer held as a conflict (`--force` adds it back), `check` reports it as
   `missing: .claude/settings.json <entry>`. A changed hook is appended to its group, not put at the old position.

   | check | result |
   |---|---|
   | tests | `just check` ok (75 + 1) |
   | lint | clippy clean |
   | specs | ok, every SYN requirement traced |
   | docs | deferred to deliverable 3 (plan) |
   | harness | `just test-harness` ok |

   red: `removes_the_entries_the_source_dropped_and_the_groups_they_empty` and three others: the dropped entries
   stayed, the removed ones were added back, `check` reported nothing.
   Code review: no findings
   not verified: none.
   decisions needed: none.
3. [x] Docs: `docs/architecture.md` ("Hooks and their declaration"), README, the hint of `test_settings.rs` naming
   the fragment, ADR 0007 accepted, spec accepted; verified by `just check`, `just test-harness`.

   done: architecture (sync steps, lock, hooks section, deny rules), README, CLAUDE.md index, update-docs map row,
   module docs of `lib.rs` and `settings.rs`, the header of `require_code_review.rs` and the message of
   `test_settings.rs` (in `files/`, synced); ADR 0007 and `specs/sync.md` accepted.

   | check | result |
   |---|---|
   | tests | `just check` ok |
   | lint | clippy clean |
   | specs | `specs/sync.md` accepted, all traced |
   | docs | updated (this deliverable) |
   | harness | `just test-harness` ok, `harness check` clean |

   red: none (docs only).
   Code review: no findings
   not verified: none.
   decisions needed: none.
4. [x] Fixes of the `high` review of the issue (10 findings).

   done: a changed hook takes the old one's place (group and event kept); settings seeded again take the entries
   without conflict; a dropped entry the project edited is reported `kept`; `check` reports nothing of missing or
   unparsable settings; `""` and `"*"` matchers equal none; one `SETTINGS` constant. SYN-29, SYN-31, SYN-36 and the
   scenarios amended, architecture adjusted.

   | check | result |
   |---|---|
   | tests | `just check` ok (79 + 1) |
   | lint | clippy clean |
   | specs | accepted, all traced |
   | docs | architecture, `specs/sync.md` |
   | harness | `just test-harness` ok |

   red: `a_changed_hook_alone_in_its_group_keeps_its_place` (`PreToolUse`… order changed), `settings_seeded_again_…`
   (3 conflicts), `a_dropped_entry_the_project_edited_…` (no `kept`), `check_reports_nothing_…` (missing reported),
   `an_empty_or_star_matcher_…` (hook added twice).
   High review, not fixed: an older `harness` reading `entry` lock lines (CI installs the pinned revision, so only a
   stale local binary); `DENY_RULES` doubling the managed deny rules (SYN-24 holds for any source, and a test keeps
   the fragment a superset of REV-19); `entries()` cloning to validate (a few hooks, not worth a second walker).
   Code review: 1 finding — `missing()` silent on absent settings: intended (SYN-36), but its comment wrongly cited
   a warning; fixed.
   not verified: none.
   decisions needed: none.

## Decisions
- Mode: autonomous
- Base commit: 3b7bd0061a5767aede736e5e3a09fa091d3c3379
- Written settings: `serde_json` pretty print with key order kept; the seed and this repository's settings reformatted
  once to it.
- A managed entry the project edited or removed is a conflict (fails unless `--force`, which restores it).
- The attribution stays a warning (SYN-26), not managed.
