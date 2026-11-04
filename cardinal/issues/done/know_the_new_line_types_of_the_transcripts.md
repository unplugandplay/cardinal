---
priority: low
why: 'no metric is wrong today; the unknown count only hides a future format change'
---
# Know the new line types of the transcripts

# Goal
`session-metrics` counts in `unknown_lines` a transcript line whose `type` is not in its list (EVA-9), to notice when
the format moves. Claude Code now writes four types it does not know, so the count is no longer zero and a real
change would go unnoticed among them. The outcome: the known list holds these types, and `unknown_lines` is back to 0
on the current transcripts.

# Findings
* Measured 2026-11-03 over 525 sessions: 1,511 unknown lines.
* The types missing from the list: `tools-used` (708 lines in 215 sessions, e.g. `{"type":"tools-used","tools":["Bash"]}`),
  `relocated` (402 lines, 6 sessions), `worktree-state` (402 lines, 6 sessions), `permission-mode` (1 line).

# Approach
1. Add the four types to the known list, with a test per type.
2. Check whether `tools-used` gives a cheaper source for a column already computed, and say so in the issue if it does.

# Specs
* `specs/evaluation.md` EVA-9.

# Milestone
Lean agent loop

# Resources
* `crates/cardinal/src/tools/session_metrics.rs`, `KNOWN_TYPES`; `specs/evaluation.md`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
Add `tools-used`, `relocated`, `worktree-state` and `permission-mode` to `KNOWN_TYPES` so `unknown_lines` is 0 again
on the current transcripts. EVA-9 is unchanged (it names "the known list of the script"), so no spec edit. Out of
scope: reading any of these lines for a column.

## Plan
1. [x] Known new line types: the four types join `KNOWN_TYPES`; verified by a check of `tests/session_metrics.rs`
   whose fixture holds one line of each type, without `timestamp` as Claude Code writes them, and still counts
   `unknown_lines` 1 and `missing_fields` 1, plus `unknown_lines` 0 on the local transcripts; makes true EVA-9. (autonomous)
2. [x] Fixes of the `high` review: the fixture lines take the key sets of the real lines (`permissionMode` added);
   verified by `just check`. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 2ccc338a92585d46d0a56a90951690ab0ed1b2de
- Test: one check over the four types in the fixture of `s1` (answer 1 a).
- `tools-used` is no cheaper source: it lists the distinct tool names of a turn, not the calls (a session measured
  with 56 `tool_use` blocks has 2 `tools-used` lines) nor the skill names, so `tool_calls` and `skills` keep reading
  the `tool_use` blocks.

## Deliverable 1 summary
done: the four types join `KNOWN_TYPES`; `unknown_lines` sums to 0 over the 541 local sessions.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: no document lists the types |
| code review | see below |

red: EVA-9: `FAIL - unknown line types are counted, tools-used, relocated, worktree-state and permission-mode are known`
Code review: no findings

not verified: none

decisions needed: none

## High review (base..c371ed7): 3 findings
1. `relocated` known hides a change of `cwd`, `project` stays the first one: not fixed. `unknown_lines` was never a
   relocation signal, and the 7 `relocatedCwd` seen are worktrees of the same repository (counted with it) and one
   `/tmp` probe.
2. The new lines could feed `issues` or `project` through `cwd`/`gitBranch`: not applicable, the real lines of the
   four types carry neither (key sets read over every local transcript).
3. The fixture shapes are made up: fixed in deliverable 2, each line now has the keys of the real ones.

## Deliverable 2 summary
done: the `permission-mode` fixture line carries `permissionMode` as the real ones; the comment says the lines have
no `timestamp`, `cwd` nor `gitBranch`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: test fixture only |
| code review | see below |

red: none
Code review: no findings

not verified: none

decisions needed: none

All deliverables done.
