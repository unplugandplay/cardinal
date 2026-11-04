---
rank: 175
why: 'last of the new ones: the costliest; the observational verdicts come first and show whether it is needed'
---
# Replay a reference task set to compare two harness variants

# Goal
Every verdict of Cardinal is observational: it compares revisions over the real sessions that happened to run on
them, with different issues, projects, models and versions. A change of a skill or of the model can only be judged
after the fact, and a regression that shows once in two runs is easily missed. The outcome: a small set of fixed tasks
on a test repository, run headless several times against two variants of the harness (or two setups), reported with
the same metrics as `cardinal-report`.

# Findings
* `cardinal-report` groups sessions by revision and names the confounders (models, versions, project mix): REP-4,
  REP-5; it has no controlled comparison.
* The article on folding subagents recommends about ten reference tasks to start, graded on the final answer, each run
  several times: in its support-agent test a regression appeared in only one run of two.
* Measured (2026-10-31, `just replay f3c6cb1 2`, Opus 5.5, Claude Code 2.1.312, the one-deliverable fixture issue):

  | run | duration | cost (API equivalent) | turns | output tokens | cache read | commits | graded |
  |---|---|---|---|---|---|---|---|
  | 1 | 203 s | $0.89 | 19 | 6,639 | 549,968 | 2 | pass |
  | 2 | 149 s | $0.94 | 25 | 8,747 | 664,151 | 2 | pass |

  Both runs worked the issue to the end with no stop: the `low` reviews, the `high` review of the whole issue, its
  fixes in a second commit, then the question to close. They differed where a controlled comparison is wanted: run 1
  called no `commit-message` nor `update-docs` (skills `code-review=3;issue-workflow=1`), run 2 did; run 1 left 3
  findings of the `high` review, run 2 fixed all 5. `session-metrics` reads their transcripts (project
  `replay-fixture`, revision the pinned commit).
* Grading is automatic (the held-out test, the commits, the review lines); reading the final answer took about a
  minute a run. The skills called, read from `session-metrics`, are the cheapest signal of a rule kept or not.
* Headless in a fresh copy, Claude Code ignores the allow rules of `.claude/settings.json` (the workspace is not
  trusted) but runs its hooks; `--permission-mode auto` was enough, no command was refused.
* Cost of the candidate: ten tasks, three runs each, two variants: 60 runs, about $55 of API equivalent and three
  hours in a row.

# Approach
1. Measure first: one task (a small issue of a fixture repository) run headless twice, its duration, tokens and the
   effort to grade it.
2. Candidate: a fixture repository with about ten small issues, a recipe that syncs a given Cardinal revision into a
   copy, runs each issue N times and feeds the transcripts to `session-metrics` and `delivered-quality`.
3. Grading: the tests of the fixture pass, the review gate held, the rules of `rule-breaches` kept.
Likely over the size limit: split at `issue-workflow` step 2 (fixture, runner, report).

# Milestone
Lean agent loop

# Resources
* `specs/evaluation.md`, `specs/delivered-quality.md`, `specs/cardinal-report.md`.
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop

# Hints
* No network in the unit tests: the runner is a recipe, not a test of `cargo test`.
* The tasks must need no answer from the user, or the run stops at the stop of step 2: hand the plan pre-validated.

# Tracking

Plan: validated

## Understanding
This issue is cut to its first step: measure what one headless run of a small fixture issue costs (duration, tokens,
cost, grading effort) before building the ten-task set. Out of scope here: the ten tasks, the comparison of two
variants, the report; they become new issues only if the measure says the replay is affordable.

## Plan
1. [x] Fixture: `fixtures/replay/`, a tiny Rust crate (outside the workspace) with one small issue whose `# Tracking`
   holds a validated plan in autonomous mode, so the run never stops for the user; verified by `cargo test` failing on
   the fixture as handed over, then passing with the expected change applied by hand (not committed); no requirement.
2. [x] Runner: a `replay <rev> <runs>` recipe of the `justfile` (this repository only, not `files/`) that copies the
   fixture to a fresh directory per run, commits it, runs `cardinal init` at `<rev>` of this repository, runs
   `claude -p` on the issue with `--output-format json`, then grades it (fixture tests, the review gate held: a commit
   exists and `issues/` records `Code review`) and prints one tab-separated line per run (duration, cost, tokens,
   turns, graded); verified by a dry run with `runs=1`; no requirement.
3. [x] Measure: two runs at `HEAD`, their lines and the grading effort written in `# Findings`, with the go or no-go
   for the other cuts (ten tasks and grading; the comparison of two variants with the metrics of `cardinal-report`).

## Decisions
- Mode: autonomous
- No spec: a measure and a recipe of this repository, not a capability of `cardinal` (step 1 skips the spec for a measure).
- Base commit: 1d828de0912389a281eff9c5defef4e8f09e7002
- Headless runs use `--permission-mode auto`, `--max-budget-usd 5` (a capped run counts as failed) and the model
  of the user's configuration.
- The other cuts (ten tasks and grading; the comparison) are written as issues only after the measure, on a go.

## Deliverable 1: fixture
done: `fixtures/replay/` (crate, spec TXT, issue with a validated plan, held-out `grade/grade.rs`); `fixtures` added to
`skipped_dirs` so `check-specs --strict` does not read its citations.

| check | result |
|---|---|
| tests | `just check` passes |
| lint | clean |
| specs | `check-specs --strict` passes |
| docs | `CLAUDE.md` index, `update-docs-map.md` row |
| code review | see below |

red: none (a fixture; checked by hand: the grading test fails to compile on the fixture as handed over, `unresolved
import replay_fixture::word_frequencies`, and its 4 tests pass with a reference `word_frequencies` not committed)
Code review: no findings

not verified: none

decisions needed: none

## Deliverable 2: runner
done: `just replay <rev> <runs> [claude]` in the `justfile`: a copy per run under `/tmp/cardinal-replay.*/replay-fixture`
(kept), `cardinal init` of this repository's `.git` at `<rev>`, the setup committed before the review gate is
installed, `claude -p` with the decided flags, then the grade (`commits` since the base, `reviews` lines of the issue,
`graded` by the held-out test) next to the fields of the JSON result.

| check | result |
|---|---|
| tests | `just check` passes |
| dry run | a stub `claude` applying the reference solution: `success`, `reviews` 1, `graded` pass; `true` as `claude`: the line `unreadable`, `graded` fail |
| docs | `CLAUDE.md` index |
| code review | see below |

red: none (a recipe; the dry run first printed no line for an empty result, fixed with `jq -e`)
Code review: 1 finding
- `reviews` counted the review lines the handed issue may already hold: fixed, the count before the run is subtracted.

not verified: a real `claude -p` run (deliverable 3)

decisions needed: none

## Deliverable 3: measure
done: the two runs written in `# Findings`; the fixture fixed from what they showed: the internal reason of its TXT-1
test (`check-specs` warned), its `Cargo.lock` committed (each run left it untracked).

| check | result |
|---|---|
| tests | `just check` passes |
| runs | 2 of 2 `success`, graded pass |
| fixture | `check-specs` in a fresh copy: only TXT-2 and TXT-3 untraced, the task |
| code review | see below |

red: none (a measure)
Code review: no findings

not verified: the variance over more than two runs

decisions needed: go or no-go for the other cuts (ten tasks and grading; the comparison of two variants)

## Whole issue: docs and `high` review
`update-docs` over `1d828de..HEAD`: nothing more (the recipe and the fixture belong to this repository only, the
`CLAUDE.md` index names them; `docs/` describes what a consumer gets).

Code review: 10 findings (`high`, `1d828de..HEAD`)
- `graded` ignored the subtype and the review gate: fixed, `graded` pass needs `success`, a commit, a review line and
  the held-out test (now its own `tests` column).
- The fixture was copied from the working tree: fixed, `git archive HEAD:fixtures/replay`.
- `rm` and `test -x` of the pre-commit hook aborted on a revision older than the native gate: fixed, both only when the
  first sync installed it (dry run at `1d828de` passes).
- `reviews` counted the `| code review |` table rows: fixed, counted as QUA-6 (not a line starting with `|`).
- A relative `claude` path broke after the `cd`: fixed, resolved first (relative to the repository root, where `just`
  runs a recipe).
- The held-out test is reachable through the source the copy's `.claude/cardinal.toml` names: not fixed, a limit for
  the task-set cut (a grade kept outside the source).
- The run loads the user's own configuration (`~/.claude/CLAUDE.md`, skills, MCP servers): not fixed, it conflicts with
  the decision to use the model of the user's configuration; a decision for the comparison cut.
- `skipped_dirs` skips a `fixtures` directory at any depth: not fixed, no other one exists and the key's semantics is
  the tools'.
- The recorded base is the setup commit, the `commits` column counts from the next one: not fixed, that commit only
  touches `issues/`, which `deliverable-size` leaves out.
- Runs are sequential: not fixed, out of the measure; for the comparison cut.

Code review: no findings (`low`, the fixes)

not verified: a `graded` pass through the new rule on a real run (the measured runs predate it, both would pass: success, 2 commits, review lines, tests)

## Outcome
No-go on the ten-task set and the comparison (user's decision, 2026-10-31): about $55 of API equivalent at the least
for measures no current change of the harness needs. The fixture and `just replay` stay for a one-off check (the
fixture task, three runs per variant, about $6); no issue is written for the other cuts.
