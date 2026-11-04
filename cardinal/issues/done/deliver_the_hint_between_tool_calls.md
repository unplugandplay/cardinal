# Deliver the hint between tool calls

# Goal
The `PostToolUse` mode of `language-hint` (commit `0a12992`) never reached the agent in a real session observed so
far, though the hook prints its added context when replayed. Find why and make the hint arrive, or drop the mode if
it cannot arrive cheaply: the Stop hook already keeps the final replies in the `user_language`, so the effort stays low.

# Findings
* 2026-10-30, Opus 5.5: two text blocks not in the `user_language` written before a tool call got no
  `hook_additional_context` attachment in their transcript, and the agent of the second got no added context:
  * a session of a consumer project, Claude Code 2.1.312, installed `cardinal` built after `0a12992`, the hook
    declared in its `.claude/settings.json` (session `6df30560`, transcript line 254);
  * a live probe in this repository (session `a4bd7eec`, transcript line 155, a harmless `date` call).
* Replaying the installed `cardinal run language-hint` with a `PostToolUse` event on the same transcripts (cut after
  the `tool_use` line, or whole) prints the added context, exit 0.
* Cut before the `tool_use` line, the replay prints nothing: the hook stays silent when that line is not written yet.
* Other `PostToolUse` hooks (`format-code`) show in the transcripts as `hook_additional_context`, so the channel works.

# Approach
1. Measure first: run one session with `claude --debug` (or a hook that copies its stdin and the transcript size to a
   temporary file) and look whether `language-hint` runs on `PostToolUse` and whether the `tool_use` line is in the
   transcript at that moment.
2. If the transcript lags: judge the text the event itself carries, if any, or wait for it in a bounded way.
3. If the hook does not run or its output is dropped: drop the `PostToolUse` mode and its requirements (LNG-7 to
   LNG-10), keeping the Stop hook.

# Specs
* `specs/language-hint.md`: LNG-7 to LNG-10 may be amended or removed.

# Resources
* `crates/cardinal/src/tools/language_hint.rs` (`after_call`, `text_before_call`, removed in `1b8ce83`)
* `specs/language-hint.md`, `issues/done/measure_the_effect_of_the_hint_between_tool_calls.md` (`# Tracking`)

# Hints
* The user finds the language hints already work rather well: keep this small.

# Tracking

Plan: validated

## Understanding
Measured on 2026-10-30 (session `5a68f5ea`, a probe `PostToolUse` hook logging the transcript at hook time): the hook
runs and its output reaches the agent, but only when the transcript is already written. Fast calls lose the race, and
some text blocks never reach the transcript. Out of scope: the Stop hook, which keeps working.

### Measure
| call | `tool_use` line at hook time | text block in the transcript | hint |
|---|---|---|---|
| `python3 -c 'time.sleep(2)'` (~3 s) | yes | yes | delivered (`hook_additional_context`, line 93) |
| 3 fast calls (`grep`, `sed`, `true`, < 0.5 s) | no | — | silent |
| 7 text blocks of the session | — | 3 written, 4 never written (still absent after the turn went on) | — |

The `PostToolUse` event carries no text of the agent (`tool_input`, `tool_response`, `tool_use_id` only), so waiting
for the transcript is the only fix, and it would not see the blocks that are never written.

## Plan
1. [x] Drop the `PostToolUse` mode: remove `after_call` and `text_before_call`, the `PostToolUse` entry of
   `entries/.claude/settings.json` (then `just sync`), LNG-7 to LNG-10 and their scenarios and non-goals, their tests,
   the row of `docs/architecture.md`; verified by `just check` and `just check-cardinal`; makes true LNG-1 to LNG-6
   alone (the Stop hook), and LNG-11.

## Decisions
- Mode: autonomous
- 1 a: drop the mode rather than wait for the transcript.
- 2 a: close `issues/done/measure_the_effect_of_the_hint_between_tool_calls.md` with this deliverable, moot without the mode.
- Base commit: eceb70ab8a3de3335ff43b8d991279ac638a2b16

## Deliverable 1
- done: `PostToolUse` mode of `language-hint` dropped; LNG-7 to LNG-10 withdrawn, LNG-11 added (a `PostToolUse` event a
  stale declaration still sends gets nothing); hook entry removed from `entries/` and synced; README and architecture
  rows updated; `issues/done/measure_the_effect_of_the_hint_between_tool_calls.md` closed without a measure.
- checks: `just check` ok (fmt, clippy, tests, specs).
- Code review: no findings
- red: LNG-11: the hook printed `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"Your text before this tool call is not in French, ..."}}`
- not verified: the removal of the entry from the consumers' settings (`cardinal sync` there, SYN-31).
- decisions needed: none
- follow-up: `is_hint` and BRE-13 still read the `hook_additional_context` attachment, for the transcripts of before.

## Deliverable 2 (fixes of the `high` review)
- done: the hook judges only a `Stop` event (LNG-2, LNG-11 widened to any other event), its two guards split; BRE-13
  scenario dated to the withdrawal (the `hinted` count between calls stops at that revision); Intent rewrapped; the
  Resources name the commit that removed `after_call`.
- checks: `just check` ok.
- red: LNG-11: on `SubagentStop` the hook exited 2 with `Your last reply is not in French, ...`.
- Code review: no findings (`low`)
- review `high`: 7 findings: 5 fixed here; the queue entry and the `issues/done/` link of LNG's non-goal are settled
  by `just close-issue`.
- not verified: none
- decisions needed: none
