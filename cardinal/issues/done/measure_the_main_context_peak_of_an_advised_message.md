---
rank: 37
why: 'right after the advisor count: once advisors are called, every advised session skews the peak that the advisor and context verdicts read'
---
# Measure the main context peak of an advised message

# Goal
`main_context_peak` (EVA-16) adds up the `input_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`
of the top-level `message.usage`. On a message with an advisor call, that usage adds up every executor `message`
iteration of `usage.iterations`, so the peak comes out about twice the real largest context. The expected outcome:
the peak of an advised session is the size of its largest context, so an advised group and a solo group can be
compared.

# Findings
* Measured on 2026-10-30 (Claude Code 2.1.312, an Opus session with an Opus advisor and one advisor call): the
  top-level usage reads input 4, cache read 28,546, cache creation 10,919, a peak of 39,469; the two `message`
  iterations read (2, 9,012, 10,522) and (2, 19,534, 397), so the largest context is 19,933.
* A message with no advisor call has a single `message` iteration (or none), so its top-level usage is already the
  context size.
* The top-level sums are what is billed, so `input_tokens`, `cache_read_tokens` and `cache_creation_tokens` stay
  right; only the peak is wrong.
* Found by the `high` review of `issues/done/count_the_advisor_calls_of_each_session.md`.

# Approach
1. When `usage.iterations` holds entries of type `message`, take the largest per-iteration sum of input and cache as
   the context of the message; otherwise keep the top-level sum.
2. Archived rows keep their old peak: say so in the spec (an advised session archived before the fix is overstated).

# Specs
* `specs/evaluation.md`: amends EVA-16.

# Milestone
Advisor

# Resources
* `crates/cardinal/src/tools/session_metrics.rs` (`Session::assistant`), `crates/cardinal/tests/session_metrics.rs`.
* `specs/evaluation.md` (EVA-16, EVA-19).

# Tracking

Plan: validated

## Understanding
The main context peak of an advised message is the largest sum of input and cache over its `message` iterations,
not the top-level sum; the billed token columns stay the top-level sums. Archived rows are not rewritten.

## Plan
1. [x] Peak of an advised message: `Session::assistant` takes the largest `message` iteration when there is one; the spec says archived rows keep the old peak; verified by a check of `advisor_checks` (top-level 39,469, iterations 19,933 → peak 19,933); makes true EVA-16.

## Decisions
- Mode: autonomous
- Base commit: a8d2b18

## Summary of deliverable 1
done: the main context peak of an advised message is its largest `message` iteration; `context_tokens` shared with the advisor input sum; EVA-16 amended (archived rows keep the old peak).

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy -D warnings clean |
| specs | EVA-16 traced, spec `accepted` |
| docs | no doc describes how the peak is computed beyond EVA-16 |
| code review | `low`, see below |

red: EVA-16: `FAIL - the peak of an advised message is its largest message iteration` (39469, not 19933)

Code review: no findings

not verified: a real advised transcript run through `session-metrics` (the fixture uses the measured numbers).

decisions needed: none

## Summary of the review of the whole issue
done: the `high` review of `a8d2b18..HEAD`; the peak rule now fires only on a message with an `advisor_message` iteration (EVA-16 says so), and a check pins the top-level peak of a message with one partial `message` iteration.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clippy -D warnings clean |
| specs | EVA-16 reworded, still traced, spec `accepted` |
| docs | none to change |
| code review | `high` on the issue, then `low` on the fixes |

red: EVA-16: `FAIL - a message with no advisor iteration keeps its top-level peak` (2, not 4)

Code review: 7 findings
- Rule fired on any `message` iteration, partial fields gave a low peak, spec criterion vs its parenthesis: fixed, the trigger is an `advisor_message` iteration.
- No check on a non-advised message with an iteration: fixed, check on the `none` session.
- Doc comment over 120 columns: fixed.
- Iterations only on the last line of a split message: not fixed, real advised transcripts (2.1.312) repeat `iterations` on every line.
- The billed `tokens` closure next to `context_tokens`: not fixed, the billed columns keep each key apart, the context sums them.

not verified: none

decisions needed: none
