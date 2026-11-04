---
rank: 87
why: 'after the issue floor of the per-issue verdicts (VRD-11), the same gap for the two other ratios of the quality table'
---
# Require enough reviews and spec lines for a ratio verdict

# Goal
VRD-9, VRD-10 and VRD-11 make a verdict `confounded` when the sample behind its metric is under a floor. The two
other ratios of the quality table (REP-6) have none: `findings_per_review` rests on `reviews` and
`doc_lines_per_spec_line` on `spec_lines`, so a group with one review gets `helped` or `hurt` from that one review.
The expected outcome: a verdict on these ratios whose group holds too small a sample reads `confounded`.

# Findings
* `too_few_values` in `crates/cardinal/src/tools/cardinal_report.rs` has a floor for `main_context_peak`, the
  advisor metrics and the per-issue metrics only.
* Left out of `require_enough_issues_for_a_per_issue_verdict` on purpose: these ratios are not per issue.

# Approach
1. Count a group whose `reviews` is below a floor as `too few` for `findings_per_review`.
2. For `doc_lines_per_spec_line`, the sample: `spec_lines` against a floor of lines, or the issues that wrote a
   spec (not counted today), or `--min-issues` on `issues`.

# Specs
* `specs/cardinal-verdicts.md`: a requirement next to VRD-11.

# Milestone
Lean agent loop

# Resources
* `specs/cardinal-verdicts.md` (VRD-6, VRD-9, VRD-10, VRD-11), `specs/cardinal-report.md` (REP-6).
* `crates/cardinal/src/tools/cardinal_report.rs` (`too_few_values`, `Floors`).

# Hints
* `too_few_values` repeats the divisor of each ratio that `quality()` computes and the metric names of
  `metric_value`: one table of (metric, numerator, divisor) read by the three would keep the floor on the right
  column (raised by the `high` review of `require_enough_issues_for_a_per_issue_verdict`).

# Tracking

Plan: validated

## Understanding
A verdict on `findings_per_review` or `doc_lines_per_spec_line` reads `confounded` when its group holds fewer issues
than `--min-issues` (the issue is the sample unit: its reviews and lines are not independent), and for
`findings_per_review` also fewer reviews than that floor. No new option, no new column of `delivered-quality`.

## Plan
1. [x] Ratio floors: one table of the quality ratios (metric, numerator, divisor, floor columns) read by `quality()`,
   `metric_value` and `too_few_values`; verified by a binary test of `cardinal-report --changes` per ratio; makes true
   VRD-12.
2. [x] High review fixes: the findings of the `high` review of `944a321..HEAD`; verified by `just check`.

## Decisions
- Mode: autonomous
- Floors: 1 a (`issues` under `--min-issues` for both ratios, plus `reviews` under it for `findings_per_review`).
- Base commit: 944a32164b35a1c5f49188f613a35e3bdfd696a7

## Deliverable 1
- done: `QUALITY_RATIOS` drives the quality ratios, their verdict values and their floors; `findings_per_review` and
  `doc_lines_per_spec_line` now need `--min-issues` issues (and reviews, for the first).
- red: VRD-12: `FAIL - a before group with fewer reviews than --min-issues confounds a findings verdict` (and the
  after-group check), the verdicts read `helped`.
- Code review: 1 finding, not fixed: no floor on `spec_lines` for `doc_lines_per_spec_line`, left out by decision 1 a (the issue is the sample unit).
- not verified: none.
- decisions needed: none.

## Deliverable 2
- done: the `high` review of the issue gave 8 findings. Fixed: the `issues` floor of `findings_per_review` tested on
  its own, the VRD-12 scenario matched to its test, `QualityRatio.divide` holds the rounding function, a unit test ties
  `QUALITY_RATIOS` to the metrics of VRD-1, a doc comment rewrapped. Not fixed: no floor on `spec_lines` and one
  `--min-issues` for reviews (decision 1 a); unchecked column names in `QUALITY_RATIOS` (the binary tests of VRD-11
  and VRD-12 read every floor, and the quality table tests every cell).
- red: none (the fixes of a review).
- Code review: no findings
- not verified: none.
- decisions needed: none.
