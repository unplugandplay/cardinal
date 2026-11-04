# 0026. A queue of issues is chained by the main loop

Status: Accepted
Date: 2026-11-02

## Context
The user wants to hand the agent the next 2 or 3 issues and be stopped once for their plans. An orchestrator that
drives each issue in a subagent was considered: a subagent cannot ask the user anything, may not start the reviewer
subagents of `/code-review`, and each handoff loses detail; the chain is sequential, so the fresh context of a
subagent buys no parallelism. `isolation: "worktree"` would break ADR 0011 and the claims of ADR 0022.

## Decision
The main loop of the session drives the queue itself, in the worktree the session was started in: it claims the
issues, writes their plans, stops once for all of them, then chains them, each in the mode validated for it, checking
before each issue whether the issues already done made its plan stale. The state of each issue stays in its `#
Tracking`, and a `Queue:` line of each plan names the queue and its base, so an automatic compaction or a resumed
session picks it up from the files. No subagent takes part (the delegated wide search was folded back since).

## Consequences
The reviews, the gates, the claims and `next-issue` work as for one issue. The context of the session grows with the
queue, which the batch size (3 at most) and the compaction bound. The stale plan check is a judgment: an impact no
file or id shows is caught only on rereading or by the `high` review. The plans of the whole queue are committed after
the stop, so the drafts of a later issue never enter the diff of an earlier one; each issue is closed when done if the
user agreed, so `delivered-quality` counts its work apart, the preparation of the later issues aside, counted with the
first.
