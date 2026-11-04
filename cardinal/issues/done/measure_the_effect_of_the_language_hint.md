# Measure the effect of the language hint

# Goal
The `language-hint` Stop hook (`specs/language-hint.md`) sends the agent back once when its last reply is not in the
`user_language`. Measure whether it lowers the share of sessions that breach rule `user-language`, then keep the hook
or drop it.

# Findings
* `cardinal run rule-breaches --summary` on 2026-10-28, before the hook: rule `user-language` breached in 12 of the
  142 sessions where it applied, 21 breaches.
* The hook was added in commit `ad56d2a`; no session had run with it when this issue was written.
* The hook judges only the last reply of a turn, while BRE-10 judges every assistant text block: intermediate blocks
  between tool calls can still count as breaches.

# Approach
1. Wait until enough sessions ran with a revision holding the hook (`skill:evaluate-harness` gives the sessions per
   revision).
2. Compare `breached` / `applied` of `user-language` before and after that revision, with the model and Claude Code
   version next to each number.
3. Candidate outcomes: keep the hook; also judge the intermediate blocks; drop it when the share does not move.

# Resources
* `specs/language-hint.md`, `specs/rule-breaches.md` (BRE-10)
* `crates/cardinal/src/tools/language_hint.rs`, `crates/cardinal/src/tools/language.rs`
* `skill:evaluate-harness`

# Hints
* A breach the hook caught and the agent then fixed still shows in the transcript as a breach of BRE-10: count the
  final replies apart, or the hook looks useless.
* Found by the review of the hook (2026-10-28): the English list of BRE-10 holds `an` and `as`, also French words
  (`un an`, `tu as`), and `Language::named` knows only the exact strings `French` and `English` (`french` turns the
  rule off silently). Both skew the measure and the hook alike.

# Tracking

Plan: validated

## Understanding
Measure whether the hook lowered the share of sessions breaching `user-language`, then decide its fate. No code
change in this issue: the outcome is the decision and, when the hook stays, the follow-up issues.

## Measure (2026-10-29)
`cardinal run rule-breaches` (summary and detail), revisions split on whether they hold `ad56d2a`; a breach is
`final` when no assistant text follows it before the next typed message, else `intermediate`.

| group | sessions applied | sessions breached | breaches | final | intermediate | models, versions |
|---|---|---|---|---|---|---|
| before the hook | 72 | 8 (11%) | 20 | 2 | 18 | opus 5.5 and sonnet 5.5, 2.1.309 to 2.1.310 |
| with the hook | 32 | 4 (12.5%) | 4 | 0 | 4 | opus 5.5, 2.1.310 to 2.1.312 |

* Model mix: the 12 breaching sessions all ran on opus 5.5; opus only, the share before the hook is 8 of 54 (15%),
  against 4 of 32 (12.5%) with it, too small a gap on these samples to name a trend.
* The hook ran on 157 stops of 51 sessions and never sent the agent back: no `stop_hook_summary` holds a hook error
  and no transcript holds its hint outside the sessions that wrote the hook.
* The share did not move, and could not: 18 of the 20 breaches before the hook were intermediate blocks, which the
  hook does not judge (non-goal of `specs/language-hint.md`). Only 2 final replies breached, both in one session.
* The 142 sessions of the Findings are now 104: older transcripts were pruned.

## Plan
1. [x] Decide the hook's fate and file the follow-up issue: the decision recorded here, one issue (`skill:create-issue`)
   to judge the text blocks that precede a tool call, carrying the two skews of the Hints (`an`/`as` in the English
   list, `Language::named` case-sensitive); verified by the checker of `create-issue` and `just check`.
2. [x] Fix the findings of the `high` review of the issue; verified by `just check-issue` and `just next-issue`.

## Deliverable 1
- done: decision recorded (keep the hook); follow-up `issues/done/judge_the_text_blocks_between_tool_calls.md`, ranked 2nd.

| check | result |
|---|---|
| `just check` | pass |
| `just check-issue` (follow-up) | pass |
| `just next-issue` | no drift |

- red: none (a measure, no test)
- Code review: no findings (`low`; the diff holds only issue files)

## Deliverable 2
- done: fixes of the `high` review of the issue (8 findings): model mix split, the "never fired" evidence, this
  table, the wording of the follow-up's candidates and baseline hint, the queue reasons.
- red: none (issue files only)
- Code review: 8 findings, all fixed in the issue files; the measure issue's own queue line goes at its closing.
- not verified: how a blocking Stop hook shows in `stop_hook_summary` (no block happened to compare); the absence
  of the hint text in every transcript stands in for it
- decisions needed: none

## Decisions
- Mode: autonomous
- Base commit: 832987be58d203a652e14e177ee5616d3e706466
- Outcome (1 a): keep the hook; the follow-up issue judges the intermediate blocks and fixes the two skews.
