# Delegate the exploration of the issue workflow to a subagent

# Goal
The main session of `skill:issue-workflow` reads the code, docs and history itself at step 1 and step 3.1, and
everything it reads stays in its context until the end of the issue. That growth is what the size limit of step 1
(~3,000 lines, 6 deliverables per issue) guards against. A read-only subagent (the built-in `Explore` type) would
return only its conclusion, so the main context would grow more slowly and an issue could keep its quality longer.
The expected outcome: the skill tells the agent when to delegate a broad exploration, and the evaluation can tell
whether that made the sessions cheaper or not.

# Findings
* The workflow already delegates one step: `code-review` runs forked, and the review gate records its marker on the
  `PostToolUse` of the `Skill` tool (`crates/harness/src/tools/review_gate.rs`, module doc).
* `session-metrics`, `rule-breaches` and `delivered-quality` already read the subagent transcripts of a session
  (`subagents(&path)`).
* `session-metrics` adds the subagent transcripts to their session (`session.read(&sub, false)`): `input_tokens` and
  the cache columns are the sum of the main thread and its subagents. No metric shows the size of the main context,
  which is the benefit expected here; a delegation could even raise the summed tokens, since each subagent starts
  cold and rereads the instructions.
* No skill of `files/` mentions a subagent today; the repository ships no `.claude/agents/` definition.

# Approach
Measure first: the current metrics cannot show the expected benefit.

1. Add a metric of the main thread only to `session-metrics` (e.g. the peak of the input and cache tokens of one
   turn of the main transcript, or the main-thread tokens apart from the subagents' ones), so a verdict can read it.
2. In `issue-workflow` step 1 and step 3.1, tell the agent to hand a broad exploration (many files, history,
   conventions) to an `Explore` subagent and to keep only its conclusion; a targeted read of a known file stays in
   the main thread. No custom agent definition, so the fixed context does not grow.
3. Commit the skill change with `Expected-effect:` lines (the new metric `down`, and `input_tokens` if it should
   move) and let `skill:evaluate-harness` give it a verdict once enough sessions ran.

Left out unless the verdict is good: a subagent writing the tests of step 3.2 from the spec alone, a subagent
running `skill:update-docs`. The spec, the plan, the implementation and the commit message stay in the main thread
(they need the dialogue with the user and the recorded decisions).

# Specs
* `specs/evaluation.md`: a new requirement for the main-thread metric (1); `specs/harness-report.md` (REP-5) to show it, and the `<metric>` list of VRD-1 in
  `specs/harness-verdicts.md` if the metric is to be named in an `Expected-effect:` line.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 1 and 3.1, size limit).
* `crates/harness/src/tools/session_metrics.rs`, `crates/harness/src/tools/transcripts.rs` (`subagents`).
* `specs/evaluation.md`, `specs/harness-verdicts.md`, `files/.claude/skills/evaluate-harness/SKILL.md`.
* `docs/adr/0006-transcript-readers-are-managed-scripts.md`.

# Hints
* A review done by anything other than the `code-review` skill leaves no marker: the commit gate refuses the commit.
  Keep the skill as the entry point of every review.
* The `red:` and `Code review:` lines that `delivered-quality` counts must stay written by the main thread in the
  issue.
* Only read-only subagents are safe in parallel: two that write in the same worktree conflict.
* A wrong or partial conclusion of the subagent is taken as is by the main thread, which never saw the raw files:
  the instruction should ask for the paths and lines behind each conclusion.

# Tracking

Plan: validated

## Understanding
Measure the size of the main context (`main_context_peak`: the largest input + cache of one main-thread turn), then
tell `issue-workflow` to hand a broad exploration to an `Explore` subagent, and record the expected drop for a
verdict. Out of scope: subagents for the tests or `update-docs`, a custom agent definition.

## Plan
1. [x] Main-context metric: `session-metrics` column `main_context_peak` (old archives upgraded), its median in the
   efficiency table, a metric of `Expected-effect:`; verified by tests of `session_metrics`, `harness_report`,
   `harness_changes`/`commit_check`; makes true EVA-16, EVA-17, REP-9, VRD-1 (~250 lines).
2. [x] Delegated exploration: `issue-workflow` steps 1 and 3.1 hand a broad exploration (many files, history,
   conventions) to a read-only `Explore` subagent asking for the conclusion with the paths and lines behind it; a
   targeted read stays in the main thread; `just sync`; commit with `Expected-effect: main_context_peak down`;
   verified by `just check` and `just check-harness` (~20 lines).
3. [x] Fixes of the `high` review: an inline sidechain turn is not a main turn (EVA-16); the upgrade finds the
   column by name; REP-5 names the new column; the skill text keeps the reviews in `code-review` and reads back only
   the lines a decision rests on.

## Decisions
- Mode: autonomous
- Base commit: de1b6477b7422422655a96e3ec0a5007a17d1ea9
- Metric: the peak of one main-thread turn (input + cache), not the main-thread sum, which grows with the session length.
- Archives of the previous header are upgraded with an empty cell, which the median leaves out (EVA-17, REP-9).
- No `Expected-effect: input_tokens`: its direction is unknown (each subagent starts cold).

## Deliverable 1: main-context metric

- done: `session-metrics` prints `main_context_peak` and upgrades the archives written before it; `harness-report`
  shows its median in `## Efficiency`; `Expected-effect: main_context_peak <up|down>` is accepted.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (EVA, REP, VRD back to `accepted`) |
| docs | `docs/architecture.md`, `.claude/update-docs-map.md` |
| code review | low, see below |

- red: EVA-16, EVA-17, REP-9, VRD-1: `FAIL - header names every column` (no `main_context_peak` column); `FAIL - the median main context peak leaves out the sessions with no peak`; `commit_check_refuses_an_expected_effect_it_cannot_read` panicked on `main context peak`.
- Code review: no findings
- not verified: the column on the real transcripts.
- decisions needed: none.

## Deliverable 2: delegated exploration

- done: `issue-workflow` step 1 hands a broad exploration to a read-only `Explore` subagent that cites the paths and
  lines behind each conclusion; step 3.1 points to it; `docs/workflow.md` says so.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | n/a (no capability changed) |
| docs | `docs/workflow.md` |
| code review | low, see below |

- red: none (skill text, no test)
- Code review: no findings
- not verified: whether the agent delegates in practice; the verdict of `main_context_peak down` needs sessions.
- decisions needed: none.

## Deliverable 3: fixes of the high review

`Code review` of `de1b647..HEAD` (high): 9 findings.

| # | finding | done |
|---|---|---|
| 1 | inline sidechain turns counted in `main_context_peak` | fixed, EVA-16 states it |
| 2 | median on fewer sessions than the group counts, verdict not marked | follow-up `issues/done/count_the_sessions_behind_a_partial_metric.md` |
| 3 | `harness-report`/`harness-changes` refuse an old archive | not fixed: `evaluate-harness` runs `session-metrics --archive` first, which upgrades it |
| 4 | `PEAK_INDEX` hard-coded | fixed: position read from `HEADER` |
| 5 | short archived row padded silently | not fixed: the merge never checked row widths, out of this issue |
| 6 | 0 for a session with no main turn | not fixed: EVA-16 says 0, as `turns` |
| 7 | REP-5 and REP-9 disagree on the column order | fixed in REP-5 |
| 8 | "reviews stay in the main thread" contradicts the forked `code-review` | fixed |
| 9 | reading back every cited line undoes the delegation | fixed: only the lines a decision rests on |

- done: the fixes above.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a (the specs carry it) |
| code review | low, see below |

- red: EVA-16: `FAIL - a sub-agent turn written in the main transcript is not a main turn`
- Code review: no findings
- not verified: none.
- decisions needed: none.
