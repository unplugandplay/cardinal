# Split an issue too large for one session

# Goal
An issue that cannot be delivered in one session with diffs a `low` review reads whole costs far more than the sum
of its parts: context grows with each deliverable, reviews see part of the diff, and the `high` review comes too late.
Give `create-issue` and `prioritize-issues` a size limit beyond which an issue is split before it is started.

# Findings
* `ship_the_harness_scripts_as_prebuilt_binaries` (closed 2026-10-27): 7 planned deliverables plus one of fixes,
  about 7,500 lines of scripts and tests moved, the launcher, a release workflow and CI, in one session.
* The `low` review saw only part of a 73 kB diff (deliverable 4) and was run twice when new files were untracked;
  the parts it missed were compared by hand.
* The final `high` review found 10 problems over the whole issue, which became an extra deliverable; several were in
  deliverable 1 and would have been cheaper to fix there.
* Natural cut points existed: launcher and release, hooks, issue tools and checkers, transcript readers, cleanup.

# Approach
1. Measure on `issues/done/`: deliverables per issue, diff size per deliverable commit, and the reviews whose report
   says they saw part of the diff (`harness run delivered-quality` gives deliverables and review findings).
2. Pick a limit from that (deliverables, or an estimated diff size), and where it is checked: `create-issue` when the
   issue is written, `issue-workflow` step 1 when the plan exceeds it (propose the split at the step 2 stop),
   `prioritize-issues` for the open ones.
3. Define how a split issue is written: one issue per cut, ordered in the queue, the dependencies in `# Hints`.

# Resources
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md` (steps 1 and 2)
* `issues/done/ship_the_harness_scripts_as_prebuilt_binaries.md` (its `# Tracking`)

# Hints
* A limit in deliverables alone misses a deliverable that moves thousands of lines; the review coverage is the cost
  that matters.

# Tracking

Plan: validated

## Understanding
Set a size limit, measured on the closed issues, that `create-issue`, `issue-workflow` (steps 1 and 2) and
`prioritize-issues` check, and the way an issue over it is split. Skill text only: no checker, no spec (no
capability of the tool changes). Out of scope: splitting the open issues of the queue (done by
`prioritize-issues` once this lands).

## Measure
57 issues of `issues/done/`; lines changed per deliverable commit (added + removed, `issues/` and `Cargo.lock`
excluded):

| measure | value |
|---|---|
| issues over 3,000 lines in all | 1 (`ship_the_harness_scripts_as_prebuilt_binaries`: 19,654 in 8 commits) |
| next largest issues | 2,782, 2,557, 2,241 lines |
| deliverable commits over 1,000 lines | 9: 6 of `ship_...` (1,037 to 8,035), then 2,229 (mostly deletions), 1,747, 1,217 |
| issues over 6 deliverables | 1 (`ship_...`, 8); then 6 (two issues) |
| reviews recording a partial view | 1: deliverable 4 of `ship_...` (2,790 lines, 173 kB raw diff) |
| reviews run twice (untracked files unseen) | 2, both in `ship_...` (deliverables 3 and 6) |

Deliverables 3 (3,073 lines) and 6 (8,035) of `ship_...` recorded no partial view; a review over 2,000 lines likely
did not read it all either, it just did not say. Moved code counts twice unless the move is a pure `git mv`
(content unchanged), which the review reads as renames.

## Plan
1. [x] Size limit and split: `issue-workflow` step 1 estimates each deliverable's lines and checks the limit
   (at most ~1,000 changed lines per deliverable, ~3,000 and 6 deliverables per issue; a pure `git mv` move
   counts as renames), step 2 proposes the split when exceeded; `details.md` says how a split issue is written
   (one issue per cut through `create-issue`, the original file keeps the first cut, the order in the queue, the
   dependencies in `# Hints`); `create-issue` checks the Approach against the limit and splits before writing;
   `prioritize-issues` flags an open issue over it and proposes its split; verified by `just check` (links,
   harness copies) and a reread of the three skills; makes true no spec requirement (skill text).
2. [x] Fixes of the `high` review: cut until each part is under the limit, re-check the real diff at step 3.5,
   re-check a resumed plan at step 0, a moved file edited later counts twice in the issue total, `create-issue`
   proposes the cuts before writing them, the cuts skip the queue step of `create-issue`, no split written in a
   linked worktree; verified by `just check` and a reread.

## Decisions
- Mode: autonomous
- Base commit: 94a4c18fcb32c817f36c35df67135ec5703b5a33
- Limits: at most ~1,000 changed lines per deliverable, ~3,000 lines and 6 deliverables per issue.
- A pure `git mv` (content unchanged) counts as renames; any other move counts twice.
- A split keeps the first cut in the original file; the other cuts are new issues.
- Fixed values in the skills, no `.claude/harness.toml` key.

## Deliverable 1
- done: `issue-workflow` step 1 states the size limit and estimates each deliverable, step 2 proposes the cuts;
  `details.md` "Split an issue" says how; `create-issue` and `prioritize-issues` check the limit and split.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| harness copies (`just sync`) | synced |
| docs | `docs/architecture.md` does not describe the workflow steps: unchanged |
| code review | no findings |

- red: none (skill text, no test).
- not verified: a real split by an agent; the first one is the next `prioritize-issues` run.
- Code review: no findings.

## High review
- Code review: 9 findings (`high`, `94a4c18..6ecc7bc`). Fixed in deliverable 2: 1 (cut in two), 2 (create-issue
  split unasked), 3 (queue written twice), 4 (moves in the issue total), 5 (split in a linked worktree), 6 (no
  re-check of the real diff), 9 (resumed plan not checked). Not fixed: 7 (`issues/` and lock files left out of the
  count: their lines are mechanical and within the `~` of the limit) and 8 (the fix deliverable of the `high`
  review over the 6 cap: the 6 measured includes such deliverables).

## Deliverable 2
- done: the 7 fixes above in `issue-workflow` (steps 0, 1, 3.5, `details.md`) and `create-issue`.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| harness copies (`just sync`) | synced |
| docs | unchanged |
| code review | no findings |

- red: none (skill text, no test).
- not verified: a real split by an agent.
- Code review: no findings.
