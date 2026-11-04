---
priority: medium
depends_on: [let_the_close_of_an_issue_commit_without_a_review]
why: 'each milestone end costs a refused commit and a review of one word; it does not block the close'
---
# Set the milestone done when its last issue closes

# Goal
The Rules of `issue-workflow` tell the agent to set a milestone `done` in the commit that closes its last open issue,
but REV-20 lets the close through without a review only for the references `close-issue` rewrites: the edit of the
`State:` line is refused (its scenario says so), so every milestone end costs a refused commit and a `low` review of
one word. The outcome: closing the last open issue of a milestone commits without a review, its `State:` set `done`
by the tool, not by the agent.

# Findings
* 2026-11-02, a consumer project: the agent closed the last issue of a milestone and set it `done` as the Rules ask;
  the native hook refused the commit, the agent ran `/code-review low` on the diff (no findings) and committed.
* The decision "milestone set `done` in the close commit stays gated" (1 a) of
  [let_the_close_of_an_issue_commit_without_a_review](let_the_close_of_an_issue_commit_without_a_review.md)
  is what this issue revisits, with this cost measured.
* The roadmap format is already read by tools: `## <name>` headings (NXT-10), a `- State:` line and an `- Issues:`
  list of links, and `release` adds `- Released:` after `State:` (REL-9).

# Approach
1. `close-issue` sets `- State: done` on each milestone of the `roadmap` documents whose listed issues are all in
   `issues/done/` once the move is done (a milestone `in issues` only), and prints it; the agent no longer edits it.
2. REV-20 accepts, in a roadmap document, the `State:` line `close-issue` would set for the issues the index moves,
   computed the same way, next to the rewritten references; any other edit stays gated.
3. The Rules of `issue-workflow` drop the manual edit and keep the release proposal on the line `close-issue` prints.

# Specs
* `specs/close-issue.md`: a new CLS requirement for the milestone state.
* `specs/review-gate.md`: REV-20 and its scenario amended.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/close_issue.rs`, `crates/cardinal/src/tools/review_gate.rs` (`records_close_only`).
* `files/.claude/skills/issue-workflow/SKILL.md`, Rules.
* `specs/release.md` (REL-9), `specs/next-issue.md` (NXT-10): how the roadmap is read today.

# Hints
* A milestone that lists an issue still open, or none, stays as it is; `idea` and `shaped` ones are the user's.
* The gate must recompute the state from the index, not trust the text of the roadmap.

# Tracking

Plan: validated

## Understanding
`close-issue` sets `done` the `in issues` milestones that list the closed issue once all their listed issues are in
`issues/done/`, and prints them; the gate accepts that edit by recomputing it from the `HEAD` roadmap and the index.
Out of scope: proposing or running the release (stays the agent's proposal, the user's trigger).

## Plan
1. [x] Milestone done at close: a pure `close_issue::milestones_done` (roadmap text, its path, the closed stems, an
   existence test) applied after the relink on each `roadmap` document, with its print; verified by tests of
   `close-issue` in a temporary repository (the CLS-8 scenarios); makes true CLS-8.
2. [x] Gate and Rules: `records_close_only` applies the same function to the `HEAD` content of a `roadmap` document
   after the rewrite, the existence read in the index; the Rules of `issue-workflow` drop the manual edit and propose
   the release on the printed line; verified by `close_checks` cases (close with the milestone set passes, a milestone
   with an open issue or not listing the stem refused); makes true REV-20.
3. [x] Docs of the whole issue and fixes of the `high` review: `docs/gates.md`, `docs/workflow.md`, the seeded and
   own `ROADMAP.md`; the gate also accepts the close with the milestone state put back; the roadmap path is
   resolved (`./ROADMAP.md`); only the links of the list to `issues/` count; verified by the new cases of
   `sets_the_milestone_done` and `close_checks`; makes true CLS-8, REV-20.

## Decisions
- Mode: autonomous
- Base commit: 2d3169d
- Milestones set `done`: only those that list a closed issue (1 a).

## Deliverable 1

- done: `close-issue` sets `done` the `in issues` milestones of the roadmap that list the closed issue and have
  every listed issue in `issues/done/`, through the pure `close_issue::milestones_done`, and prints each.

| check | result |
|---|---|
| tests | `just check` green (`close_issue`: `sets_the_milestone_done`, 3 cases) |
| lint | clippy -D warnings clean |
| specs | CLS-8 traced, `close-issue` accepted |
| docs | module doc of `close_issue.rs` |
| code review | see below |

red: CLS-8: `FAIL - only the milestone whose issues are all closed and that lists the closed one is set done`
Code review: no findings

not verified: none. decisions needed: none.

## Deliverable 2

- done: `records_close_only` replays `close_issue::milestones_done` on the rewritten `HEAD` content of a `roadmap`
  document, each listed issue looked up in the index git commits; the Rules of `issue-workflow` and its `details.md`
  leave the milestone state to `close-issue`; module doc of `review_gate.rs`.

| check | result |
|---|---|
| tests | `just check` green (`close_checks`: 12 cases; `skills` check of the Rules updated) |
| lint | clippy -D warnings clean |
| specs | REV-20 traced, `review-gate` accepted |
| docs | skill Rules, `details.md`, module doc |
| code review | see below |

red: REV-20: `FAIL - native lets through the close of an issue with the references and the milestone close-issue rewrote`
Code review: no findings

not verified: none. decisions needed: none.

## Deliverable 3

- done: docs of the whole issue (`docs/gates.md`, `docs/workflow.md`, `seed/ROADMAP.md`, `ROADMAP.md`) and the
  fixes of the `high` review (8 findings).

| check | result |
|---|---|
| tests | `just check` green (`sets_the_milestone_done`: `./ROADMAP.md`, a URL and a doc link in the list; `close_checks`: 13 cases) |
| lint | clippy -D warnings clean |
| specs | CLS-8 and REV-20 reworded (links to `issues/` only; with or without the milestone state) |
| docs | gates, workflow, roadmaps |
| code review | see below |

red: none (fixes of findings; the new cases are written with them)

`high` review: 8 findings
- fixed: the gate refused a close with only the reference rewrite once a milestone could be set `done` (it accepts
  both forms now); `./ROADMAP.md` never matched (resolved path); a URL or doc link in the list blocked the milestone
  (only links to `issues/` count); `exists` ran for every milestone (only when the state and the closed issue match);
  two forms of the `issues/done/` test (one `done_stem`); a missing comma in `docs/workflow.md`.
- not fixed: a `## ` line inside a fence or an HTML comment cuts a milestone (`release` and `next-issue` read the
  roadmap the same way: a roadmap grammar shared by the three is its own change); the section parser duplicated from
  `release::mark_released` (same reason).

Code review: no findings

not verified: none. decisions needed: none.
