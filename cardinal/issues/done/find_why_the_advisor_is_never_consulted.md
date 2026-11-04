---
priority: low
why: 'every session since the advisor paragraph ran an executor as strong as its advisor; needs sessions with a weaker one'
---
# Find why the advisor is never consulted

# Goal
`skill:issue-workflow` names four moments to consult the advisor (its section "Advisor", commit `dc4b1e0`), yet the
advisor is still almost never called. Either the agent ignores the paragraph, or consulting a model as strong as the
executor is rightly skipped. The outcome: a measured cause, and the paragraph kept, conditioned on the setup, or
removed.

# Findings
* Measured 2026-11-03: the advisor tool is offered in 466 of 525 sessions (`advisor_models` set) and called in 2.
* Since `dc4b1e0` (2026-10-30), 78 sessions loaded `skill:issue-workflow`: all ran `claude-opus-5-5` advised by
  `claude-opus-5-5`, and none called the advisor.
* Before it, 102 sessions ran `claude-sonnet-5-5` advised by `claude-opus-5-5`, with 1 call: no session with a weaker
  executor has run the paragraph yet.
* The paragraph costs about 700 bytes of `skill:issue-workflow`, loaded once per issue session.
* Cause, decided 2026-11-03 without a replay: an executor advised by the same model gains nothing from the call (same
  knowledge, same blind spots; the code review already gives a fresh context), so zero calls in the `solo` setups is
  expected, not the paragraph ignored. The paragraph is kept unchanged; a weaker executor with a stronger advisor, if
  the user ever runs one, shows its calls in `cardinal-report --by setup` without a replay.

# Approach
1. Wait for or run about 10 issue sessions with a weaker executor and a stronger advisor (`just replay` can drive them),
   and count the calls per issue session.
2. If they call it, condition the paragraph on an advisor stronger than the executor; if they do not, read the
   transcripts at the four moments to see why.
3. Decide with the user: keep, condition, or remove the paragraph (commit with `Expected-effect: advisor_calls`).

# Milestone
Lean agent loop

# Resources
* `issues/done/tell_the_issue_workflow_when_to_consult_the_advisor.md`,
  `issues/done/compare_an_advised_executor_with_a_solo_model.md`.
* `files/.claude/skills/issue-workflow/SKILL.md`, section "Advisor"; `specs/evaluation.md` (the advisor columns).

# Hints
* `cardinal-report --by setup` labels an executor advised by the same model as `solo`.


# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Decisions
- No replay: the user judged the measure not worth its cost; the issue closes on the reasoned cause above, the
  paragraph unchanged.
