---
rank: 25
why: 'first: nothing on the advisor can be judged before its use is counted; the advisor issues after it rest on it'
---
# Count the advisor calls of each session

# Goal
Claude Code can pair the session's model with a stronger advisor model (`advisorModel` setting, `/advisor`,
`--advisor`), which the executor consults mid-turn. The evaluation of Cardinal cannot tell today whether a session had
an advisor, how often it consulted it, nor what it cost. The outcome: `session-metrics` gives per session the advisor
model and its calls (and its tokens if the transcript has them), so the harness changes that steer the advisor can get
a verdict.

# Findings
* Measured on the user's machine (2026-10-30, counts only): the `assistant` lines of the transcripts carry a top-level
  `advisorModel` field (about 44,000 lines with an Opus advisor).
* No content block named `advisor` nor typed `advisor_*` was found in any transcript, sub-agents included; the
  `usage.iterations[]` of the assistant messages hold only `type: "message"` entries, no `advisor_message`. Unknown
  whether Claude Code never writes the call or the advisor was never called (the main model was mostly the same Opus).
* The API returns the advisor's usage in `usage.iterations[]` (`type: "advisor_message"`), billed at the advisor's
  rate, and its result as an `advisor_tool_result` or `advisor_redacted_result` block.
* `session-metrics` (EVA-1) has `models` but no advisor column; EVA-3 already folds the sub-agent transcripts in.

# Approach
1. Measure first: run a short session with an advisor that is stronger than the main model (e.g. a Sonnet session with
   an Opus advisor) and a prompt that asks for an advisor call, then read what its transcript records.
2. Add the columns that the transcript supports: `advisor_models` (distinct `advisorModel`, as `models`),
   `advisor_calls`, and `advisor_tokens` if the usage iterations carry them; an absent field is an empty cell, not 0.
3. Carry them into the efficiency table of `cardinal-report` and, if a count is reliable, into the metrics of
   `Expected-effect:`.
If the transcript records no call at all, say so in the spec's non-goals and keep only `advisor_models`.

# Specs
* `specs/evaluation.md`: amends EVA-1, new requirements for the advisor columns.
* `specs/cardinal-report.md`: amends REP-5; `specs/cardinal-verdicts.md` VRD-1 if a metric is added.

# Milestone
Advisor

# Resources
* `crates/cardinal/src/tools/session_metrics.rs`, `specs/evaluation.md`, `specs/cardinal-report.md`.
* Advisor tool: https://platform.claude.com/docs/en/agents-and-tools/tool-use/advisor-tool
* Advisor in Claude Code: https://code.claude.com/docs/en/advisor
* Article: https://claude.com/blog/the-advisor-strategy

# Hints
* An advisor less capable than the main model is not attached to the request: the setting alone does not prove an
  advisor was available; read the pair.
* Opus 5.x, Sonnet 5.5, Haiku 5.5, Fable and Mythos advisors return an encrypted result: count the call, never its text
  (EVA output holds no message text).

# Tracking

Plan: validated

## Understanding
Measured on 2026-10-30 (Claude Code 2.1.312, Sonnet + Opus advisor, and Opus + Opus): a call is a `server_tool_use`
block named `advisor` followed by an `advisor_tool_result` (`advisor_redacted_result` content); its tokens are an
`advisor_message` entry of `usage.iterations` (with its `model`), absent from the top-level `usage`. Opus + Opus is
attached too. So all the columns are supported; the 44,000 earlier lines simply held no call. Out of scope: whether
a set advisor was attached (spec non-goal).

## Plan
1. [x] session-metrics advisor columns: `advisor_models`, `advisor_calls`, `advisor_input_tokens`,
   `advisor_output_tokens` after `main_context_peak`, empty with no advisor set, and the archive upgrade of the older
   headers; verified by tests of `crates/cardinal/tests/session_metrics.rs`; makes true EVA-1, EVA-18, EVA-19, EVA-20.
2. [x] cardinal-report advisor columns of the efficiency table; verified by tests of
   `crates/cardinal/tests/cardinal_report.rs`; makes true REP-5, REP-11.
3. [x] The advisor metrics of `Expected-effect:` and their `too few` rule; verified by tests of `commit-check` and
   `cardinal_report.rs`; makes true VRD-1, VRD-10.

## Decisions
- Mode: autonomous
- Base commit: 5ba0c86f53022bedc9e3fd3192b5d09773080505
- Answers 1 a, 2 a, 3 a: two token columns (input, output); empty cells with no advisor set, 0 when set and not called; the advisor metrics join `Expected-effect:`.

## Deliverable 1
- done: `session-metrics` prints `advisor_models`, `advisor_calls`, `advisor_input_tokens`, `advisor_output_tokens`;
  an archive written before them (or before `main_context_peak`) is upgraded by column name.
- red: EVA-1, EVA-18, EVA-19, EVA-20: every check failed, `header names every column` first (the header had no
  advisor column).
- Code review: no findings
- not verified: none.

## Deliverable 2
- done: the efficiency table of `cardinal-report` gives `advisor_models`, `advisor_sessions` and the medians of the
  advisor calls and tokens over the sessions that set one; `evaluate-harness` reads them with `advisor_sessions`.
- red: REP-5, REP-11: `the report prints an efficiency table with its columns` and the three advisor checks failed
  (no advisor column).
- Code review: no findings
- not verified: none.

## Deliverable 3
- done: `advisor_calls`, `advisor_input_tokens` and `advisor_output_tokens` are `Expected-effect:` metrics; a group
  with fewer `advisor_sessions` than `--min-sessions` makes their verdict `confounded`; the three specs are back to
  `accepted`.
- red: VRD-1, VRD-10: `commit-check` refused `Expected-effect: advisor_calls up`; `advisor metrics set by fewer
  sessions than the threshold confound` failed (the verdict was `helped`).
- Code review: no findings
- not verified: none.

## Deliverable 4: docs and high review of the issue
- done: `skill:update-docs` over `5ba0c86..HEAD` (nothing stale left); `/code-review high` fixes: a change of the most
  frequent `advisor_models` confounds a verdict on an advisor metric (VRD-10 amended); the efficiency row reads the
  advisor calls once.
- red: VRD-10: `a change of the most frequent advisor confounds an advisor metric, not another one` failed (the
  verdict was `helped`).
- Code review (high): 6 findings.
  1. The peak is doubled on a message with an advisor call: outside EVA-18 to EVA-20, so it is filed as
     `issues/done/measure_the_main_context_peak_of_an_advised_message.md`.
  2. A change of the advisor set does not confound: fixed (VRD-10).
  3. A set advisor that is never attached counts in `advisor_sessions`: not fixed, a non-goal of the spec (the
     transcript cannot tell); the probe showed an equal Opus advisor attached.
  4. Calls with no `advisorModel` field are blanked: not fixed, no transcript shows it (every advised line carries
     the field).
  5. `too_few_values` has a branch per optional metric: not fixed, two cases; generalize at the third.
  6. The advisor calls are parsed twice: fixed.
- Code review: 1 findings (low): the table and the verdict count the advisor sessions apart; not fixed, both
  are `filled(…, "advisor_calls").len()`.
- not verified: none.
