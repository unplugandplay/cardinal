# Judge the text blocks between tool calls

# Goal
Rule `user-language` is mostly breached in the text blocks the agent writes between tool calls, which the Stop hook
`language-hint` never judges. Send the agent back on those blocks too, so the share of sessions breaching the rule
falls, and remove the two skews that make the hook and the measure misjudge a text.

# Findings
* Measure of 2026-10-29 (`issues/done/measure_the_effect_of_the_language_hint.md`, `# Tracking`): before the hook, 18 of
  the 20 breaches of `user-language` were intermediate blocks (assistant text followed by more assistant text before
  the next typed message), 2 were final replies; with the hook, 4 breaches, all intermediate. The hook ran on 157
  stops of 51 sessions and no transcript holds its hint (how a blocking Stop hook shows in `stop_hook_summary` was
  not checked).
* The English stop-word list of `crates/cardinal/src/tools/language.rs` holds `an` and `as`, also French words
  (`un an`, `tu as`).
* `Language::named` matches only the exact strings `French` and `English`: `user_language = "french"` turns the rule
  and the hook off silently.

# Approach
1. Check which hook event sees the text block of an assistant message before its tool call (`PreToolUse` or
   `PostToolUse` with `transcript_path`), and how each feeds a message back to the agent without failing the call.
2. Candidates, can be combined:
   1. A `PostToolUse` hook that judges the assistant text written since the last tool result or typed message, with
      the test of BRE-10, and tells the agent once per turn to write in the `user_language`.
   2. A `PreToolUse` hook doing the same, through added context rather than a denied call if the event allows it
      (a denied call costs a retry of the tool).
   3. Extend `language-hint` to judge every text block of the turn at Stop: the hint comes after the fact, the user
      already read the blocks.
3. Drop `an` and `as` from the English list; match the language name without case, or warn on an unknown name.
4. Measure again with `cardinal run rule-breaches` once sessions ran with the change, final and intermediate blocks
   counted apart.

# Specs
* `specs/language-hint.md`: its non-goal on intermediate blocks changes; new requirements for the chosen event.
* `specs/rule-breaches.md`: BRE-10 (stop-word lists, language name).

# Resources
* `specs/language-hint.md`, `specs/rule-breaches.md`, ADR 0005
* `crates/cardinal/src/tools/language_hint.rs`, `crates/cardinal/src/tools/language.rs`
* `entries/.claude/settings.json` (where a new hook is declared)
* `issues/done/measure_the_effect_of_the_language_hint.md` (the measure and its classification of the breaches)

# Hints
* A breach the hook caught and the agent then fixed still shows as a breach of BRE-10: count the corrected blocks
  apart, or the measure hides the effect.
* The transcript writes each content block of a message on its own line sharing one `message.id`: the line of the
  tool call holds no text.
* Changing the stop-word lists changes what BRE-10 counts: re-run the baseline with the new lists on the transcripts
  still kept (pruning removes the old ones) before comparing with the table of 2026-10-29.
* One hint per turn at most: a hook firing on every tool call of a long turn floods the context.

# Tracking

Plan: validated

## Understanding
Judge the text the agent writes before a tool call in a `PostToolUse` mode of `language-hint` (one added context per
turn, the call never fails), fix the two skews of BRE-10, and let `rule-breaches` tell final, intermediate and hinted
breaches apart. Out of scope: the measure after the change, which needs sessions run with it (a follow-up issue).

## Findings (preparation)
* A blocking Stop hook shows as a user line with `isMeta: true` whose content starts with `Stop hook feedback:\n[<command>]: <stderr>`;
  `typed_text` already skips it.
* A `PostToolUse` hook printing `hookSpecificOutput.additionalContext` shows as an `attachment` line of type
  `hook_additional_context` (seen for the formatter); the transcript writes the text and `tool_use` lines of a message
  before its tool results, so the text before a call is on disk when its `PostToolUse` runs.

## Plan
1. [x] Stop words and language name: drop `an` and `as` from the English list, match `user_language` without case;
   verified by tests through the binaries of `rule-breaches` and `language-hint`; makes true BRE-10.
2. [x] Kinds of language breach: `detail` `hinted`, `final` or `intermediate`, then the baseline re-run with the new
   lists on the transcripts kept, recorded here; verified by a `rule-breaches` test; makes true BRE-13.
3. [x] `PostToolUse` mode of `language-hint`, declared for every tool in `entries/.claude/settings.json`, synced and
   installed; verified by `language-hint` tests through the binary; makes true LNG-1, LNG-2, LNG-5, LNG-7 to LNG-10.
4. [x] Follow-up issue to measure the change once sessions ran with it (`skill:create-issue`); verified by its checker.
5. [x] Fixes of the `high` review of the issue; verified by the `language-hint` and `rule-breaches` tests; makes
   true LNG-7, LNG-8, BRE-10, BRE-13.

## Deliverable 1
- done: `an` and `as` left the English list; `user_language` read without case (`french` works).

| check | result |
|---|---|
| `just check` | pass |
| docs | none to change (the lists are not documented outside the code) |
| code review | `low`, see below |

- red: BRE-10, LNG-3, LNG-4: `rule_breaches` FAIL on the `french` project and the `an`/`as` block; `language_hint`
  exit `Some(0)` instead of `Some(2)` for `user_language = "french"`.
- Code review: no findings (`low`)
- not verified: none
- decisions needed: none

## Deliverable 2
- done: a `user-language` breach carries `detail` `hinted`, `final` or `intermediate`; both hints share the mark
  `HINT_MARK` of `language.rs`; baseline below.

| check | result |
|---|---|
| `just check` | pass |
| docs | none to change (`docs/usage.md` and `docs/architecture.md` do not list the detail column) |
| code review | `low`, see below |

- red: BRE-13: `rule_breaches` FAIL, every `user-language` row had an empty `detail`.
- Code review: no findings (`low`)
- not verified: none
- decisions needed: none

### Baseline (2026-10-29, new lists)
Revisions split by `git merge-base --is-ancestor ad56d2a <revision>`, on the transcripts still kept.

| group | sessions applied | sessions breached | breaches | final | intermediate | hinted |
|---|---|---|---|---|---|---|
| before the hook | 89 | 9 | 21 | 2 | 19 | 0 |
| with the Stop hook | 29 | 4 | 4 | 0 | 4 | 0 |

* The old lists (base `503e634`, built apart) give the same summary on these transcripts: dropping `an` and `as`
  changes no count here.
* The counts differ from the table of 2026-10-29 (72 and 32 sessions applied) by the grouping above and the sessions
  run since; the comparison of the follow-up measure starts from this table.

## Deliverable 3
- done: `language-hint` judges the text before a tool call on `PostToolUse` (every tool) and adds one context per
  turn; it reads only the lines of the turn, without parsing tool results or other attachments; declared in
  `entries/.claude/settings.json`, synced; both specs `accepted`.

| check | result |
|---|---|
| `just check` | pass |
| `just check-cardinal` | pass, no install asked |
| live run (`cardinal run language-hint`, crafted transcript) | added context printed, exit 0 |
| time per call, 14 MB transcript | ~25 ms with a release binary (consumers); ~150 ms through `cardinal run` here (debug build of the local source) |
| docs | `docs/architecture.md` (hooks table, readers table for BRE-13), `README.md` (Gates) |
| code review | `low`, see below |

- red: LNG-7, LNG-8, LNG-9: the tests got `code: Some(2)` and the Stop hint on stderr instead of an added context
  (the event was judged as a Stop).
- Code review: no findings (`low`)
- not verified: that Claude Code shows the added context to the model in a real session (the formatter's context
  shows as `hook_additional_context`, the same channel); the follow-up measure checks it in the transcripts
- decisions needed: none

## Deliverable 4
- done: `issues/done/measure_the_effect_of_the_hint_between_tool_calls.md`, ranked 6th (after the other stacks, so
  sessions have run with the hint).

| check | result |
|---|---|
| `just check-issue` | pass |
| `just next-issue` | no drift |

- red: none (issue files only)
- Code review: no findings (`low`; issue files only)
- not verified: none
- decisions needed: none

## Deliverable 5
- done: fixes of the `high` review (`503e634..28e0441`, 9 findings).

| # | finding | done |
|---|---|---|
| 1 | a slash or `!` command did not open a turn for the hook | `user_turn` shared in `transcripts.rs` (LNG-7, LNG-8) |
| 2 | a hint marked every unhinted breach of the turn | a hint covers the run of blocks it judged (BRE-13) |
| 3 | every line of the turn parsed on each call | only user, assistant and hint lines parsed: ~7 ms per call in release on a 14 MB transcript |
| 4 | a failed call (`PostToolUseFailure`) judges nothing | non-goal of `specs/language-hint.md`: the next call that succeeds judges the text |
| 5 | a Stop hint did not count for the once-per-turn rule | `is_hint` counts both kinds (LNG-8) |
| 6 | any meta line holding the mark counted as a hint | Stop feedback must start with `Stop hook feedback:` |
| 7 | the hint test was copied in two files | `language::is_hint`, shared |
| 8 | dead turn check in `hinted_this_turn` | removed with it, the scan stops at the hint |
| 9 | `user_language` not trimmed | trimmed (BRE-10) |

| check | result |
|---|---|
| `just check` | pass |
| docs | n/a (the hook table and Gates line still hold) |
| code review | `low`, see below |

- red: LNG-7, LNG-8, BRE-10, BRE-13: the hint was withheld after a slash command, `" french "` read as no language, the
  blocks of later calls were `hinted`.
- Code review: no findings (`low`)
- not verified: none
- decisions needed: none

## Decisions
- Mode: autonomous
- Base commit: 503e63421de559e6e4ae59ec7e931533fa03cbfd
- Event (1 a): `PostToolUse`, the text before the call is on disk and the hint comes with the tool result.
- Language name (2 a): matched without case; an unknown name stays silent.
- Tools (3 a): every tool, no matcher.
- Cost (asked at validation): no token while the language is kept, one line of ~40 tokens per turn at most; ~40 ms
  per tool call measured on a 14 MB transcript with the Stop mode, accepted.
- Closed: 2026-10-29, every deliverable done; the measure after the change is `issues/done/measure_the_effect_of_the_hint_between_tool_calls.md`.
