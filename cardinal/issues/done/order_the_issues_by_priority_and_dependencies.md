---
priority: high
why: 'the claim and the batch of issues both need the set of ready issues it lists'
---
# Order the issues by priority and dependencies

# Goal
The `rank` of each issue file (ADR 0019) packs three things into one number: the order of the milestones (already
written in the `roadmap` documents), the dependencies between issues (written in prose in `why` or the Hints, checked
by nothing), and the priority. The user finds it rigid: a new issue must be slotted between two neighbours, a
re-ranking rewrites every open issue, and nothing tells which issues are ready to start in parallel. The outcome:
each issue declares a `priority` (`high`, `medium`, `low`) and the issues it `depends_on`, and `next-issue` derives
the order from them and from the milestone order of the roadmap, listing the ready issues apart from the blocked ones.

# Findings
* `why` of the open issues, 2026-10-31: `claim_an_issue_from_the_session_that_works_it` says "after the queue drop",
  a dependency that only the agent reading the text can honour; `next-issue` would offer the claim issue first if
  its rank were lower.
* `skill:prioritize-issues` step 4 ranks by hard dependencies, then milestone order, then value, then cost: the first
  two are facts that a tool can compute, re-derived by hand at each re-ranking.
* `skill:create-issue` step 5 picks a rank halfway between neighbours and never edits another issue; ties are
  reported by `next-issue` (NXT-3) and settled by a re-ranking.
* `drive_several_issues_in_a_row_from_one_session` and `claim_an_issue_from_the_session_that_works_it` both need
  the set of issues that can start now, which the rank does not give.

# Approach
1. Front matter of an open issue: `priority: high | medium | low`, optional `depends_on: [<stem>, ...]`, `why` kept
   as the reason for the priority. `rank` goes away (or stays an optional tie-breaker, to decide at step 1 of the
   workflow; the proposal is to drop it and add it back only if the tie-breaker below proves not enough).
2. `next-issue` order: the ready issues first (every `depends_on` is in `issues/done/`), sorted by the order of
   their milestone in the `roadmap` documents (an issue of no milestone after the milestones), then priority, then
   a stable tie-breaker (age of the file in git, or stem); then the blocked issues, each with what blocks it. It
   reports an unknown stem in `depends_on`, a cycle, a missing or invalid `priority`.
3. `check-issue` validates the keys (an issue split or renamed leaves `depends_on` that cite a stem no longer there).
4. Skills: `create-issue` sets a priority and the dependencies instead of a rank; `prioritize-issues` reviews the
   priorities and dependencies (and guards against every issue drifting to `high`) instead of renumbering.
5. Format: a change the format 5 `cardinal` would misread, so `cardinal-format` and `FORMAT` rise to 6 (ADR 0014),
   a `migrations/6.toml` (ADR 0017), and an ADR that supersedes 0019. Converting the ranks of a consumer: either a
   `next-issue --convert` (ranks to priorities by thirds, no dependency) or a one-time pass of
   `skill:prioritize-issues` by the agent the migration warns; the second costs no code.

Estimated size: ~1,500 to 2,500 changed lines (the `next-issue` code and tests, two specs, three skills, the docs,
an ADR, a migration), 4 to 5 deliverables; cut it if step 1 of the workflow finds more.

# Specs
* `specs/next-issue.md`: NXT-1, NXT-2, NXT-3 amended (order, missing priority, ties), new requirements for the
  dependencies (ready and blocked, unknown stem, cycle) and the milestone order.
* `specs/close-issue.md`: the next issue it offers follows the new order.
* `specs/parallel-issues.md`: the first free issue (PAR-4) among the ready ones.

# Milestone
Longer autonomy

# Resources
* `docs/adr/0019-each-issue-file-carries-its-rank.md`, `docs/adr/0014-*`, `docs/adr/0017-*`.
* `crates/cardinal/src/tools/next_issue.rs`, `crates/cardinal/src/tools/check_issue.rs`,
  `crates/cardinal/src/tools/close_issue.rs`.
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`,
  `files/.claude/skills/issue-workflow/details.md` ("Split an issue"), `files/cardinal.just`.
* `ROADMAP.md` (milestone order and the `seed/ROADMAP.md` text that names the `rank`), `docs/workflow.md`,
  `docs/usage.md`, `docs/architecture.md`.

# Hints
* Start after `drop_the_format_4_issue_queue`: the format 4 code of `next_issue.rs` (NXT-4 to NXT-6) is then gone.
* The milestone order must be read from the `roadmap` documents of `.claude/cardinal.toml`, which a consumer may
  not have: with no roadmap, the order is priority then tie-breaker.
* An issue named under a milestone in the roadmap but whose `# Milestone` section disagrees: pick one source of truth.

# Tracking

Plan: validated

Done: every deliverable committed; the follow-up is `issues/done/warn_when_starting_a_blocked_issue.md`.

## Understanding
Replace the `rank` by `priority` and `depends_on` in each issue's front matter; `next-issue` derives the order (ready
by milestone, priority, tie-breaker; then blocked) and `check-issue` validates the keys of a new issue. Out of scope:
`next-issue --convert` (the migration warns on `rank:`, `skill:prioritize-issues` converts once), dependencies on
anything but an issue. The `# Milestone` section of the issue is the source of its milestone, the roadmap headings of the order.

## Plan
1. [x] Order by priority, dependencies and milestone: `next_issue.rs` reads `priority`, `depends_on`, `# Milestone`
   and the roadmap headings, lists `Blocked:` lines, reports unknown stems, cycles, unknown milestones; `close-issue`
   offers among the ready ones; format 6 (`cardinal-format`, `FORMAT`), `migrations/6.toml` (`rank:` name, seeded
   `ROADMAP.md` line), `seed/ROADMAP.md`, ADR 0021, the open issues of this repo converted; verified by
   `tests/next_issue.rs` and the CLS-7 test; makes true NXT-1, NXT-2, NXT-7 to NXT-14, PAR-4, CLS-7. ~900 lines.
2. [x] `check-issue` validates the keys: priority, `depends_on` syntax, self and unknown stems, sharing the
   front matter reader of `next_issue.rs`; verified by `tests/check_issue.rs`; makes true CKI-1 to CKI-3. ~250 lines.
3. [x] Skills and docs: `create-issue` sets a priority and the dependencies, `prioritize-issues` reviews them (guard
   against all `high`, one-time conversion of the ranks), `issue-workflow` (`next-issue` rule, "Split an issue"),
   `cardinal.just` and the tool's `about`, `docs/workflow.md`, `docs/architecture.md`, `README.md`, `CLAUDE.md`,
   `ROADMAP.md`, ADR 0019 status; `just sync`; verified by `just check` and `just check-cardinal`. ~500 lines (copies counted twice).

4. [x] Fixes of the `high` review of the issue; verified by `tests/next_issue.rs`.

## Decisions
- Mode: autonomous
- Base commit: bdb358bef3239a698cf98508fd68310a2e77aa5a
- Tie-breaker: the oldest first by the commit that added the file, then stem.
- `MIN_FORMAT` stays 5: the issues of a format 5 project are reported without a priority (exit 1).
- `ROADMAP.md`: *Lean agent loop* moves after *Longer autonomy* (its last issue waits on sessions).
- This repo's issues: `claim` medium and `drive` low, both `depends_on` this issue; `judge` low.
- `migrations/6.toml` names `rank:` so the update lists the ranked issues.

## Deliverable 1
- done: `next-issue` orders the ready issues by milestone, priority, age, then lists `Blocked:`; format 6 and `migrations/6.toml`; this repo's issues, roadmap and replay fixture converted.
- checks: `just check` ok; deliverable-size 776, within; docs of the skills and `docs/` left to deliverable 3 as planned.
- red: NXT-1, NXT-2, NXT-7, NXT-8, NXT-9, NXT-10, NXT-11, NXT-12, NXT-13, NXT-14: `the_priority_orders_the_issues` printed `Next: issues/a.md` (the stem order of unranked issues).
- NXT-9 reworded at implementation: a cycle is reported once per group of issues that depend on each other, its stems sorted.
- The test fixtures declare the format 6 (`tests/common/mod.rs`), as at the format 5.

## Deliverable 2
- done: `check-issue` reports a missing or unknown priority, a `depends_on` that is not a list, names the issue itself or an unknown stem, reusing the reader of `next_issue.rs`.
- red: CKI-1, CKI-2: `check-issue` printed nothing and exited 0 on an issue with no priority and on `depends_on: [b, gone]`.

## Deliverable 3
- done: `create-issue` writes the priority and dependencies in the template, `prioritize-issues` reviews them (rare `high`, conversion of a `rank`), `issue-workflow` and the docs name the new order; synced.
- red: none (skills and docs; `skills.rs` keeps the commit passage of `prioritize-issues`).

## Deliverable 4 (high review)
- Code review: 10 findings.
  - fixed: age read with `--no-renames` and `core.quotePath=false` (a renamed issue keeps an age), the template heading `# Milestone (optional; ...)` read, `depends_on` deduplicated, the `check-issue` message names the real directory, `front_matter` private and `offer` takes the free list; `drive_...` names its milestone `Longer autonomy`.
  - not fixed: `rank:` may warn on a consumer's own `rank:` field (accepted at the stop of step 2, question 5); `No ready issue` also shown when the blocker is unknown or a cycle (stderr names it, exit 1); `claim_...` has no milestone (none in the roadmap: the user's call).
  - new issue: `issues/done/warn_when_starting_a_blocked_issue.md` (`start-issue` opens a blocked issue).
- red: NXT-1: without `--no-renames`, `the_oldest_issue_breaks_a_tie` listed the renamed `z` after the untracked `0`.
