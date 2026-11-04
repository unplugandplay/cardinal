# Hint when the reply is not in the user language

# Goal
The `user_language` rule is always-on text and the agent still breaks it. Add a `Stop` hook that reads the last
assistant message, applies the same language test as `rule-breaches`, and sends the agent back once with a short
hint when the reply is in the other language.

# Findings
* `harness run rule-breaches --summary` on 2026-10-28: rule `user-language` breached in 12 of the 142 sessions
  where it applied, 21 breaches, over 7 revisions, the latest on 2026-10-28.
* BRE-10 (`specs/rule-breaches.md`) already defines the test: a text block of at least 20 words once code is
  removed, holding more stop words of the other language; the lists are in
  `crates/harness/src/tools/rule_breaches.rs` (`stop_words`).
* `clippy-gate` is an existing `Stop` hook and lets go when `stop_hook_active` is set, which avoids a loop.

# Approach
1. Write the spec of the hook (a new capability file or requirements in an existing one): when it fires, what it
   reads (the transcript path of the `Stop` event), the hint it gives, when it lets go.
2. Share the language test between `rule-breaches` and the hook instead of copying the lists.
3. Add a `harness run` tool, declare it in `entries/.claude/settings.json` under `Stop`, then measure with
   `skill:evaluate-harness` whether `breached_share` of `user-language` drops.

# Specs
* `specs/rule-breaches.md` (BRE-10, shared test); the hook's own spec, written at `issue-workflow` step 1.

# Resources
* `specs/rule-breaches.md`, `crates/harness/src/tools/rule_breaches.rs`
* `crates/harness/src/tools/clippy_gate.rs`, `specs/clippy-gate.md` (an existing `Stop` hook)
* `entries/.claude/settings.json`

# Hints
* After a `harness sync` that adds a hook, run `just check-harness` and the `cargo install` it prints.
* A reply made mostly of code, paths or quoted output must not trigger the hint (the 20-word floor of BRE-10).

# Tracking

Plan: validated

## Understanding
A `Stop` hook `language-hint` judges the last reply with the BRE-10 test (moved to a module shared with
`rule-breaches`) and exits 2 once with a one-line hint. It reads `last_assistant_message` of the `Stop` event (Claude
Code 2.1.312 sends it), the transcript only as a fallback. The measure needs sessions run with the hook: a follow-up.

## Plan
1. [x] Shared language test: move `stop_words`, `prose_words` and the 20-word floor of `rule_breaches.rs` into
   `tools/language.rs`, one function judging a text; verified by the `rule_breaches` tests unchanged (~120 lines).
2. [x] Hook `language-hint`: the tool, its declaration under `Stop` in `entries/.claude/settings.json`, `just sync`,
   docs (architecture table); verified by `tests/language_hint.rs` through the binary; makes true LNG-1 to LNG-6,
   spec `accepted` (~350 lines).
3. [x] Follow-up issue to measure `breached_share` of `user-language` with `skill:evaluate-harness` once sessions ran
   with the hook (~40 lines, `issues/` only).
4. [x] Fixes of the `high` review: the transcript fallback stops at a typed message, the config falls back to
   `CLAUDE_PROJECT_DIR` when the cwd's work tree has none, a 600 s timeout, the git and config lookups shared;
   verified by `tests/language_hint.rs`; amends LNG-2, LNG-3.

## Decisions
- Mode: autonomous
- Base commit: fddf3f4e14becb841336fd2e6b0e11c4976e54a9
- The measure of `breached_share` goes to a follow-up issue (deliverable 3): no session ran with the hook yet.

## Deliverable 1
- done: the stop words, the prose filter and the 20-word floor moved to `tools/language.rs` (`Language::named`,
  `breached_by`); `rule-breaches` calls it.
- | check | result |
  |---|---|
  | tests | `cargo test` green, BRE-10 test unchanged |
  | lint | clippy -D warnings clean |
  | specs | `just check-specs` ok |
  | docs | no doc names the moved code (grep of `docs/`, `README.md`, `CLAUDE.md`) |
  | code review | low, no findings |
- red: none (pure refactor under the existing BRE-10 test)
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: tool `language-hint` (`tools/language_hint.rs`), declared under `Stop` in `entries/.claude/settings.json` and
  synced; spec `language-hint.md` accepted; README (Gates), architecture (hooks table) and the docs map updated.
- | check | result |
  |---|---|
  | tests | `tests/language_hint.rs` 6 tests green, `cargo test` green |
  | lint | clippy -D warnings clean |
  | specs | `just check-specs` ok, LNG-1 to LNG-6 traced |
  | docs | README, `docs/architecture.md`, `.claude/update-docs-map.md` |
  | code review | low, no findings |
- red: LNG-1, LNG-2, LNG-3, LNG-4, LNG-5, LNG-6: harness: unknown tool `language-hint` (exit 3, the 6 tests failed)
- Code review: no findings
- not verified: the hook in a live session (the field `last_assistant_message` was read from the Claude Code 2.1.312
  bundle, not from a recorded event)
- decisions needed: none

## Deliverable 3
- done: `issues/done/measure_the_effect_of_the_language_hint.md`, queued last (it needs sessions run with the hook).
- red: none (an issue, no test)
- Code review: no findings (a diff under `issues/` only needs no review)

## Deliverable 4 (fixes of the `high` review of fddf3f4..4020f75, 10 findings)
- done: LNG-2 and LNG-3 amended and fixed, `timeout: 600` on the hook (a `harness run` with a local source builds
  first, like clippy-gate), `transcripts::git` and `Language::of_config` reused instead of copies.
- | check | result |
  |---|---|
  | tests | `tests/language_hint.rs` green, `cargo test` green |
  | lint | clippy -D warnings clean |
  | specs | `just check-specs` ok |
  | docs | spec amended (LNG-2, LNG-3, non-goal on `stop_hook_active`) |
  | code review | low, no findings |
- red: LNG-2, LNG-3: the English reply of the previous turn was judged (exit 2), and the nested clone with no config
  read no language (exit 0)
- Findings of the `high` review not fixed here:
  - `stop_hook_active` is shared by every Stop hook, so a clippy-gate block turns the hint off for that turn: a
    non-goal of the spec now, the flag is what prevents a loop.
  - The hook judges the joined blocks of the last line, BRE-10 each block: `last_assistant_message` is one joined
    text already; the measure issue counts final replies apart.
  - `an`/`as` in the English list, exact language names: outside this issue's requirements, recorded in
    `issues/done/measure_the_effect_of_the_language_hint.md`.
  - The seed's `user_language = "English"` now becomes a gate: by design, the key declares the language; the
    user's projects set French.
- Code review: no findings
- not verified: none
- decisions needed: none
