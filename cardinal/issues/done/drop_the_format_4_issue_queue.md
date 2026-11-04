---
rank: 100
why: 'first: a small cleanup that nothing blocks; the two others wait on the user''s choice or on sessions'
---
# Drop the format 4 issue queue

# Goal
The format 5 orders the issues by the `rank` of each issue file (ADR 0019), and `cardinal` still reads the format 4
`issues/queue.md` so that a consumer not yet converted keeps working. Every consumer is now converted (the user, on
2026-10-31): the code, specs, tests and docs of the queue are dead weight that every reader of `next-issue` and
`close-issue` still pays. The outcome: no tool reads or writes `issues/queue.md`, and the docs no longer describe it.

# Findings
* `crates/cardinal/src/tools/next_issue.rs`: reads `queue.md` while it exists (NXT-4, `from_queue`), `--convert`
  turns it into ranks (NXT-5, `convert`), hints at `--convert` when a file has a rank the queue hides (NXT-6), and
  leaves `queue` out of the issue stems (NXT-1).
* `crates/cardinal/src/tools/close_issue.rs`: `drop_from_queue` removes the closed issue's line while the file exists.
* `crates/cardinal/src/tools/issue_worktree.rs`: skips the stem `queue`.
* Tests: `tests/next_issue.rs` (28 hits), `tests/close_issue.rs` (18), `tests/skills.rs` (2 allowed citations of
  `issues/queue.md` for NXT-6).
* Specs: `specs/next-issue.md` (NXT-1 exclusion, NXT-4 to NXT-6 and their scenarios), `specs/close-issue.md` (a CLS-4
  scenario cites `queue.md` as an example link), `specs/parallel-issues.md` (PAR-8 scenarios speak of "the queue").
* Docs: `docs/usage.md` (the paragraph on a project seeded before the format 5), the comment of `next-issue` in
  `files/cardinal.just` (`--convert`).
* `migrations/5.toml` renames `issues/queue.md` and names `just next-issue --convert` in its warning.
* `session_metrics.rs` and `memory.rs` hit `queue` for other meanings (a transcript record type, a BFS queue): out of
  scope.

# Approach
1. Withdraw NXT-4 to NXT-6 (marked withdrawn, ids kept), drop `queue.md excluded` from NXT-1, reword the scenarios of
   CLS-4 and PAR-8 that cite the queue.
2. Remove the queue paths of `next-issue`, `close-issue` and `issue-worktree`, and their tests.
3. Docs: `docs/usage.md`, `files/cardinal.just` (then `just sync`).
4. Open questions for step 1 of `issue-workflow`:
   - Does dropping the reading of a file need a raised format (ADR 0014)? A pinned revision runs its own binary, so an
     older consumer keeps its `next-issue`; a lock of the format 4 synced to the new source would lose its queue.
   - `migrations/5.toml` names `just next-issue --convert` in its warning text: keep it as history (a migration is
     applied once, from a lock of the format 4), or reword it.

# Specs
* `specs/next-issue.md`: NXT-1 amended, NXT-4, NXT-5, NXT-6 withdrawn.
* `specs/close-issue.md`, `specs/parallel-issues.md`: scenarios reworded, no requirement changed.

# Resources
* `docs/adr/0019-each-issue-file-carries-its-rank.md`, `docs/adr/0014-the-source-declares-the-format-cardinal-must-read.md`,
  `docs/adr/0017-the-source-declares-its-migrations.md`.
* `issues/done/rank_the_issues_without_a_shared_queue_file.md`: how the queue was replaced.

# Hints
* `issues/done/` and `CHANGELOG.md` cite the queue as history: leave them.

# Tracking

Plan: validated

## Understanding
No tool reads or writes `issues/queue.md` any more, and the specs and docs no longer describe it. Out of scope: the
history (`issues/done/`, `CHANGELOG.md`, the ADRs) and the other meanings of "queue" (`session_metrics.rs`, `memory.rs`).

## Plan
1. [x] `next-issue` without the queue: remove `from_queue`, `--convert`, the NXT-6 hint and the `queue` stem
   exclusion; move the setups of `tests/next_issue.rs` that wrote a `queue.md` to ranked files, delete the NXT-4 to
   NXT-6 tests, reword the reason of the two banned citations of `tests/skills.rs`; the `next-issue` comment of
   `files/cardinal.just` and the paragraph of `docs/usage.md` (they name `--convert`); verified by `just check`;
   makes true NXT-1 (amended), NXT-4 to NXT-6 withdrawn (~450 lines).
2. [x] `close-issue` and `issue-worktree` without the queue: remove `drop_from_queue` and the `queue` skip, their
   tests (`queue_checks`, the queue of the PAR-8 test); verified by `just check`; makes true CLS-4 and PAR-8 as reworded
   (~250 lines).
3. [x] `cardinal` syncs the format 5 and higher: `MIN_FORMAT` 5, ADR 0020 (supersedes the minimum of ADR 0016 and the
   queue part of ADR 0019), `migrations/4.toml` and `5.toml` removed, SYN-51 amended; the high review of the whole
   issue (PAR-4 citation, CLS-4 fixture) fixed in it; verified by the SYN-51 tests at the format 4; makes true SYN-51
   (~150 lines).

## Decisions
- Mode: autonomous
- Base commit: be60fe270285d7d916178dc8ff92f5f4d0cbb02e
- No format raise (ADR 0014): the managed content keeps its meaning, only the binary stops reading a project file.
- `migrations/5.toml` keeps its text as history: it applies once, to a lock of the format 4. Superseded after the
  high review: the minimum format rises to 5 (ADR 0016 foresaw it), so `migrations/4.toml` and `5.toml` go.

## Deliverable 1
done: `next-issue` reads only the ranks; `--convert`, the queue reading and its hint are gone, with their tests and docs.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (`check-specs --strict`) |
| docs | `docs/usage.md`, `files/cardinal.just` |
| code review | no findings |

red: none (a removal: the withdrawn requirements lose their tests, the kept ones stay green)

not verified: none. decisions needed: none.
Code review: no findings (`split_front_matter` is still used by `front_matter` and `title`; `--convert` is now read as a directory and exits 2, as any unknown argument).

## Deliverable 2
done: `close-issue` no longer edits a `queue.md`, `start-issue` accepts the stem `queue`; the PAR-8 test checks the
issue files alone, the CLS-4 test cites `bar.md#top` like its reworded scenario.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (`check-specs --strict`) |
| docs | none left (the module comment of `close_issue.rs`) |
| code review | no findings |

red: none (a removal)

not verified: none. decisions needed: none.
Code review: no findings

## Deliverable 3
done: `MIN_FORMAT` is 5 (ADR 0020, ADR 0016 superseded, ADR 0019 partly), `migrations/4.toml` and `5.toml` removed,
SYN-51 and its docs at the format 5; the high review's PAR-4 citation and CLS-4 fixture (a link to a missing
`gone.md#top`) fixed.

High review of `be60fe2..HEAD`: 8 findings. Fixed: the migration naming `--convert` (removed), `MIN_FORMAT` (5),
ADR 0019 (partly superseded), PAR-4 citing NXT-4, the CLS-4 fixture. Not fixed: a leftover `queue.md` read as an
issue and `start-issue queue` accepted (`next-issue` reports it as `issue without a rank: queue`), `--convert` read
as a directory (as any unknown argument).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (`check-specs --strict`) |
| docs | `docs/usage.md`, `docs/architecture.md`, ADR 0016, 0019, 0020 |
| code review | no findings |

red: SYN-51: `init_and_sync_refuse_a_source_below_the_format_5_and_write_nothing` panicked at `format_cli.rs:281`
(the exit code of a
sync of a source of the format 4 was not 3)

not verified: none. decisions needed: none.
Code review: no findings

Status: done (3 deliverables committed; nothing left).
