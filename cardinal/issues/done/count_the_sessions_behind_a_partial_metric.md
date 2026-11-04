# Count the sessions behind a metric that some sessions lack

# Goal
The median `main_context_peak` of `harness-report` (REP-9) leaves out the sessions whose cell is empty (archived
before the column, EVA-17), but the `sessions` count of the group and its `too few` flag (REP-4), which the verdicts
read (VRD-6), count every session. A verdict on `main_context_peak` can then rest on one or two sessions and still
be `helped` or `hurt`. The expected outcome: a verdict on such a metric is `confounded` (or `no data`) when the
sessions that have it are fewer than `--min-sessions`.

# Findings
* Raised by the `high` review of `issues/done/delegate_the_exploration_of_the_issue_workflow_to_a_subagent.md` (commits
  `e311105`, `657bf10`).
* Only `main_context_peak` can be empty in a session row today; every other efficiency column is always filled.
* The archived rows lose their transcripts over time, so the groups before the column was added hold the most
  empty cells: the before group of the change that names `main_context_peak down` is the one at risk.

# Approach
1. Show next to the median the count of the sessions it was computed from (e.g. `main_context_peak` as `n=…` in
   a column of its own), so the reader sees the sample.
2. In the verdict, treat a group as `too few` for a metric when the sessions holding a value of that metric are
   fewer than `--min-sessions`.
Measure first how many archived rows lack the peak; if the archive is young, 1 may be enough.

# Specs
* `specs/harness-report.md` (REP-9), `specs/harness-verdicts.md` (VRD-6).

# Resources
* `crates/harness/src/tools/harness_report.rs` (`main_context_peak`, `verdicts`).
* `specs/harness-report.md`, `specs/harness-verdicts.md`, `specs/evaluation.md` (EVA-16, EVA-17).

# Tracking

Plan: validated

## Understanding
Show the sample behind the median `main_context_peak` (`peak_sessions`, REP-10) and make a verdict on that metric
`confounded` when either group has fewer sessions with a peak than `--min-sessions` (VRD-9). Measured on
2026-10-28: 298 sessions from the transcripts, 0 without a peak, no archive in use; both steps are kept since the
second one is small and is the outcome the goal asks for.

## Plan
1. [x] Count and guard the peak sample: `peak_sessions` column after `main_context_peak`, verdict on
   `main_context_peak` confounded when a group's `peak_sessions` is under `--min-sessions`; verified by tests of
   `harness_report` through the binary; makes true REP-10, VRD-9 (~120 lines).
2. [x] Fixes of the `high` review: the guard counts the peaks from the group rather than parsing the printed cell;
   VRD-9 states `no data` when a group has no peak at all; REP-5 names `peak_sessions`; verified by a
   `harness_report` check for the `no data` case.

## Decisions
- Mode: autonomous
- Scope: both steps (answer `1 a`)
- Base commit: 3758a7d6502f9b4db77287412fe6d3a39f67830c

## Deliverable 1

done: `peak_sessions` column in the efficiency table; a `main_context_peak` verdict is `confounded` when either group
has fewer sessions with a peak than `--min-sessions`; `evaluate-harness` and `docs/architecture.md` say so.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | REP-10, VRD-9 traced, both specs accepted |
| docs | architecture, evaluate-harness skill |
| code review | see below |

red: REP-10, VRD-9: `FAIL - the sessions with a peak are counted next to its median`, `FAIL - a main context peak held by fewer sessions than the threshold confounds, before or after`

Code review: no findings

not verified: none

decisions needed: none

## Deliverable 2

done: fixes of the `high` review of `3758a7d..c3a37e2`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | VRD-9 amended (`no data` case), REP-5 names `peak_sessions` |
| docs | architecture |
| code review | see below |

red: none (the `no data` case already held; the check pins it)

High review: 6 findings
1. guard read the printed cell back: fixed, counted from the group;
2. stale comment on the before group: fixed;
3. VRD-9 and the architecture promised `confounded` for a group with no peak: fixed, `no data`, with a check;
4. hard-coded `main_context_peak` special case: not fixed, only that column can be empty (Findings);
5. REP-5 column list missed `peak_sessions`: fixed;
6. peaks computed for a `too few` group: not fixed, VRD-5 computes the values of a `too few` group too.

Code review: no findings

not verified: none

decisions needed: none
