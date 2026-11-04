---
rank: 100
why: 'for team use, and its risk of conflict is not measured yet'
---
# Rank the issues without a shared queue file

# Goal
`issues/queue.md` holds the order of every open issue in one file that `create-issue` inserts into,
`close-issue` removes from and `prioritize-issues` rewrites. When several people (or parallel issue worktrees)
create or close issues on different branches, they edit neighbouring lines of the same file and their merges
conflict. Keep one readable order of the issues without that conflict.

# Findings
* Writers of `issues/queue.md`: `skill:create-issue` (step 5, inserts a line at its rank), `close-issue` (removes a
  line), `skill:prioritize-issues` (rewrites the list), `skill:issue-workflow`.
* Reader: `next-issue` (`crates/cardinal/src/tools/next_issue.rs`) prints the first line and reports the drift
  (queue lines without a file, issues missing from the queue).
* No conflict on `issues/queue.md` has been measured yet: the risk comes from the shape of the file, one line per
  issue edited from several branches.
* The user agreed on 2026-10-29 that this is worth doing for a team.

# Approach
1. Measure first: count the merges of the history that touched `issues/queue.md` on both sides, in this repository
   and in a consumer.
2. Candidates:
   1. A rank in the front matter or the first line of each issue file (`priority: <n>` with gaps); `next-issue`
      sorts the files; a branch only touches its own issue.
   2. Keep `queue.md`, but let only the main branch rewrite it (`prioritize-issues`), new issues landing unranked
      until then and `next-issue` listing them.
   3. A git merge driver (`merge=union`) for `issues/queue.md`, keeping the file as it is.
3. Keep the "why here" of each rank, which the queue carries today.

# Specs
* `specs/close-issue.md` if `close-issue` stops editing the queue; a spec of `next-issue` if the order moves into
  the issue files (none exists yet).

# Resources
* `issues/queue.md`, `crates/cardinal/src/tools/next_issue.rs`, `crates/cardinal/src/tools/close_issue.rs`.
* `files/.claude/skills/create-issue/SKILL.md`, `prioritize-issues/SKILL.md`, `issue-workflow/SKILL.md`.
* `docs/adr/0003-one-worktree-per-issue.md`, `specs/parallel-issues.md`.

# Hints
* The queue format is read by consumers' `cardinal`: a change of its meaning raises `cardinal-format` and `FORMAT`
  (ADR 0014) and declares a migration (ADR 0017).

# Tracking

Plan: validated

## Understanding
Move the order of the open issues from `issues/queue.md` into a `rank`/`why` front matter of each issue file, so a
branch only edits its own issue (candidate 1). Measured: 0 conflict on `issues/queue.md` in this repository and in
the consumer (no merge commit, rebase then fast-forward; none in the session transcripts). Out of scope: renumbering.

## Plan
1. [x] Spec NXT, ADR 0019, `next-issue` orders the files by their `rank`, reports the missing ranks and the ties,
   keeps reading `issues/queue.md` while it exists; PAR-4 says "the first free issue" instead of "of the queue";
   verified by `tests/next_issue.rs`; makes true NXT-1, NXT-2, NXT-3, NXT-4 (~350 lines).
2. [x] `next-issue --convert` writes the ranks from `issues/queue.md` and deletes it; verified by
   `tests/next_issue.rs`; makes true NXT-5 (~200 lines).
3. [x] Format 5: `cardinal-format` and `FORMAT` to 5, `migrations/5.toml` (warns the citations of
   `issues/queue.md`), `seed/issues/queue.md` dropped, `create-issue`, `prioritize-issues`, `issue-workflow`
   (`details.md`) and the docs say the rank, `close-issue` keeps dropping a line only when the queue exists; this
   repository converted; verified by `just check`, `tests/skills.rs` and `just next-issue` (~400 lines).
4. [x] Fixes of the `high` review of the issue: the front matter read as a closed block with unquoted values, the
   `why` single-quoted, the line endings kept, `--convert` refused in a linked worktree, the hint of NXT-6;
   verified by `tests/next_issue.rs`; makes true NXT-1, NXT-5, NXT-6 as amended.

## Decisions
- Mode: autonomous
- Base commit: 1e361ed01134c61c02f32efc3a85d179cfc6b010
- Rank: a front matter `rank` (gaps of 100) and `why`, like the specs.
- Conversion: `next-issue --convert`, run by the agent that `migrations/5.toml` warns; not in `sync`.

## Deliverable 1
- done: `next-issue` orders the issue files by their front matter `rank`, reports the missing ranks and the ties;
  `issues/queue.md` still gives the order while it exists.

| check | result |
|---|---|
| tests | pass (`tests/next_issue.rs`, 5 tests) |
| lint | pass |
| specs | pass (NXT draft, NXT-5 untraced until deliverable 2) |
| docs | n/a here: the skills and docs that describe the queue change with the format 5 (deliverable 3) |
| code review | low, run |

- red: NXT-1, NXT-2, NXT-3: `no issues/queue.md` (exit 2) where the ranks were expected.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `next-issue --convert` writes `rank` (100 per position) and `why` in the front matter of each issue of
  `issues/queue.md`, keeping the other keys and the body, then deletes the queue; spec NXT `accepted`.

| check | result |
|---|---|
| tests | pass (`tests/next_issue.rs`, 6 tests) |
| lint | pass |
| specs | pass (NXT accepted, every requirement traced) |
| docs | n/a here: `--convert` is documented with the format 5 (deliverable 3) |
| code review | low, run |

- red: NXT-5: `left: "# Title B\n\nBody.\n"` where the front matter `rank: 100` was expected.
- Code review: 3 findings
  1. the queue is deleted when a line has no file: not fixed, such a line names no issue to rank and is reported
     (NXT-5).
  2. a CRLF front matter is not found by the writer and gets a second one above it: not fixed, the reader takes the
     first block, so the rank written is the one read; the issue files are written with LF.
  3. `front_matter` made private: no other caller, `just check` passes.
  A second `low` run (the issue file joined the diff) added: a front matter closed at the end of the file without
  a newline is doubled (not fixed: an issue file holds its title and body after it, `check-issue`), and a queue line
  with no reason keeps an older `why` of the file (not fixed: the files of a format 4 queue hold no `why`).
- not verified: none
- decisions needed: none

## Deliverable 3
- done: format 5 (`cardinal-format`, `FORMAT`, `migrations/5.toml`), `seed/issues/queue.md` dropped, the skills and
  docs rank each issue in its front matter, `next-issue` says `Every open issue is taken.` / `No open issue.`, this
  repository converted with `just next-issue --convert`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (PAR intent and PAR-8 reworded) |
| docs | `README.md`, `CLAUDE.md`, `ROADMAP.md`, `docs/workflow.md`, `docs/usage.md`, `docs/architecture.md`, ADR 0019 accepted |
| code review | low, run |

- red: none (the format and the docs; the test of the migrations named the format 5 as one above the source, now
  `99.toml`).
- Code review: no findings
- not verified: the migration on a consumer (RetroDoc) before its update: its `cardinal update` is the user's.
- decisions needed: none

## Deliverable 4 (review of the issue)
- done: `code-review high` on `1e361ed..HEAD` gave 10 findings; fixed: (1) the migration did not warn a stock
  consumer, now `next-issue` hints at `--convert` once a file has a rank (NXT-6); (2) `create-issue` converts a
  left queue first; (3) CRLF and a closing `---` at the end of the file are kept; (4) a line with no reason removes
  the `why`; (5) the front matter must be closed, quotes are read off; (6) `why` is written single-quoted, valid
  YAML (the two issues of this repository too); (7) the title skips the front matter; (8) `--convert` refuses in a
  linked worktree. Not fixed: (9) one front matter reader shared by the four tools: a refactor of three other
  tools, outside this issue; (10) the copy of the entries into `&str` pairs in `main`: a few lines, it keeps the
  rest of `main` unchanged.

| check | result |
|---|---|
| tests | pass (`tests/next_issue.rs`, 9 tests) |
| lint | pass |
| specs | pass (NXT-1, NXT-5 amended, NXT-6 added) |
| docs | `docs/usage.md`, ADR 0019, `create-issue`, `prioritize-issues` |
| code review | low, run |

- red: NXT-1, NXT-5, NXT-6: `issue without a rank: c` missing for an unclosed front matter, `why: needed first`
  where `why: 'needed first'` was expected, no hint naming `--convert`, `--convert` exit 0 in a linked worktree.
- Code review: no findings
- not verified: none
- decisions needed: none
