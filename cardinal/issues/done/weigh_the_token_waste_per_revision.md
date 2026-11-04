---
rank: 118
why: 'right after the token waste rule, whose verdict it completes; before the advisor comparison, which needs no waste figure'
---
# Weigh the token waste per revision

# Goal
Rule `oversized-result` of `rule-breaches` (BRE-19) flags each tool result over 25,000 bytes with its size as
`detail`, but only its `breached_share` reaches a verdict: a change that halves the oversized bytes while each
session keeps one large result gets `no change`. And the rule is no instruction the agent breaks: the skills that
read the rules summary take a rule broken in a noticeable share of the sessions for a hook or rewording candidate,
which this one will always be. The expected outcome: a waste figure per revision, in bytes, that a change can expect
to move, and the waste rule kept out of the hook candidates.

# Findings
* Over the 200 local sessions with a Cardinal revision, `oversized-result` applied in 200, breached in 82, 119
  breaches; over all 1611 transcripts, the results over 25 KB weigh 6.4 MB of 67.3 MB of results.
* `audit-instructions` step 2 and `evaluate-harness` send a rule whose `breached_share` stays high to the audit as a
  hook candidate; the `--summary` of `rule-breaches` (BRE-3) has no bytes column.
* The `high` review of the waste rule raised both points.

# Approach
1. A `bytes` column in the `--summary` of `rule-breaches` (the sum of a numeric `detail`), read by the rules table of
   `cardinal-report`, and an `Expected-effect:` metric on it (`oversized_bytes_per_session`, say).
2. Or a column of `session-metrics` (the bytes of the results over the threshold per session), so the efficiency
   table of `cardinal-report` carries it, and the rule leaves `rule-breaches`.
3. In either case, name the waste rules apart in `audit-instructions` and `evaluate-harness`: a measure, not a
   hook candidate.

# Specs
* `specs/rule-breaches.md` (BRE-3, BRE-19), `specs/cardinal-report.md`, `specs/cardinal-verdicts.md` (VRD-1).

# Milestone
Lean agent loop

# Resources
* `issues/done/detect_token_waste_in_the_transcripts.md`: the measure and the rule.
* `crates/cardinal/src/tools/rule_breaches.rs`, `session_metrics.rs`, `cardinal_report.rs`.
* `files/.claude/skills/audit-instructions/SKILL.md`, `files/.claude/skills/evaluate-harness/SKILL.md`.

# Hints
* Approach 2 changes the columns of `session-metrics`, which `cardinal-report` reads by header: check REP-3.

# Tracking

Plan: validated

## Understanding
Approach 1: the waste stays a rule of `rule-breaches`, whose summary gains a `bytes` column filled for the waste
rules only; the rules table of `cardinal-report` sums it and gives `bytes_per_session` (a mean: the median is 0), which
an expectation names as `bytes_per_session:<rule>`. The skills read a filled `bytes` as a measure, not a hook
candidate. Out of scope: moving the waste to `session-metrics` (approach 2), other waste patterns.

## Plan
1. [x] Bytes per rule: `bytes` in the `--summary` of `rule-breaches`, `bytes` and `bytes_per_session` in the rules
   table of `cardinal-report`; verified by unit tests of both tools; makes true BRE-20, REP-13.
2. [x] Waste metric: `bytes_per_session:<rule>` accepted by `commit-check` and `cardinal-changes`, given a verdict by
   `cardinal-report --changes`; verified by tests of the three; makes true VRD-13.
3. [x] Skills: `audit-instructions` and `evaluate-harness` name a rule with a filled `bytes` a waste measure, read
   through `bytes_per_session`, never a hook or rewording candidate; `just sync`; verified by `just check-cardinal`.
4. [x] Docs and `high` review fixes of the whole issue.

## Decisions
- Mode: autonomous
- Question 1: a (a `bytes` column in `rule-breaches`).
- Base commit: 4d941fb68312e21f929c41b47990c6d3d4c8a45d

## Deliverable 1

- done: `bytes` closes the `--summary` of `rule-breaches` (filled for `oversized-result` only); the rules table of
  `cardinal-report` adds `bytes` and `bytes_per_session`; `rule-breaches` and `cardinal-report` specs back to
  `accepted`.
- red: BRE-20, REP-13: the summary had no `bytes` column (`oversized-result	r1	1	1	3	0`), and `cardinal-report`
  refused the new summary header.
- Code review: no findings.
- not verified: the summary on the local transcripts (the tests use fixtures).

## Deliverable 2

- done: `bytes_per_session:<rule>` accepted by `commit-check` and `cardinal-changes`, read from the rules table by the
  verdicts of `cardinal-report`; `cardinal-verdicts` spec back to `accepted`.
- red: VRD-13: `cardinal-changes` dropped the line `Expected-effect: bytes_per_session:oversized-result down` as an
  unknown metric.
- Code review: no findings.

## Deliverable 3

- done: `audit-instructions` and `evaluate-harness` take a rule whose `bytes` is filled for a waste measure, read
  through `bytes_per_session`, never a hook or rewording candidate; synced.
- red: none (skill wording, verified by `just check-cardinal`).
- Code review: no findings.

## Deliverable 4

- done: `docs/usage.md` and `docs/architecture.md` name the waste bytes; `bytes_per_session:<rule>` accepted for a
  waste rule only (VRD-13); VRD-1 and VRD-5 name the metric; the waste rule name is one constant, its bytes summed
  only for the waste rules.
- red: VRD-13: `commit-check` accepted `Expected-effect: bytes_per_session:no-verify down`.
- High review: 8 findings; fixed: a non-waste rule accepted, VRD-1 and VRD-5 silent, bytes summed for every rule,
  the rule name repeated, `docs/usage.md` stale. Not fixed: no floor for the mean of the bytes (each group already
  holds `--min-sessions` sessions, and `evaluate-harness` takes a verdict for a trend only when the next groups keep
  it); an older 6-column summary refused (the summaries are written afresh by each evaluation, as REP-3 wants); an
  empty `bytes` row emptying the group's sum (REP-13, the same rule as the quality sums).
- Code review: no findings.
