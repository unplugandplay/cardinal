---
rank: 70
why: 'after the tokens per issue, before the exploration verdict: that verdict reads a per-issue metric and must not rest on one issue'
---
# Require enough issues for a per-issue verdict

# Goal
A verdict (VRD-6) is `confounded` when a group has fewer sessions than `--min-sessions`, and VRD-9 and VRD-10 apply
the same floor to the sessions a peak or an advisor metric is computed from. The per-issue metrics of the quality
table (`fixes_after_per_issue`, `input_tokens_per_issue`, `output_tokens_per_issue`, REP-6) have no such floor: a
group of 10 sessions that closed one issue gets `helped` or `hurt` from that one issue. The expected outcome: a
per-issue verdict whose group holds too few closed issues reads `confounded`.

# Findings
* The `high` review of `report_the_tokens_spent_per_closed_issue` raised it: `too_few_values` in
  `crates/cardinal/src/tools/cardinal_report.rs` has a floor for `main_context_peak` and the advisor metrics only.
* The quality table gives `issues` per group, the sample of these means.

# Approach
1. Count a group whose `issues` is below a floor as `too few` for these metrics, as VRD-9 does with `peak_sessions`.
2. The floor: `--min-sessions`, or a separate `--min-issues` (an issue spans several sessions, so fewer issues than
   sessions is the norm).

# Specs
* `specs/cardinal-verdicts.md`: a new requirement next to VRD-9 and VRD-10.

# Milestone
Lean agent loop

# Resources
* `specs/cardinal-verdicts.md` (VRD-6, VRD-9, VRD-10), `specs/cardinal-report.md` (REP-6).
* `crates/cardinal/src/tools/cardinal_report.rs` (`too_few_values`, `metric_value`).

# Tracking

Plan: validated

## Understanding
A per-issue verdict (`fixes_after_per_issue` on `issues`, the two token ratios on `token_issues`) reads `confounded`
when either group holds fewer issues than a floor, as VRD-9 does for the peak. Out of scope: a floor on `reviews`
(`findings_per_review`) or `spec_lines` (`doc_lines_per_spec_line`).

## Plan
1. [x] Issue floor: `too_few_values` reads the group's `issues` / `token_issues` cell of the quality table against
   `--min-issues` (REP-3 option, refused out of 1..=1000); the token verdict test passes `--min-issues 2`; verified by
   a new verdict test (2 issues before, 6 after: `confounded`) and a refusal check; makes true VRD-11, REP-3. Docs:
   `docs/usage.md`, `docs/architecture.md`, `evaluate-harness` skill (`files/` then `just sync`). ~150 lines.

## Decisions
- Mode: autonomous
- Base commit: 60595c6
- Floor: a separate `--min-issues`, default 5 (an issue spans several sessions).
- A floor on `reviews` and `spec_lines` goes to an issue of its own.

## Deliverable 1
- done: `too_few_values` reads `issues` / `token_issues` of the quality table against `--min-issues` (default 5).

| check | result |
|---|---|
| tests | `just check` green |
| specs | VRD-11 traced, `cardinal-verdicts.md` back to `accepted` |
| docs | `docs/usage.md`, `docs/architecture.md`, `evaluate-harness` |

- red: VRD-11, REP-3: `FAIL - a group with fewer issues of known tokens than --min-issues (5) confounds a token verdict` (`unknown argument --min-issues`)
- Code review: no findings
- not verified: none
- decisions needed: none
- follow-up: `issues/done/require_enough_reviews_and_spec_lines_for_a_ratio_verdict.md` (floor on `reviews`, `spec_lines`).

## Deliverable 2: fixes of the `high` review
- done: a check with the before group alone under the `issues` floor; a `floor` helper for both options
  (`ok_or_else`); `docs/usage.md` names the issues of known tokens; doc comment wrapped at 120; this tracking fixed.

| check | result |
|---|---|
| tests | `just check` green |
| specs | unchanged, traced |
| docs | `docs/usage.md` |

- red: none (a test strengthened, no new behaviour)
- Code review (high): 7 findings: 6 fixed; the single table of ratios and their divisors (`quality()`,
  `metric_value`, `too_few_values`) left to the follow-up issue, as a hint, since it is a refactor across the three.
- Code review (low): no findings
