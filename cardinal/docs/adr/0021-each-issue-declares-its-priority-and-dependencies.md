# 0021. Each issue declares its priority and dependencies

Status: Accepted
Date: 2026-11-01

## Context
The `rank` of each issue file (ADR 0019) packs into one number the order of the milestones, already written in the
`roadmap` documents, the dependencies between issues, written in prose in `why` or the Hints and checked by nothing,
and the priority. A new issue must be slotted between two neighbours, a re-ranking rewrites every open issue, and
nothing tells which issues can start now, which the issues of driving several issues in a row and of claiming an
issue both need.

## Decision
Each open issue declares `priority: high | medium | low` and the issues it needs, `depends_on: [<stem>, ...]`, in
its front matter; `why` states the reason for the priority; `rank` goes away. `next-issue` derives the order: the
ready issues (every dependency in `issues/done/`) by the order of their milestone in the `roadmap` documents (the
`# Milestone` section of the issue names it; none, after the milestones), then priority, then a tie-breaker; then
the blocked ones, each with what blocks it. It reports a missing priority, an unknown dependency, a cycle and an
unknown milestone; `check-issue` validates the keys of a new issue. The format of the source rises to 6 (ADR 0014);
`migrations/6.toml` (ADR 0017) names the `rank:` line, so that the update of a consumer lists its issues still
ranked, which `skill:prioritize-issues` converts once. This supersedes the `rank` of ADR 0019, whose one file per
issue stays.

## Consequences
A new issue edits only its own file and needs no free slot; a re-ranking edits only the issues whose priority or
dependencies change. Two issues of the same milestone and priority are ordered by the tie-breaker, not reported.
The priority orders only within a milestone: an issue of an earlier milestone comes first whatever its priority. A
dependency on something other than an issue (a measure, a number of sessions) stays prose. A consumer updated to
this format sees its ranked issues as without a priority until the conversion.
