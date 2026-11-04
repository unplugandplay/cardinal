# Allow cardinal run in the synced projects

# Goal
The managed skills tell the agent to run `cardinal run <tool>`, but the rule that allows it without a prompt,
`Bash(cardinal run *)`, is only in the seeded settings, copied once. A project seeded before the rename allows
`Bash(harness run *)` only, so each `cardinal run` of a skill asks for a permission (denied in a non-interactive run).
Make the synced projects allow `cardinal run` without the user editing their settings.

# Findings
* `seed/.claude/settings.json` holds `Bash(cardinal run *)` since the binary rename; the managed entries of
  `entries/.claude/settings.json` hold deny rules and hooks, no allow rule (SYN-29).
* Found by the `high` review of `rename_the_binary_to_cardinal`.

# Approach
1. Let `entries/.claude/settings.json` manage allow rules as it manages deny rules, and declare
   `Bash(cardinal run *)` there.
2. Or have `check` and `sync` warn when the settings allow `Bash(harness run *)` and not `Bash(cardinal run *)`,
   as SYN-24 warns of a missing deny rule.

# Specs
* `specs/sync.md` (SYN-29 to SYN-36 for managed entries, or a warning next to SYN-24).

# Resources
* `crates/cardinal/src/entries.rs`, `crates/cardinal/src/settings.rs`, `entries/.claude/settings.json`,
  `seed/.claude/settings.json`, ADR 0007

# Hints
* The hooks themselves need no allow rule: only the commands the agent runs through Bash.

# Tracking

Plan: validated

## Understanding
Approach 1: `sync` manages `permissions.allow` like `permissions.deny` (SYN-29 to SYN-36, ADR 0013 extending 0007),
and `entries/.claude/settings.json` declares `Bash(cardinal run *)`, dropped from the seed. Out of scope: removing the
old `Bash(harness run *)` of a project (never recorded as managed, so it stays the project's).

## Plan
1. [x] Managed allow rules: `Entry::Allow` in `entries.rs` (parse, lock, add, remove, report `allow <rule>`),
   `Bash(cardinal run *)` moved from `seed/` to `entries/`, `just sync`, docs; verified by tests of
   `settings_entries_cli.rs`; makes true SYN-29, SYN-34 (allow rules), SYN-31, SYN-36 for them.
2. [x] Fixes of the `high` review: the permission lists paired with their entry constructor (no wildcard fallback), the
   doc comment of `of_source` on the order; verified by the existing tests.
3. [x] `type Rule` for the constructor table: commit 3eb4e91 went in with `clippy::type_complexity` red (the commit
   was chained without waiting for `just check`); verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: 846a38a62c324c19421fbad478943a2cacd6f75c
- Old `Bash(harness run *)`: left to the project (1 a).
- ADR: new 0013 extending 0007 (2 a).

## Summary of deliverable 1
done: `sync` manages `permissions.allow` like `permissions.deny`; `Bash(cardinal run *)` moved from the seed to the
managed entries; docs and ADR 0013.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | updated (architecture, usage, README, CLAUDE.md, docs map) |
| code review | `low`, no findings |

red: SYN-29, SYN-34, SYN-31, SYN-36: the allow rule was neither added (`left: ["Bash(harness run *)"]`), reported,
removed nor reported missing (`check` exit 0).
Code review: no findings

not verified: none. decisions needed: none.

## Summary of deliverable 2 (fixes of the `high` review)
done: `PERMISSIONS` pairs each list with its constructor, `Entry::rule` and its wildcard gone; `of_source` doc fixed.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | n/a (no behaviour change) |
| code review | `low`, see below |

red: none (refactor under the green tests).

`high` review: 8 findings.
- older binary drops the allow rules / fails on the new lock lines: issue
  `refuse_a_cardinal_older_than_the_source_it_syncs`.
- malformed `permissions.allow` stops the sync of every entry: not fixed, the same as a malformed `deny` (SYN-35).
- a managed allow rule cannot be revoked, a seeded `Bash(cardinal run *)` is adopted as managed: not fixed, by design
  as for the hooks and deny rules (ADR 0007).
- wildcard `Entry::rule`, twin variants: fixed by the constructor table.
- `of_source` doc on the order: fixed.
Code review: no findings

## Summary of deliverable 3
done: `type Rule = fn(String) -> Entry`, clippy green again. red: none. Code review: see below.
Code review: no findings
