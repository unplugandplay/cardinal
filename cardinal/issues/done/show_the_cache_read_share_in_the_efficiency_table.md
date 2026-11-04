---
rank: 58
why: 'before the exploration verdict, which reads input_tokens: cheap, from columns already collected, and makes every token verdict tell a cache read from a full-price input'
---
# Show the cache read share in the efficiency table

# Goal
The `input_tokens` column of the efficiency table adds up the uncached input, the cache reads and the cache writes
(REP-5). A cache read is billed about a tenth of an uncached input and a cache write more than one, so a change that
breaks the cache and one that grows the context look alike, and a verdict on `input_tokens` cannot tell a cheaper
session from a costlier one. The expected outcome: the efficiency table shows how much of the input came from the
cache, and the verdicts can read it.

# Findings
* `session-metrics` already prints `input_tokens`, `cache_read_tokens` and `cache_creation_tokens` per session
  (EVA-1); only the report merges them (REP-5).
* The token-waste literature names an unstable prompt prefix (a timestamp, a changing order) among the main causes
  of waste; Cardinal measures the size of the fixed context (`fixed-context`, REP-7) but not whether it is read from
  the cache.

# Approach
1. A `cache_read_share` column: per group, the median over its sessions of `cache_read_tokens` over the sum of the
   three input columns, as a whole percent.
2. Or the three input columns kept apart (medians of each), the sum staying for continuity.
3. Optionally, a cost-weighted input (cache read and write weighted by their price ratio to an uncached input): the
   ratios are prices of the provider and may change, so a column with fixed weights may age.
Then the metric for `Expected-effect:` (VRD-1) and its verdict.

# Specs
* `specs/cardinal-report.md`: amends REP-5 or adds a requirement for the new column.
* `specs/cardinal-verdicts.md`: VRD-1 gains the metric.

# Milestone
Lean agent loop

# Resources
* `specs/cardinal-report.md` (REP-5, REP-9 to REP-11: how a column was added last), `specs/cardinal-verdicts.md`.
* `crates/cardinal/src/tools/cardinal_report.rs`, `session_metrics.rs`.

# Hints
* A ratio of medians is not the median of ratios: say which one the column is.

# Tracking

Plan: validated

## Understanding
Approach 1: a `cache_read_share` column right after `input_tokens`, the median over the group's sessions of each
session's cache reads over its input sum (median of ratios), as a whole percent; `input_tokens` unchanged. Out of
scope: the three columns apart (2) and a cost-weighted input (3). The verdicts read it as an `Expected-effect:` metric.

## Plan
1. [x] Cache read share: the column in `cardinal-report`, the metric in `cardinal-changes` and the verdicts, the
   evaluate-harness skill names it; verified by the efficiency checks of `tests/cardinal_report.rs` (fixture share
   2/6 = `33%`, a group without input empty) and the metric accepted by `commit-check`; makes true REP-12, VRD-1
   (amended), REP-9 (moved after the new column). ~150 lines.
2. [x] Fixes of the `high` review of the whole issue; verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: f6f53eb4dcc114fea41f24df9d16e3600acf549b
- No count of the sessions with input next to the share: a session without input is left out of the median.

## Summary of deliverable 1
- done: `cache_read_share` after `input_tokens` in the efficiency table; `cache_read_share` accepted by
  `Expected-effect:` and read by the verdicts (its `%` dropped as for `breached_share`); evaluate-harness reads
  `input_tokens` with it.
- red: REP-12, VRD-1: `FAIL - the median share of the input read from the cache leaves out the sessions with no
  input`, `FAIL - the cache read share is the median of the shares of the sessions, empty with no input`;
  `assertion failed: cache read share` (commit-check).
- Code review: no findings
- not verified: the column on the real transcripts of this machine.

## Summary of deliverable 2
- done: update-docs over the issue (nothing else to change: `docs/architecture.md` does not list the report's columns);
  fixes of the `high` review: a verdict check on `cache_read_share`, the `%` of an efficiency cell named in the doc of
  `metric_value`, the share computed with one input sum per session, fixture comments corrected.
- red: none (the fixes add a check of a path the first deliverable already made work).
- Code review (high): 8 findings; fixed 2 to 5, 7, 8. Not fixed: 1, no `too few` for a verdict on
  `cache_read_share` from its sessions with input, as decided at the plan (a session with no input is marginal); 6, the
  share rounded in floating point while `breached_share` uses integers: a median of ratios needs fractions, and f64
  is exact far below a whole percent.
- Code review: no findings (low, on the fixes)
- not verified: the column on the real transcripts of this machine.
