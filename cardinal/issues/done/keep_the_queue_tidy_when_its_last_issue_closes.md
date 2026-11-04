# Keep the queue tidy when its last issue closes

# Goal
`close_issue.rs` drops the line of the closed issue from `issues/queue.md` and writes the kept lines joined by `\n`
plus one `\n`. When that line was the last entry, the blank line that separates the header from the list stays, so
the queue ends with `\n\n`: an empty queue then differs from a freshly written one. The queue should end with
exactly one newline whatever entry is removed.

# Findings
* 2026-10-26, `/code-review high` of `count_typed_slash_commands_as_interventions`: closing the last open issue
  left `issues/queue.md` ending with `\n\n`.

# Approach
1. Trim the trailing blank lines of the kept lines before writing them back, then add one `\n`.
2. A script test in `test_close_issue.rs` (or the test that already covers `close_issue.rs`): closing the last
   entry leaves the header ending with one newline.

# Resources
* `files/.claude/skills/issue-workflow/close_issue.rs` (the queue rewrite in `main`)
* `issues/queue.md` (line format)

# Hints
* Change `files/`, then `just sync`; `just test-harness` runs the script tests.

# Tracking

Plan: validated

## Understanding
After `close_issue.rs` drops a queue line, the queue must end with exactly one `\n`, also when the dropped line was
the last entry (the blank line under the header is trimmed). Out of scope: any other queue edit, a spec (the queue
rewrite is no requirement of `specs/parallel-issues.md`; this is a cosmetic fix of a harness script).

## Plan
1. [x] Trim the queue on close: drop the trailing blank kept lines before writing them back with one `\n`; verified
   by a new check in `test_close_issue.rs` (closing the only entry leaves the header ending with one newline, closing
   a middle entry keeps the others).
2. [x] Fixes of the `high` review: a queue with no header is left empty, not `\n`; verified by a new check in
   `test_close_issue.rs`.

## Decisions
- Mode: autonomous
- Base commit: 48eeb1bddb7669e1167fe38b441dc7f2f0044a18

## Deliverable 1

- done: `close_issue.rs` drops the trailing blank lines of the queue before writing it back with one `\n`.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| specs | no spec concerned |
| docs | no change needed |
| code review | `/code-review low` run |

- red: `FAIL - closing the last entry leaves the header ending with one newline`
- Code review: no findings
- not verified: none
- decisions needed: none

## Review high (48eeb1b..3800b7c)

| # | finding | done |
|---|---|---|
| 1 | trimming drops the blank line between the header and the list | kept: an emptied queue now equals `seed/issues/queue.md`, the goal |
| 2 | a headerless queue becomes `\n` | fixed (deliverable 2) |
| 3 | queues already emptied keep `\n\n` | not fixed: no entry to close, a one-time hand fix in the consumer |
| 4 | CRLF turned into LF on rewrite | not fixed: pre-existing, every consumer is LF |
| 5 | line match duplicates `next_issue.rs` `entry()` | not fixed: rust-scripts share no code, pre-existing |
| 6 | second pass to detect a removal | not fixed: pre-existing, negligible |

## Deliverable 2

- done: closing the only entry of a headerless queue leaves it empty.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| specs | no spec concerned |
| docs | no change needed |
| code review | `/code-review low` run |

- red: `FAIL - a queue with no header is left empty`
- Code review: no findings
- not verified: none
- decisions needed: none
