# Report the evolution of the harness by revision

# Goal
On demand, tell whether a harness revision made the agent more efficient, the work better and the rules better
followed, compared to the previous ones, with the confounders shown next to the numbers. Also show what the harness
costs in every session, and which skills and hooks earn their place.

# Findings
* 2026-10-26, skill calls over all the transcripts: `code-review` 272, `issue-workflow` 98, `commit-message` 78,
  `create-issue` 46, `update-docs` 32, `write-spec` 21, `prioritize-issues` 1, `audit-instructions` 0.
* Model mix of the assistant turns: 15629 `claude-sonnet-5-5`, 2298 `claude-opus-5-5`, 1550 `claude-sonnet-5`;
  10 Claude Code versions in 30 days. A revision compared to another often differs by model and version too.
* The fixed context of the harness (`CLAUDE.md`, `AGENTS.md`, skill descriptions, hook outputs) is paid in every
  session; `CLAUDE.md` and `AGENTS.md` alone are 4.4 kB here.

# Approach
1. A skill (`evaluate-harness`, name to confirm) that runs the extractors of the issues below and writes a report
   per harness revision: efficiency (tokens, turns, user interventions per issue), quality, rule breaches, hook
   refusals and their false positives, skill usage (unused skills, skills called where another fits), fixed
   context cost of the revision.
2. Show next to each number the model, Claude Code version and project mix, and the sample size; refuse to
   conclude below a threshold (to pick).
3. Optionally commit an aggregate snapshot per revision (numbers only, no project name, no message text) so the
   history outlives the transcripts: to decide.
4. Report to the user as `AGENTS.md` section "Reports to the user" says (a table per axis, the trend only).

# Specs
* `specs/harness-report.md` (REP), new.

# Resources
* `issues/done/extract_session_metrics_from_the_transcripts.md`, `issues/done/detect_rule_breaches_in_the_transcripts.md`,
  `issues/done/measure_the_delivered_quality_from_the_git_history.md`
* `.claude/transcripts/delivered_quality.rs --summary` (quality signals per revision, `specs/delivered-quality.md`)
* `files/.claude/skills/audit-instructions/SKILL.md` (sibling: it proposes rules, this one measures them)
* `files/AGENTS.md`, section "Reports to the user"; `.claude/harness.toml` (`confidential`)

# Hints
* Numbers become targets once shown: report trends, never pass/fail thresholds on agent behaviour.
* The report must hold nothing specific to one consumer when it lands in a shared file.

# Tracking

Plan: validated

Status: done (2026-10-26)

## Understanding
On demand, one markdown table per axis (efficiency, quality, rules, skills, fixed context), one row per group of
consecutive revisions holding enough sessions, with model, version and project mix and the sample size. Out of scope:
verdicts, causes (the replay issue), false positives of refusals and misplaced skill calls (judgment). Means: two
managed scripts (`fixed_context.rs`, `harness_report.rs` joining the TSV outputs of the three extractors) and a skill
`evaluate-harness` that runs them.

## Plan
1. [x] Fixed context: `fixed_context.rs`, bytes of `CLAUDE.md`, `AGENTS.md` and skill descriptions per project and
   revision; verified by `test_fixed_context.rs` on fixture transcripts and repositories; makes true REP-1, REP-2.
2. [x] Report, efficiency axis: `harness_report.rs` reads the inputs, groups the revisions, prints the efficiency
   table with the confounders; verified by `test_harness_report.rs` on fixture TSV files; makes true REP-3 to REP-5,
   REP-8.
3. [x] Report, other axes: quality, rules, skills, fixed context tables; verified by the same test; makes true
   REP-6, REP-7 (spec `accepted`).
4. [x] Skill `evaluate-harness`: runs the scripts (a quality summary per repository) and reports the trend per axis;
   architecture, README, `audit-instructions` cross-reference; verified by `test_skills.rs`.
5. [x] Fixes of the `high` review: symbolic links, sorted and unquoted skill names, multi-line values, one pass of
   the pins per repository, the revision rule shared in `transcripts.rs`, empty sums and rule rows for a group with no
   input, the archive caveat in the skill; verified by `test_fixed_context.rs` and `test_harness_report.rs`; amends
   REP-2, REP-6.

## Decisions
- Mode: autonomous
- Base commit: ed19f9330c6ddf19e59703666509ed4a9afb138c
- Unit of comparison: groups of consecutive revisions closed at `--min-sessions` sessions (default 10).
- No committed snapshot: the session archive (EVA-11) and the git history suffice; `--anonymous` allows one later.
- False positives of the refusals and misplaced skill calls: out of scope (judgment).
- Skill name: `evaluate-harness`.

## Deliverable 1
- done: `fixed_context.rs` (bytes of `CLAUDE.md`, `AGENTS.md`, skill `name` and `description` per project and
  revision, from the commit the revision came in force with; the skills shipped), architecture, README; REP-5 now
  counts the models and versions by session (the rows give no turns per model).
- red: the script did not exist, every check failed.
- Smoke run on the real transcripts: 44 rows in 3 s; the skill descriptions cost 1.9 to 3.1 kB per revision.
- Code review: 2 findings, both fixed: a description over several lines (`>` block) counted 1 byte, a nested `name:`
  counted (REP-2 amended, fixture extended).

## Deliverable 2
- done: `harness_report.rs` (inputs checked by header, groups of consecutive revisions closed at `--min-sessions`,
  hashes cut to 7, `## Efficiency` table with medians and the model, version and project mix, `--anonymous`);
  architecture, README; REP-3 to REP-5 made precise (headings, median, input token columns).
- red: the script did not exist, every check but the refusals failed.
- Smoke run on the real transcripts: 77 sessions with a revision, 6 groups of 10 to 17 sessions and a last one `too few`.
- Code review: 1 finding, not fixed here: the other inputs are read but print no table yet, deliverable 3; a second
  review after the tracking was written: no findings.

## Deliverable 3
- done: `## Quality`, `## Rules`, `## Skills` (calls, hook refusals, unused skills) and `## Fixed context` tables,
  each printed with its input; a `too few` group is one row per table; spec `accepted`; architecture, README.
- red: the eight checks of the new tables failed (no table printed).
- Smoke run on the real data: every table in under 0.1 s; the rules table is the longest (one row per rule and group).
- Code review: no findings (noted: a group with no rule row of its revisions has no row in `## Rules`, as REP-6 says).

## Deliverable 4
- done: skill `evaluate-harness` (collect the five outputs in a temporary directory, read the trend per axis
  against the confounders, report `metric | groups | trend`, `--anonymous` for a shared file, never commit);
  `audit-instructions` points to it; architecture, README.
- red: `evaluate-harness: skill missing` for the five passages of `test_skills.rs`.
- Code review: 1 finding, fixed: the outputs went to bare relative names, so in the repository; every path now
  goes under the `mktemp -d` directory.

## Deliverable 5 (fixes of the `high` review)
- Fixed (9 of 10 findings): a symbolic `CLAUDE.md` counted as its target (REP-2); skills sorted, `core.quotePath=false`;
  blank lines and quotes over several lines in a description; the pins of a repository read once; `pinned_revision`
  shared by `revision` and `fixed_context.rs` (every including script restamped); a group with no quality row has
  empty sums and a group with no rule row one empty row (REP-6); the quality rows borrowed, not cloned; the skill
  warns that an archive older than the transcripts has no other axes.
- Not fixed: two repositories with the same directory name share a row: the project is the name everywhere (EVA-7),
  so the report could not tell them apart anyway.
- red: the bytes of the link, the order of `review` and `review-pr`, the empty quality sums and the empty rule row
  failed.
- Code review: no findings.
