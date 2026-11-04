---
priority: low
depends_on: [order_the_issues_by_priority_and_dependencies]
why: 'low by the user''s choice: a comfort, one session per issue works today'
---
# Drive several issues in a row from one session

# Goal
Today one session drives one issue, and the user starts the next one by hand. The user wants to hand the agent the
next few issues of the queue and let it chain them: the main agent becomes an orchestrator, each issue runs in a
subagent with a fresh context, and the user is stopped as rarely as safety allows. The outcome: a batch of 2 or 3
independent issues done in one session, with one grouped plan validation up front and one final report.

# Findings
* `skill:issue-workflow` calls the stop of step 2 (plan validation) "never lifted", even in autonomous mode: chaining
  N issues as they are written today stops N times.
* A subagent can't ask the user anything: a blocking question has to come back to the orchestrator.
* The agent never creates or enters a worktree (ADR 0011, step 0 of the skill); `isolation: "worktree"` of the Agent
  tool would break that and the "taken" detection of `next-issue` (branch `<stem>`, the claim of
  ADR 0022).
* Not verified: whether `/code-review high`, run at the end of each issue, works inside a subagent (it may start
  subagents of its own).
* The article on folding subagents into the main loop (see Resources) keeps a subagent only for a measured context
  degradation, parallel work on fresh contexts, or tool confusion; the chain here is sequential, and each handoff from
  orchestrator to subagent loses detail. When a subagent stays, it is called as a tool: a fixed answer schema, and the
  main agent verifies its claims on real data.

# Approach
Sequential, in the main worktree, so the review marker, the hooks and `next-issue` work as they do now.

1. **Prepare**: the orchestrator takes the next N issues of `just next-issue` (N small, 2 or 3). A subagent per issue
   runs step 1 (understanding, spec, plan). Each plan records what it rests on (`Rests on:` in `## Decisions`: spec
   ids, ADRs, files).
2. **One grouped stop**: the step 2 stop of the N issues at once, in one questions table. The orchestrator crosses the
   `Rests on:` of each plan with what the other plans change and raises every overlap as a question (keep, drop from
   the batch, reorder). This changes the rule from "never lifted" to "grouped, never lifted".
3. **Chain**: one subagent per issue, in autonomous mode (step 3 unchanged, `low` and `high` reviews included, local
   commits). Between two issues the orchestrator checks `just check`, the commits and the ticked `# Tracking`.
4. **Stale plan check**, before issue k starts: intersect `git diff --name-only <base of k>..HEAD` and the decisions the
   earlier issues added with the `Rests on:` of k. No hit: go. A hit without impact: note it in `## Decisions` and go.
   A hit that changes the plan: the chain stops, issue k goes back to `Plan: proposed` and its own step 2 stop; the
   done issues stay committed.
5. **Stops the chain**: as in autonomous mode (blocking question, unsettled finding, red check), plus the stale plan.
6. **Final report**: a table `issue | commits | state`, then one grouped close request.

Candidate forms: a new skill (e.g. `drive-queue`) or a mode of `skill:issue-workflow`. To settle first: whether
`/code-review high` runs in a subagent; if not, the orchestrator runs it itself after each subagent returns.

Alternative to weigh at step 1, no orchestrator: one loop chains the issues itself, compacting its context between two
issues, and the stale plan check of point 4, a judgment, becomes an advisor call that may answer stop
(`issues/done/tell_the_issue_workflow_when_to_consult_the_advisor.md`). If subagents stay, their answer is the 3.6 summary
schema, and the orchestrator checks it against `just check`, `git log` and the ticked `# Tracking`.

Non-goal: parallel issues driven by the agent; parallelism stays one session per worktree opened by the user.

# Milestone
Longer autonomy

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 2 and 3, Autonomous mode) and its `details.md`.
* `docs/adr/0011-the-user-opens-the-issue-worktree.md`, `specs/parallel-issues.md`.
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop
* `issues/done/issue_workflow_autonomous_mode.md`, `issues/done/run_agents_in_parallel_on_different_issues.md`,
  `issues/done/delegate_the_exploration_of_the_issue_workflow_to_a_subagent.md`.

# Hints
* The stale plan check is a judgment, not a deterministic proof: an impact that no file or id shows (a change of
  intent) is caught only if the subagent sees it on rereading, or by the `high` review. Keep batches small and
  batch only independent issues.
* An issue that depends on another of the batch waits for it to be done, then gets its own step 2 stop.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
Let the user hand the agent the next 2 or 3 ready issues: one grouped plan stop, then the issues chained in autonomous
mode, with a stale plan check between two of them and one final report. Out of scope: parallel issues, a new tool.
Means: the main loop chains the issues itself, no orchestrator subagent (ADR 0026 draft).

## Plan
1. [x] Queue mode of `skill:issue-workflow`: a section in `details.md` (take the issues by `next-issue` then
   `issue-claim` one after the other on a clean tree, step 1 of each with its `Rests on:`, one grouped step 2 stop
   crossing them, the chain, the stale plan check, the stops, the final report and grouped close), its entry in
   `SKILL.md` (description, step 2 "grouped, never lifted", Autonomous mode, an Advisor moment for the stale plan
   check), ADR 0026, `just sync`; verified by new required passages in `crates/cardinal/tests/skills.rs`, red first. (autonomous)
2. [x] Docs and `high` review fixes of the whole issue (steps 3.2 to 3.8). (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 6565ce13ef2bf1c6e0f47d52ca58d846c2352fb5
- Answers of the plan stop: the main loop chains the issues (1 a); a section of `details.md` with a short entry in
  `SKILL.md` (2 a); 3 issues at most per batch (3 a).
- No spec: a harness change (skill text), proved by the required passages of `tests/skills.rs` like the other modes.
- Taking N issues needs no tool change: once one is claimed, `next-issue` skips it and offers the next ready one, so
  the batch holds only ready issues; the claims are all taken before any file is written (CLM-9 wants a clean tree).
- `/code-review high` inside a subagent: not verified; moot with the main loop, where the reviews run as today.
- Measure: QUA-8 counts the preparation of issues 2 and 3 with issue 1 (it runs before issue 1's first commit);
  accepted, the per-issue tokens of a queue are approximate.

## Summary of deliverable 1
done: section "Drive a queue" in `details.md`, its entry in `SKILL.md` (description, intro, never lifted, Advisor),
ADR 0026 accepted, `docs/workflow.md`, synced.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | n/a (harness) |
| docs | `docs/workflow.md`, ADR 0026 |
| code review | see below |

red: none (no spec): `tests/skills.rs` failed on the 6 missing passages of the queue before the text.
Code review: 1 findings
- The plan signature had replaced `Plan: proposed` in the Approach text instead of the `# Tracking` line: fixed, both
  lines restored.

not verified: a real queue run (the text is proved by its required passages only).
decisions needed: none.

## Summary of deliverable 2
done: the `high` review fixes of the queue mode: a `Queue:` line with the base of the queue and a resume rule (also
in step 0), the plans and drafts of the queue committed after the stop, the mode of each issue kept, the close of
each issue asked at the stop, no per-issue final summary inside a queue, the chain after a re-validated stale plan,
a dropped issue's claim freed by compare-and-swap, `docs/workflow.md` and ADR 0026 aligned.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | n/a (harness) |
| docs | `docs/workflow.md`, ADR 0026 |
| code review | `high` on the issue, then `low` on the fixes |

red: none (text fixes, the passages of deliverable 1 still hold).
Code review: 9 findings (`high`), all fixed in the text: close counted after the later issues (closed when done if
agreed at the stop), drafts of later issues in the diff of the first (queue plans committed after the stop), a
dropped issue's files (committed with them, `Plan: proposed`), no record of the queue (`Queue:` line, Resume), the
per-issue end of the Autonomous mode (overridden in a queue), the mode forced autonomous (the recorded one), the
chain after a stale plan (goes on), a raw delete of the claim (compare-and-swap on the own session), the docs saying
`next-issue` claims (it offers, `issue-claim` claims).

`low` on the fixes: 3 findings, fixed: a dropped issue loses its own `Queue:` line; the base of the queue is the
commit of the plans, found by `git log -G`, so no edit is left after it; a queue all ticked is over and its issues
resume alone.

not verified: a real queue run.
decisions needed: none.
