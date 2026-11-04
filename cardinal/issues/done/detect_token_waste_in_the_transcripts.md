---
rank: 112
why: 'after the skills due, which extends rule-breaches the same way; needs the cache share and the tokens per issue to weigh its findings'
---
# Detect token waste in the transcripts

# Goal
The report shows how many tokens a revision spends but not where they are wasted. Known patterns of agent waste are
mechanical and can be found without a model: a file read again though unchanged, the same tool call repeated with
the same input, a retry loop on the same error, a tool result far larger than what the agent used. The expected
outcome: these are findings with their location, counted per revision, so a change of the instructions or of a
tool gets a verdict on the waste it removes, and the worst patterns can be read in the transcripts.

# Findings
* `rule-breaches` already reads the transcripts through `transcripts.rs` and prints findings with a `location`
  (`<file>:<line>`) and a summary per rule and revision (BRE-1, BRE-3), which the rules table of the report reads
  (REP-6).
* An article on agent token waste claims up to 30% wasted tokens with no named study; the figure is unverified, the
  patterns above are the ones it and the usual tracing tools name.
* Not measured: how often each pattern occurs in the user's sessions.

# Approach
Measure first: count each pattern over the existing transcripts before deciding which become rules.
1. Rules of `rule-breaches`: `reread-unchanged` (a `Read` of a path with no `Edit`, `Write` or `Bash` call in
   between that may have changed it), `repeated-call` (same tool, same input), `retry-loop` (N failing calls in a row
   with the same error), `oversized-result` (a tool result over a threshold in bytes).
2. Or a separate `token-waste` tool with an estimate of the tokens per finding (bytes of the result), if a rule's
   count of sessions applied and breached says too little.
3. Only if `retry-loop` proves frequent: a shared `PreToolUse` hook that refuses the N-th identical failing call.
Likely over the size limit: split at `issue-workflow` step 2 (measure, rules, hook).

# Specs
* `specs/rule-breaches.md`: one requirement per rule kept.

# Milestone
Lean agent loop

# Resources
* `specs/rule-breaches.md`, `crates/cardinal/src/tools/rule_breaches.rs`, `transcripts.rs`.
* `issues/done/count_the_skills_a_session_should_have_loaded.md`: adds rules to `rule-breaches` the same way.

# Hints
* A `Bash` call can change a file without naming it: `reread-unchanged` must treat it as a possible change, or it
  counts legitimate rereads.
* Some re-reads are needed after a compaction dropped the file from the context: a compaction boundary resets it.

# Tracking

Plan: validated

## Understanding
Add the one waste pattern the measure shows to `rule-breaches` (BRE-19 `oversized-result`), each finding weighed by
the bytes of its result in `detail`, so `breached_share:oversized-result` gives a verdict per revision. Out of scope:
`reread-unchanged`, `repeated-call`, `retry-loop`, the separate `token-waste` tool and the `PreToolUse` hook (dropped by the measure).

## Measure
Over the local transcripts (1611: 440 sessions and their sub-agents), the `assistant` and `user` lines only (a first
count also read the `api-request-blob` lines, copies of the calls sent to the API, and doubled every call), findings /
transcripts:
- `reread-unchanged`, no `Bash` in between: 0 / 0; with `Bash` ignored: 3 / 2.
- `repeated-call`, no edit in between: 524 / 235, of which `Skill` 501 (a `code-review` per deliverable), `Bash` 21,
  `Read` 2. Dropped: not waste, or too rare.
- `retry-loop`, same tool and error in a row: 2 in a row 4 / 4, 3 in a row 2 / 2. Dropped, and with it the hook.
- `oversized-result`: results over 10 KB 1521 / 935 (26.5 MB of 67.3 MB of results), over 25 KB 193 / 177 (6.4 MB;
  `Bash` 112, `Read` 75), over 50 KB 13 / 13. Kept.
- `rule-breaches` with both rules, over the 200 sessions with a Cardinal revision: `oversized-result` breached in 82,
  119 breaches; `reread-unchanged` applied in 83, breached in none.

## Plan
1. [x] Waste rule: `oversized-result` in `rule-breaches`, main and sub-agent transcripts; verified by a test of
   `crates/cardinal/tests/rule_breaches.rs` through the binary; makes true BRE-19.
2. [x] Fixes of the `high` review: the fixture holds the text the output must not print, the summary test counts
   the waste rule, a non-goal for the image parts; verified by the same test; makes true BRE-19.

## Decisions
- Mode: autonomous
- Oversized threshold: 25,000 bytes (11% of the transcripts; 10 KB flags 58%).
- `reread-unchanged` dropped after the corrected measure (0 transcripts); BRE-19 is `oversized-result`.
- Base commit: 6ed761512da1d1183a833544f31d7aa5c990b140

## Summary of deliverable 1
done: rule `oversized-result` in `rule-breaches`, a tool result over 25,000 bytes, its size as `detail`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BRE-19 traced, spec `accepted`) |
| docs | `docs/usage.md`, `docs/architecture.md`, the module doc |
| code review | see below |

red: BRE-19: `FAIL - a tool result over 25,000 bytes is a breach at its line, in any transcript`

Code review: no findings

not verified: none

decisions needed: none

## Summary of deliverable 2
done: fixes of 4 of the 8 findings of the `high` review of `6ed7615..4f10461`; follow-up issue for 2 others.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | module doc rewrapped; non-goal of `specs/rule-breaches.md` |
| code review | see below |

red: none

Code review: no findings

High review, not fixed: a resumed session counting the results it copies (no tool result of the 23,662 local ones
is in two sessions); the result text built for each result to read its length (one copy, negligible beside the
JSON parse); the bytes not summed per revision and the rule read as a hook candidate: issue
[weigh_the_token_waste_per_revision](weigh_the_token_waste_per_revision.md).

not verified: none

decisions needed: none
