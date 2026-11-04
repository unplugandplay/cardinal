# Check the red run of the tests that trace a requirement

# Goal
A `// spec: ABC-n` citation proves that a test names a requirement, not that it fails without the code that meets it.
The `red:` line of `issue-workflow` step 3.6 records the failing run, but per deliverable, in prose, and nothing reads
it. Make the evidence that each traced test was seen failing checkable.

# Findings
* 19 issues of `issues/done/` hold a `red:` line, some `red: none`; it names test titles, not requirement ids.
* `check_specs.rs` counts citations; `delivered_quality.rs` reads the plan and `Code review` lines, not `red:`.

# Approach
1. Measure: over `issues/done/`, how many deliverables that traced a requirement recorded `red: none`.
2. Write the requirement ids in the `red:` line (`red: ABC-3, ABC-4: <failure>`), and have `delivered_quality.rs`
   report the traced ids with no red run.
3. Or a mutation check, heavier: revert the deliverable's non-test changes in a temporary worktree and require the
   cited tests to fail.

# Specs
* `specs/delivered-quality.md` if the measure reads the `red:` line.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 3.2 and 3.6), `files/.claude/transcripts/delivered_quality.rs`
* `files/.claude/specs/check_specs.rs`

# Tracking

Plan: validated

## Understanding
Make "each traced test was seen failing" measurable: the `red:` line names the requirement ids, and
`delivered-quality` counts the ids an issue traced and those with no recorded red run. Out of scope: the mutation
check (approach 3), refusing a commit. Measured over `issues/done/`: 32 issues add `// spec:` citations; 15 of them
hold no `red:` line, 11 hold at least one `red: none` or `not observed`, and no `red:` line names an id.

## Plan
1. [x] `delivered-quality` columns `traced` and `traced_without_red` (after `review_findings`), summed by
   `--summary` and accepted by `harness-report` (its `QUALITY` header, REP-6 sums every column); verified by
   `tests/delivered_quality.rs` and `tests/harness_report.rs`; makes true QUA-1, QUA-11 (spec set `accepted`).
2. [x] `issue-workflow` step 3.6: the `red:` line opens with the ids its test traces
   (`red: ABC-3, ABC-4: <failure>`), `none` only for a deliverable that traces none; verified by `tests/skills.rs`
   and `just check-harness` after `just sync`.
3. [x] Fixes of the `high` review: citation lines read as `check-specs` reads them, net of the removed ones, red ids
   only from the list that opens a `red:` item (continuation lines included), empty cells when git fails; verified by
   `tests/delivered_quality.rs`; makes true QUA-11 (amended).

## Decisions
- Mode: autonomous
- Base commit: 914591c7d40cd923fab39887018e09b2537e9f83
- Means: the `red:` line names the ids and `delivered-quality` counts them (approach 2), not a mutation check.
- The older issues count: their traced ids with no id in a `red:` line fall in `traced_without_red` (the baseline).

## Deliverable 1

- done: `delivered-quality` prints `traced` and `traced_without_red` after `review_findings` (QUA-11, spec
  `accepted`); `harness-report` reads the new header and sums both in its quality table.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | QUA-11 traced |
| docs | `docs/architecture.md` |
| code review | low, see below |

- red: QUA-11: `FAIL - the distinct ids of the added citations are traced, docs excluded`, `... have no red run`,
  `... a renamed test traces nothing` (and the header checks of QUA-1).
- Code review: 1 finding: `traced` with no commit would read `HEAD`; not fixed, unreachable (`issue_row` returns early when the issue has no commit).
- not verified: a `red:` line that names an id in prose (not as a traced id) hides it from `traced_without_red`;
  seen once on this repository (`install_a_git_pre_commit_review_gate`, 10 traced, 9 without red).
- decisions needed: none

## Deliverable 2

- done: `issue-workflow` step 3.6 asks for `red: ABC-3, ABC-4: <failure>`; synced to `.claude/`.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | no change (skill text, QUA-11 reads it) |
| docs | the skill itself |
| code review | low, see below |

- red: none (no requirement traced): `FAIL - issue-workflow: missing '`red: ABC-3, ABC-4: <failure>`'`.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3 (fixes of the high review)

- done: QUA-11 amended and met: a citation is a line opening with `// spec:` (no string, no prose); the ids removed
  by the issue's diffs are subtracted (moved, split or extended citations); a red id is one of the list opening a
  `red:` item, continuation lines included (`covered:`, `red: none (ABC-3)` and the failure text name none); the
  columns are empty when git cannot show the diffs; `-G// spec:` narrows the diff; `is_req_id` shared with
  `check-specs` (three capitals). On this repository no done issue has a traced id with a red run (the old
  `install_a_git_pre_commit_review_gate` 10/9 was the `covered:` match).

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy clean |
| specs | QUA-11 amended, traced |
| docs | `docs/architecture.md` |
| code review | low, see below |

- red: QUA-11: `eps.traced: expected "3", got Some("5")`, `eps.traced_without_red: expected "1", got Some("3")`,
  `eta.traced: expected "1", got Some("2")`.
- High review: 9 findings, 8 fixed here; the 9th (QUA-6 says its section ends "as in QUA-3", which would stop at
  `## Plan`, while the code ends it at `# `) is outside this issue: filed as
  `issues/done/align_the_tracking_section_end_of_qua_6.md`, queued second.
- Code review: 1 finding, then no findings on the fix: the removed citations were pooled over the issue, so a citation one commit adds and a later
  one rewrites traced nothing; fixed, subtracted commit by commit (red: `eta.traced: expected "1", got Some("0")`).
- not verified: the empty cells when `git show` fails (no fixture makes it fail).
- decisions needed: none
