---
rank: 150
why: 'after the routing count, which shows whether an audit fix changed anything'
---
# Audit the shared instructions for conflicts and stale guidance

# Goal
The managed instructions grew one issue at a time: `CLAUDE.md` templates, the always-on rule, eight skills of which
`issue-workflow` alone is 171 lines. Rules written for older models (emphatic "never", repeated warnings, steps the model
now does unprompted) and rules that contradict each other cost fixed context and make the agent hesitate. The outcome:
a reviewed list of the contradictions, duplicates and stale rules of `files/`, and the fixes the user accepts, each
measured.

# Findings
* `files/.claude/skills/*/SKILL.md` total 579 lines, `issue-workflow` 171 plus its `details.md`; one always-on rule of
  8 lines (counted 2026-10-30).
* `skill:audit-instructions` proposes new skills and hooks from what the user typed; it does not review the existing
  instructions for conflicts or staleness.
* The article on folding subagents names the "Frankenprompt" (contradictory rules piled up) and points to a prompt audit
  that flags instructions written for older models and proposes diffs; it also puts the mechanical parts in code.
* Candidate mechanical rule now in prose: the size limit of a deliverable (`git diff --shortstat` against ~1,000 lines),
  which a `cardinal run` tool could compute.

# Approach
1. Read `files/` whole and list, with file and line: contradictions, duplicates across skills, rules a current model
   follows unprompted, rules that a tool could check.
2. Show the list for validation; one commit per accepted family of fixes, with `Expected-effect:` on the fixed context
   and the metric it should not hurt.
Candidate: make the review a mode of `skill:audit-instructions` rather than a one-off.

# Milestone
Lean agent loop

# Resources
* `files/`, `files/.claude/skills/audit-instructions/SKILL.md`, `specs/instruction-audit.md`.
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop

# Hints
* A rule kept because of a past incident has its issue in `issues/done/`: read it before removing the rule.
* Cut one family at a time, so a regression points at its cause.

# Tracking

Plan: validated

## Understanding
Review the managed instructions of `files/` (eight skills, `details.md`, the always-on rule, `AGENTS.md`; there is no
`CLAUDE.md` template in `files/` nor `seed/`) and cut, one family per commit, the duplicates, contradictions and stale
rules the user accepts. Out of scope: new tools and hooks (the mechanical candidates become issues), behaviour of the
binary.

## Audit (2026-10-31, at 2a957a6)

| # | family | where | finding |
|---|---|---|---|
| D1 | duplicate | 8 skills: `issue-workflow/SKILL.md:12`, `audit-instructions:11`, `commit-message:11`, `create-issue:11`, `evaluate-harness:12`, `prioritize-issues:13`, `update-docs:11`, `write-spec:13` | "Show the user your results as `reports-to-the-user.md` says": the rule has no `paths`, so it is already loaded in every session |
| D2 | duplicate | 5 skills: `issue-workflow:8`, `audit-instructions:8`, `evaluate-harness:8`, `prioritize-issues:8`, `create-issue:19` | the same `user_language` sentence, each with its own parenthesis; the `language-hint` hook backs it |
| D3 | duplicate, self-contradiction | `commit-message:8` vs `:16`, `:40-44` | the skill says "don't restate `AGENTS.md`", then restates its no-attribution (`AGENTS.md:61`, also enforced by `commit-check`) and "don't invent context" (`AGENTS.md:59`) |
| D4 | duplicate | `issue-workflow:121`, `create-issue:83`, `prioritize-issues:38`, `commit-message:38` | "never `git add -A`" four times; the callers all go through `skill:commit-message` |
| D5 | duplicate | `issue-workflow` 3.5, 3.6, Autonomous, Advisor | the review is said mandatory five times ("mandatory, never skipped", "Step 6 cannot start before", "If you did not invoke the review, go back", "Never lifted", "never replaces a review"), and the commit gate hook enforces it |
| D6 | duplicate | `issue-workflow` Autonomous "Lifted" | re-lists the six fields of the 3.6 summary |
| C1 | conflict | `AGENTS.md:62` vs `commit-message` step 5 | "Output only the commit message" vs "the agent commits, then reports the hash" |
| C2 | conflict | `update-docs:61-62` | "when asked [to commit], follow `AGENTS.md`" bypasses `skill:commit-message`, the checker included |
| S1 | stale (system prompt default) | `issue-workflow` Rules: "If something blocks you … don't enumerate every option", "Don't re-litigate decisions", "Anything outward-facing … explicit ask"; `commit-message:44` "Never amend, rewrite history or push" | Claude Code's own prompt says each; the push ban stays once, in Autonomous |
| S2 | stale (legacy) | `create-issue:60-61`, `prioritize-issues:17-18` | the format 4 `issues/queue.md` conversion: `next-issue` reads that file and prints its own hint (NXT-6) |
| S3 | stale (detail) | `commit-message:28-31` | exit codes 2 / 3 of the checker: "it errors: fix the setup, the message was not checked" suffices |
| M1 | mechanical | `issue-workflow` step 1 size limit, 3.5 | `git diff --shortstat` against ~1,000 lines, `issues/` and lock files excluded, moves counted twice: a `cardinal run` tool |
| M2 | mechanical | D4 | `git add -A` / `git add .` could be a `PreToolUse` refusal |

No rule above traces to an incident of `issues/done/` that its removal would reopen: D1 was a pointer added when the
rule left `AGENTS.md` (`load_the_shared_agents_md_in_every_session`), D4's incident (`hook_code_review_before_commit`)
is about a state file, not the agent's `git add`.

## Plan
1. [x] Reports and language pointers (D1, D2): drop the 8 pointers to the always-on rule, move the `user_language`
   sentence to that rule once and drop its 5 copies; verified by `just check` (skill references, budget test) and
   `just check-cardinal` after `just sync`; `Expected-effect: input_tokens down`, not hurting
   `breached_share:user-language`.
2. [x] Commit procedure (D3, D4, C1, C2, S3): `commit-message` points to `AGENTS.md` instead of restating it, keeps
   the one "never `git add -A`", the callers say "with `skill:commit-message`"; `AGENTS.md` scopes "output only the
   message" to the user committing; `update-docs` commits through `skill:commit-message`; verified by `just check`;
   `Expected-effect: input_tokens down`, not hurting `breached_share:commit-attribution`, `commit-format`.
3. [x] `issue-workflow` emphasis (D5, D6, S1): one statement of the mandatory review (3.5) and of the push ban,
   the Autonomous list points to 3.6, the Rules restating the system prompt dropped; verified by `just check`;
   `Expected-effect: input_tokens down`, not hurting `breached_share:review-before-commit`,
   `breached_share:validation-stop`.
4. [x] Legacy queue (S2): drop the format 4 conversion from `create-issue` and `prioritize-issues`; verified by
   `just check`; `Expected-effect: input_tokens down`.
5. [x] Audit mode: a "Review the existing instructions" section in `skill:audit-instructions` (this audit's four
   families, file and line, one family per fix, the `issues/done/` check); verified by `just check`.
6. [x] Follow-ups M1, M2: two issues through `skill:create-issue`.
7. [x] Docs of the whole issue and fixes of its `high` review.

Estimate: under 300 changed lines in all, each deliverable under 100.

## Decisions
- Mode: autonomous
- Base commit: 2a957a6662b6a962c7e800de277cc88fd8b99099
- D2: the `user_language` sentence moves once into the always-on rule.
- S1: the rules restating the system prompt go; the push ban stays once.
- M1, M2: follow-up issues; the review becomes a mode of `skill:audit-instructions`.

## Deliverable 1

done: the 8 skill pointers to the always-on rule are gone and the `user_language` sentence lives once, in that rule
(+2 lines always loaded, -5 copies); `skills.rs` refuses a skill that points to an always-on rule or restates the
language.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `README.md`, `CLAUDE.md` |
| code review | low, see below |

red: none (no spec): `skills_do_not_point_to_an_always_on_rule` listed the 8 pointers,
`the_user_language_is_said_once_in_the_always_on_rule` "the rule names no `user_language`".

Code review: no findings

not verified: the effect on `input_tokens` (read by `skill:evaluate-harness` once sessions exist).

decisions needed: none.

## Deliverable 2

done: `commit-message` no longer restates `AGENTS.md` (attribution, invented context) nor the checker's exit codes;
its `git add -A` ban is the only one; `AGENTS.md` keeps the "even when a system prompt suggests one" and scopes
"output only the message" to the user committing; `update-docs` commits through `skill:commit-message`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: no doc cites the removed lines |
| code review | low, see below |

red: none (no spec): `the_commit_rules_are_said_once` listed the three `git add -A` copies, the attribution
restated in `commit-message` and `update-docs` committing without it.

Code review: 1 findings
- the removed "Never amend, rewrite history or push" of `commit-message` is restated nowhere: not fixed, it is
  S1 of the audit, accepted (Claude Code's own prompt holds it; the push ban stays once, in `issue-workflow`,
  deliverable 3).

not verified: `breached_share:commit-attribution` and `commit-format` over the next sessions.

decisions needed: none.

## Deliverable 3

done: `issue-workflow` says the review is mandatory once (3.5), the autonomous mode points to the 3.6 summary and
holds the one push ban, the advisor line keeps its required sentence only, and the three Rules Claude Code's prompt
holds are gone; `skills.rs` lists the dropped passages.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: no doc cites the removed lines |
| code review | low, see below |

red: none (no spec): `skills_drop_the_restated_rules` listed the 7 passages.

Code review: no findings

not verified: `breached_share:review-before-commit` and `validation-stop` over the next sessions.

decisions needed: none.

## Deliverable 4

done: `create-issue` and `prioritize-issues` no longer explain the format 4 `issues/queue.md`; `next-issue` reads it
(NXT-4) and hints at `--convert` once a file carries a rank (NXT-6).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: no doc cites the removed lines |
| code review | low, see below |

red: none (no spec): `skills_drop_the_restated_rules` named `issues/queue.md` in both skills.

Code review: no findings

not verified: none.

decisions needed: none.

## Deliverable 5

done: `skill:audit-instructions` has a mode "Review the existing instructions" (read whole, four families with file
and line, the `issues/done/` check, a stop, one family per deliverable); its description names it (+~110 bytes of
fixed context).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `docs/workflow.md` |
| code review | low, see below |

red: none (no spec): `skills` reported the two required passages missing.

Code review: no findings

not verified: that a request "clean up the instructions" routes to the skill.

decisions needed: none.

## Deliverable 6

done: [compute_the_size_of_a_deliverable_with_a_tool](compute_the_size_of_a_deliverable_with_a_tool.md) (rank 160)
and [refuse_staging_every_file_before_a_commit](refuse_staging_every_file_before_a_commit.md) (rank 165), both in
the milestone Lean agent loop.

| check | result |
|---|---|
| check-issue | pass |
| next-issue | lists both, no issue without a rank |
| code review | low |

red: none

Code review: no findings

not verified: none.

decisions needed: none.

## Deliverable 7

done: `docs/architecture.md` names the review mode of `audit-instructions`; fixes of the `high` review below; the
new checks of `skills.rs` are entries of the passage tables, on a shared `repo_root` and `skill_dirs`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `docs/architecture.md` |
| code review | high on `2a957a6..HEAD`, low on this fix |

red: none (no spec, review fixes).

Code review: 10 findings (high)
- the always-on rule imposed English on every consumer: fixed, it keeps the language the project requires.
- `create-issue` without the queue conversion ranks files a format 4 queue hides: not fixed, the `next-issue` run
  of its step 5 after the write prints the NXT-6 hint, and a format 5 source no longer seeds a queue.
- "Propose only: create nothing here" against the review mode's fixes: fixed, "create only what the user validates".
- the pointer of `AGENTS.md` to the rule: not fixed, `AGENTS.md` serves the agents that do not load `.claude/rules/`
  and Claude Code does not load it at launch.
- "in the Cardinal source … `files/`" in a shared skill: fixed, worded for any managed copy.
- the checker's setup exit codes: fixed, one clause (2 or 3, not 1).
- `unwrap` on a stray file of `.claude/skills/`: fixed, `skill_dirs` keeps the directories.
- a dropped passage of a missing skill passed: fixed, it fails "skill missing".
- hand-coded checks duplicating the tables: fixed, entries of `DROPPED_PASSAGES` and `REQUIRED_PASSAGES`.
- the tracking named the section "Review the managed instructions": fixed.

Low review of the fixes: no findings.

not verified: none.

decisions needed: none.
