# Measure the effect of the hint between tool calls

# Goal
The `PostToolUse` mode of `language-hint` (commit `0a12992`) adds a context once per turn when the text written
before a tool call is not in the `user_language`. Measure whether the share of sessions breaching rule
`user-language` fell, and whether the hint reaches the agent, then decide whether to keep the mode, change it or drop
it.

# Findings
* Baseline of 2026-10-29 (`issues/done/judge_the_text_blocks_between_tool_calls.md`, `### Baseline`): before the Stop hook,
  9 of 89 sessions breached (21 breaches: 2 final, 19 intermediate); with the Stop hook, 4 of 29 (4 intermediate),
  revisions split by `git merge-base --is-ancestor ad56d2a <revision>`.
* `rule-breaches` marks each `user-language` breach `hinted`, `final` or `intermediate` (BRE-13). The hint shows in a
  transcript as an `attachment` line of type `hook_additional_context`, or as meta Stop hook feedback, holding
  `the user_language of .claude/cardinal.toml`.
* Not verified when the mode shipped: that Claude Code shows the added context to the model in a real session.

# Approach
1. Wait until enough sessions ran on a revision holding `0a12992` (about as many as the baseline's group with the Stop
   hook, 29 sessions applied).
2. Run `cardinal run rule-breaches` (summary and detail), grouped as the baseline (before the Stop hook, Stop hook
   only, with `0a12992`), counting `hinted`, `final` and `intermediate` apart, with the model and Claude Code version
   of each group.
3. Check in the transcripts that each hint shows as `hook_additional_context`, and whether the blocks after a hint in
   the same turn breach less than those before it.
4. Decide: keep the mode, change it (for example a hint per block rather than per turn), or drop it; file a follow-up
   when a change is needed.

# Resources
* `issues/done/judge_the_text_blocks_between_tool_calls.md` (the baseline and the decisions)
* `issues/done/measure_the_effect_of_the_language_hint.md` (the method of the previous measure)
* `specs/language-hint.md`, `specs/rule-breaches.md` (BRE-10, BRE-13), `skill:evaluate-harness`

# Hints
* Pruning removes old transcripts: re-run the baseline groups on the transcripts kept at the time of the measure
  instead of comparing with the table of 2026-10-29 alone.
* A `hinted` breach is the block the hook caught: count it apart, the effect shows in the blocks after it.

# Tracking

Plan: validated

## Understanding
Measure the `PostToolUse` hint of `language-hint` against the baseline groups and decide its fate. Found at
preparation (2026-10-30): the group with `0a12992` holds 15 sessions applied (threshold ~29), 1 breach, 0 `hinted`;
that breach (`6df30560…jsonl:254`, `Assistant`, Claude Code 2.1.312) got no hint in the transcript, though replaying
the installed hook on the transcript cut after the call prints the added context. So check delivery first.

## Plan
1. [x] Interim measure: the three groups (sessions, breaches, `hinted`/`final`/`intermediate`, model and Claude Code
   version) and the missed hint written in `## Interim measure`; verified by `just check`.
2. [x] Live probe: one text block not in French before a harmless tool call in a real session, then look in its
   transcript for the `hook_additional_context` attachment; a missing one files a follow-up issue on the delivery
   (`skill:create-issue`); verified by the transcript line cited.
3. [ ] Final measure and decision once the group with `0a12992` reaches ~29 sessions: re-run the groups, compare the
   blocks after a hint with those before it, keep, change or drop the mode; verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: `9670a847d424931528b24e1152a89a49ae61dd31`
- Effort: minimal (the user, 2026-10-30: the language hints already work rather well); no new tooling for this measure.
- Live probe: in the session of the plan validation, one English sentence before a harmless command (1 a); that
  breach is left out of the counts.
- Waiting: deliverables 1 and 2 now, the issue stays open for deliverable 3 (2 a).

## Interim measure (2026-10-30)
`user-language`, revisions split by `git merge-base --is-ancestor` (`ad56d2a`, then `0a12992`), on the transcripts
kept; Opus 5.5 in every group.

| group | sessions applied | sessions breached | breaches | final | intermediate | hinted |
|---|---|---|---|---|---|---|
| before the Stop hook | 72 | 8 | 20 | 2 | 18 | 0 |
| Stop hook only | 55 | 6 | 6 | 0 | 6 | 0 |
| with `0a12992` | 15 | 1 | 1 | 0 | 1 | 0 |

* The breach of the last group (a consumer, Claude Code 2.1.312) got no hint in its transcript; the replayed hook
  prints one.

## Live probe (2026-10-30)
* An English sentence before a `date` call in this session (transcript `a4bd7eec`, line 155): no
  `hook_additional_context` in the transcript, no context received by the agent; the replayed hook prints one.
* Follow-up: `issues/done/deliver_the_hint_between_tool_calls.md`, ranked first. Deliverable 3 waits for it and for ~29
  sessions with a delivered hint.

## Deliverables 1 and 2
- done: interim measure and live probe recorded above; follow-up issue filed and queued.

| check | result |
|---|---|
| `just check-issue` (follow-up) | pass |
| `just next-issue` | pass, no drift |
| code review | none: the diff is under `issues/` only |

- red: none
- Code review: no findings (no code changed)
- not verified: why the hint is not delivered (the follow-up measures it)
- decisions needed: none

## Closed without a measure
2026-10-30: the `PostToolUse` mode was dropped (`issues/done/deliver_the_hint_between_tool_calls.md`): Claude Code often
runs the hook before it writes the text and the call to the transcript, so there is no hint left to measure.
