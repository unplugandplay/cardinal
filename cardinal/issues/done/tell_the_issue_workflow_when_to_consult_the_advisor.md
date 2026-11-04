---
rank: 50
why: 'after the advisor count, which gives it its verdict; the cheapest lever on the plan quality'
---
# Tell the issue workflow when to consult the advisor

# Goal
Claude Code has no setting to force or cap the advisor calls: only the instructions decide when the agent consults
it. `skill:issue-workflow` names no such moment, so a configured advisor is consulted at random or never. The outcome:
the skill names the few moments where advice pays (before committing to a plan, when stuck, before declaring done) and
how to weigh it, and stays neutral when no advisor is configured.

# Findings
* The advisor tool docs recommend a call before substantive work (after the orientation reads), when stuck or changing
  approach, and before declaring done once the deliverable is durable; and to follow the advice unless a step fails
  empirically or primary evidence contradicts it, then consult once more to reconcile.
* Anthropic measured on SWE-bench Multilingual +2.7 points and -11.9% cost per task for Sonnet with an Opus advisor
  against Sonnet alone.
* No advisor call is found in the user's transcripts though an advisor is configured (see
  `count_the_advisor_calls_of_each_session.md`).
* The workflow's matching moments: end of step 1 (plan written, before the stop of step 2), a check still red after a
  fix (step 3.5), before the `high` review of the whole issue.

# Approach
1. Add to `skill:issue-workflow` a short "Advisor" paragraph: the three moments above, the weighing rule, "skip it when
   no advisor tool is available".
2. Commit with `Expected-effect:` on the advisor call count (once it exists) and on `fixes_after` or
   `findings_per_review` down.
Candidate: the same paragraph as an always-on rule in `.claude/rules/` instead, if other skills need it; it costs fixed
context on every session.

# Milestone
Advisor

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 1, 2, 3.5, end of issue).
* https://platform.claude.com/docs/en/agents-and-tools/tool-use/advisor-tool (suggested system prompt for coding).
* `issues/done/count_the_advisor_calls_of_each_session.md`.

# Hints
* The advisor never replaces the `low` and `high` reviews nor the stop of step 2: they stay never lifted.
* Opus executors lost pass rate when nudged to call more: no "call it at least N times" rule.

# Tracking

Plan: validated

## Understanding
Add to `skill:issue-workflow` an "Advisor" section: consult at the three moments (plan written before the stop of step
2, a check still red after a fix in step 3.5, before the `high` review of the issue), follow the advice unless a step or
primary evidence contradicts it then consult once to reconcile, never instead of a review or a stop, no minimum count,
skip it when no advisor tool is available. Out of scope: an always-on rule in `.claude/rules/` (fixed context on every
session, no other skill needs it yet). No spec: harness content.

## Plan
1. [x] Advisor section of `skill:issue-workflow` (~15 lines in `files/`, synced copy): verified by new required phrases
   in `crates/cardinal/tests/skills.rs` (section, the skip when absent, the never-replaces line); commit with
   `Expected-effect: advisor_calls up` and `Expected-effect: fixes_after_per_issue down`.
2. [x] Fixes of the `high` review on base..HEAD: the moments include a red test of 3.3, the plan call is not repeated
   on an unchanged plan, the last call comes after the issue's `update-docs`; "only there" and the duplicate skip
   sentence dropped; `docs/workflow.md` points to the skill instead of copying the moments; one more required phrase.

## Decisions
- Mode: autonomous
- Base commit: 48b8623ca0e2e09c95dece4462b4a313e84b8f02
- Placement: a section of `issue-workflow`, not an always-on rule (1 a).

## Summary of deliverable 1
- done: "Advisor" section in `skill:issue-workflow` (three moments, weighing rule, never replaces a review or a stop,
  skipped without an advisor); `docs/workflow.md` and `docs/architecture.md` mention it.
- red: none (no spec; the required phrases of `tests/skills.rs` failed: missing '## Advisor', 'no advisor tool is
  available', 'never replaces a review or a stop').
- check | result: just check ok (fmt, clippy, tests, specs); docs updated; code review `low` run.
- Code review: no findings
- not verified: that an agent actually consults the advisor at these moments (`advisor_calls` will tell).
- decisions needed: none.

## Summary of deliverable 2
- done: fixes of the `high` review (findings 1, 2, 4, 5, 6, 8, 10).
- check | result: just check ok; docs updated; code review `low` run.
- red: none (wording fixes; the new required phrases already held in the text written by deliverable 1).
- High review: 10 findings; not fixed: 3 (pointers at steps 2 and 3.5 would repeat the section in a skill read whole),
  7 (moving the section to details.md: ~120 tokens, and the issue chose the skill), 9 (the diagram stays at step
  level, the advisor is not a step).
- Code review: no findings
- not verified: that an agent consults the advisor at these moments (`advisor_calls` will tell).
- decisions needed: none.
