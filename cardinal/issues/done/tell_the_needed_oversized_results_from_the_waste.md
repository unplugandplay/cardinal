---
priority: medium
why: 'the only token waste the report measures, and today it mostly counts the diffs a review must read'
---
# Tell the needed oversized results from the waste

# Goal
`rule-breaches` counts every tool result over 25,000 bytes as `oversized-result` (BRE-19), the token waste
`cardinal-report` weighs per session. Over the last five complete groups it reads 8 to 42 KB per session with no
trend, but most of these results are diffs a reviewer must read, so the measure cannot tell a regression of the agent
from the size of the issues. The outcome: the report separates the results a step needs from those the agent could
have bounded, and the avoidable ones shrink, starting with the report's own output.

# Findings
* Measured 2026-11-03 over 525 sessions of 7 projects: 174 `oversized-result` breaches, 138 in sub-agent transcripts
  and 36 in the sessions' own.
* 121 of the 138 come from the reviewer sub-agents of the built-in `/code-review`: 95 at `high` (the review of a whole
  issue, `skill:issue-workflow` step 6) and 26 at `low`; 2 from `Explore`.
* By tool, all transcripts: `Read` 72 results (3.0 MB), `git` 60 (1.6 MB), `cd …;` commands 17, `cat` 11, `sed` 5.
* The 36 of the sessions' own: `cd …;` 14, `cat` 9, `Read` 7, `git` 2, `WebFetch` 2, `cardinal` 1.
* `cardinal run cardinal-report` printed 30 KB for 19 groups (its rules table alone 197 lines), so the audit that read
  it breached the rule; it has no option to limit the groups.

# Approach
1. Locate each breach by its source in the summary or the report: the sub-agents of a review, other sub-agents, the
   session's own results; candidates: a column per source, or a `--source` filter of `rule-breaches`.
2. Give `cardinal-report` a `--last <n>` (the last `n` groups) and use it in `skill:evaluate-harness`.
3. For the review diffs, measure first how much of a whole-issue diff is the issue file, specs and docs, then decide
   whether the `high` review should leave them out.
4. For the session's own `cat` and `Read` of large files, check whether a rule or a hint would bound them (a range, a
   `head`), and whether it pays its fixed context.

# Specs
* `specs/rule-breaches.md` BRE-19, BRE-20; `specs/cardinal-report.md` (a new requirement for `--last`).

# Milestone
Lean agent loop

# Resources
* `specs/rule-breaches.md`, `specs/cardinal-report.md`, `files/.claude/skills/evaluate-harness/SKILL.md`.
* `files/.claude/skills/issue-workflow/SKILL.md`, steps 3 and 6 (the `/code-review` calls).
* `issues/done/judge_the_delegated_exploration_before_keeping_it.md` (the sub-agents it judges are not the source here).

# Hints
* A sub-agent's oversized result does not enter the main context; it costs the sub-agent's tokens, not the peak.
* A sub-agent transcript has a `.meta.json` next to it with its `agentType`; the review reviewers are
  `general-purpose` with no description and a first prompt that starts with the review level.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
Split the `oversized-result` waste by its source (the session's own results, the `/code-review` reviewers, other
sub-agents), give `cardinal-report` a `--last <n>` used by `skill:evaluate-harness`, then measure the two avoidable
sources (the docs share of a whole-issue diff, the session's own `cat` and `Read`) before deciding any rule.
Means: two new waste rules of `rule-breaches` (BRE-22) rather than a column or a `--source` filter, so the summary,
the rules table and the verdicts need no change.

## Plan
1. [x] Waste by source: `oversized-result` keeps the session's own results, `oversized-result-review` and
   `oversized-result-subagent` take the sub-agents' by their `.meta.json`; verified by tests of `rule-breaches`
   (own, reviewer, `Explore`, no `.meta.json`) and of its summary; makes true BRE-19, BRE-20, BRE-22. (autonomous)
2. [x] `cardinal-report --last <n>`: only the rows of the last `n` groups, verdicts over every group; refused with
   `--by setup` or a bad value; `skill:evaluate-harness` passes `--last 6`; verified by tests of `cardinal-report`;
   makes true REP-17. (autonomous)
3. [x] Measure the avoidable sources: share of `issues/`, `specs/`, `docs/` in the whole-issue diffs of the closed
   issues, and the session's own `cat`/`Read` breaches by file and need; findings and the decision proposed in this
   issue, a follow-up issue for any change of a skill or rule; verified by the numbers recorded here. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: f116984c0003b4b49ad4e4207326f221e426c5b0
- Source of the waste: three waste rules (`oversized-result`, `oversized-result-review`, `oversized-result-subagent`), not a `--source` filter.
- `skill:evaluate-harness` passes `--last 6` (the five complete groups and the open one).
- The review filter and the read hint go to follow-up issues (deliverable 3).

## Deliverable 1
done: `rule-breaches` tells the oversized results by source: `oversized-result` (the session's own),
`oversized-result-review` (a `general-purpose` sub-agent with no description), `oversized-result-subagent` (any other).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BRE accepted, REP draft until deliverable 2) |
| docs | `docs/evaluation.md`, `docs/usage.md` |
| code review | see below |

red: BRE-19, BRE-20, BRE-22: `FAIL - a tool result over 25,000 bytes is a breach at its line, of the rule of its transcript's source`

Code review: no findings (it noted the draft REP-17 in the diff: the spec drafts go with the first deliverable).

not verified: the split on the real transcripts (deliverable 3 reads it).

## Deliverable 2
done: `cardinal-report --last <n>` prints only the last `n` groups (verdicts over all of them, refused with
`--by setup`); `skill:evaluate-harness` passes `--last 6` and tells the three waste rules apart.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (REP accepted) |
| docs | `docs/usage.md`, `files/.claude/skills/evaluate-harness/SKILL.md` (synced) |
| code review | see below |

red: REP-17: `FAIL - --last succeeds` (the option was refused as unknown)

Code review: no findings

not verified: the size of the real report with `--last 6` (deliverable 3 runs it).

## Deliverable 3
Measured 2026-11-03 on the transcripts of this machine, with the rules of deliverable 1:

| rule | sessions applied | breached | breaches | bytes | bytes per session |
|---|---|---|---|---|---|
| `oversized-result` | 290 | 30 | 36 | 1.08 MB | 3,708 |
| `oversized-result-review` | 262 | 108 | 138 | 4.74 MB | 18,078 |
| `oversized-result-subagent` | 6 | 2 | 2 | 52 KB | 8,605 |

* Reviewers' results over the threshold, by kind: 75 whole `Read`s of a `tool-results/` file (the diff Claude Code
  saved because it was large, 3.3 MB), 102 `git diff` (2.8 MB; 21 already left paths out), 13 other: the waste of the
  reviews is the diff itself, read whole.
* Whole-issue diffs (`<base>..<close>^`) of the 204 closed issues with a base commit, 7 projects, by changed lines:
  code and tests 64%, `issues/` 13%, `.claude/` 9% (synced copies of Cardinal), `files/` 8%, `specs/` 2%, `docs/` 2%;
  median share of `issues/`, `specs/` and `docs/` per issue 29%.
* The session's own results over the threshold, in every session of the machine (69, of which the 36 above in
  the sessions with a Cardinal revision): `Read` 24 (10 whole code files, 8 whole `tool-results/` files, a result
  Claude Code had cut), `cat` 22 (code 12, docs 5, other 5), `cd …;` commands 8, `WebFetch` 7, others 8.
* `cardinal-report` with sessions, rules and fixed context: 28.8 KB in full, 8.5 KB with `--last 6`.

Code review: 1 findings: the breakdown of the session's own results did not add up to 36 (it counted every session
of the machine); fixed, its scope stated.

Decisions, validated by the user:
* The `high` review leaves out `issues/` and the synced copies (`.claude/`, the paths of `.claude/cardinal.lock`):
  about a fifth of the lines, none of them code a review can fault; `specs/` and `docs/` stay (4%, the
  requirements the code is read against): `issues/done/leave_the_issue_file_and_the_synced_copies_out_of_the_issue_review.md`.
* No rule in the fixed context for the session's own reads: 3.7 KB per session does not pay a line every session
  loads; a `PreToolUse` hint on a whole `Read` of a file over 25 KB (`tool-results/` included) costs nothing until
  it fires: `issues/done/hint_before_a_whole_read_of_a_large_file.md`.

## Deliverable 4 (whole-issue review)
`/code-review high` on `f116984..HEAD`: 7 findings. Code review of the fixes: no findings. (autonomous)
* Fixed: `skill:evaluate-harness` ran the `--by setup` report with `--last 6`, which the tool refuses; the
  duplicated helper of the `--last` test.
* Not fixed, by design: the reviewer heuristic of BRE-22 (no other `general-purpose` sub-agent with no description
  among 1,493 on the machine; the first prompt is the built-in's text, no steadier); the verdicts listing every
  change (REP-17 keeps them, a verdict is the point of a change); `--last` counting the `too few` group (REP-4
  shows it as a group); the hidden groups computed (a few medians); `hidden` on `Group` (every table reads it through
  `table()`, an index would go through each of them).
* Noted: the `Expected-effect` of `c652c48` is read by no verdict (no change of `files/`).
