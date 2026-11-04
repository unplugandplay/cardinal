---
rank: 100
why: 'after the exploration verdict: cheap, reuses rule-breaches; gives the skill changes a routing signal'
---
# Count the skills a session should have loaded

# Goal
A single agent loop works only if the agent loads the right skill at the right moment. The evaluation counts the skills
called and those never called (REP-7), but not the moments a skill was due and skipped: a commit made without
`commit-message`, an issue worked without `issue-workflow`, a spec written without `write-spec`. The outcome: such
misses are findings of `rule-breaches`, so a change of a skill description or of the instructions gets a verdict on its
routing.

# Findings
* `rule-breaches` already judges rules from the transcripts and the git history (BRE-4 to BRE-10) and feeds the rules
  table of `cardinal-report` (`applied`, `breached`).
* REP-7 lists `unused` skills per group: a skill never called, not a skill missed when due.
* The article on folding subagents recommends checking, in the evals, that the right skill or tool was loaded.

# Approach
1. Pick the triggers that are deterministic from a transcript: a `git commit` call with no `commit-message` skill call
   before it in the session; a `Plan: proposed` write with no `issue-workflow` call; a new file under `specs/` with no
   `write-spec` call.
2. One rule per trigger (`skill-routing-<skill>` or one rule with the skill as detail), `applied` when the trigger
   occurs.
Measure first how often each trigger occurs; drop a trigger that never fires.

# Specs
* `specs/rule-breaches.md`: new requirements, one per routing rule.

# Milestone
Lean agent loop

# Resources
* `specs/rule-breaches.md`, `crates/cardinal/src/tools/` (the `rule_breaches` module), `specs/cardinal-report.md` REP-6.

# Hints
* A user who commits by hand or asks for a quick fix legitimately skips a skill: the rule is a signal, never a gate.

# Tracking

Plan: validated

## Understanding
Add three `skill-routing-<skill>` rules to `rule-breaches` (BRE-15 to BRE-18): a commit without `commit-message`, a
`Plan: proposed` without `issue-workflow`, a new spec without `write-spec`, each applying only when the project's lock
lists the skill. Out of scope: any gate, sub-agent transcripts, `cardinal-report` changes (REP-6 lists every rule).

## Measure
Over the local transcripts (all projects, sessions holding the trigger / of them with the skill never loaded before it):
`git commit` 379 / 163, new or rewritten `specs/*.md` 32 / 24, `Plan: proposed` 3 / 2. All three fire: none dropped.

## Plan
1. [x] Skill routing rules: the loaded skills of the main transcript, the three triggers, the lock gate; verified by
   tests of `crates/cardinal/tests/rule_breaches.rs` through the binary; makes true BRE-15, BRE-16, BRE-17, BRE-18.
2. [x] Fixes of the `high` review: a failed `Skill` call loads nothing, the commit trigger is the `commit` subcommand
   with a message to write, a spec counts once per session, the spec list in one block; verified by the same test;
   makes true BRE-15, BRE-16, BRE-18.

## Decisions
- Mode: autonomous
- One finding per trigger (as `validation-stop`), not one per session.
- A plan is proposed only by a line `Plan: proposed` of its own (BRE-17): a `create-issue` quoting the marker in a
  sentence was the one real finding of the first run. BRE-8 keeps its wider match: follow-up below.
- A new spec is a `Write` of a path absent from the default branch at the session's `start`.
- Base commit: 69ba7f79b75ef03adb1eb03ea4ffe48f68118fa1

## Summary of deliverable 1
done: rules `skill-routing-commit-message`, `-issue-workflow`, `-write-spec` in `rule-breaches`, gated on the lock.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BRE-15 to BRE-18 traced, spec `accepted`) |
| docs | `docs/usage.md`, `docs/architecture.md` |
| code review | 1 finding |

red: BRE-15, BRE-16, BRE-17, BRE-18: `FAIL - a trigger is a finding until its skill is called or typed, in the session's own transcript only` (no `skill-routing` row)

On the local transcripts: `skill-routing-commit-message` applied 194, breached 28; `-write-spec` 19 / 14;
`-issue-workflow` 0 (no plan written through `Edit` or `Write` in a session whose lock lists the skill).
Code review: 1 finding: `check_routing` and `proposes_plan` (BRE-8) match `Plan: proposed` differently; not fixed,
BRE-8 is an accepted requirement outside this deliverable (follow-up below).

not verified: none

decisions needed: none

## Summary of deliverable 2
done: fixes of 7 of the 10 findings of the `high` review of `69ba7f7..609c1b8`.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a (no documented behaviour changed) |
| code review | see below |

red: BRE-15, BRE-16, BRE-18: `FAIL - a trigger is a finding until its skill is called or typed, in the session's own transcript only` (the new cases against the code of deliverable 1)

Code review: 3 findings: a chained `git add && git commit`, a `--no-edit` of a later git command, a `Skill` call
with no result read; all fixed (each `git` of the command read on its own, an unread result counts as loaded).

not verified: none

decisions needed: none

High review, not fixed: the `blocked` kind of a failed trigger (BRE-15 says so, as BRE-4 for `no-verify`); a spec
committed on an unmerged issue branch counts as new (the validated decision: the default branch); the two readings of
`Plan: proposed` and the `ends_with("code-review")` match of BRE-5 (accepted requirements outside this issue).

## Follow-ups
- BRE-8 (`validation-stop`) matches `Plan: proposed` anywhere in the written text, so a new issue quoting the marker
  opens a false validation stop; the line rule of BRE-17 would fix it (a change of an accepted requirement: the user's call).
