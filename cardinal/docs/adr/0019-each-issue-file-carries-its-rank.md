# 0019. Each issue file carries its rank

Status: Accepted, partly superseded by 0020 (no `--convert`, `issues/queue.md` is no longer read) and 0021 (the `rank` gives way to `priority` and `depends_on`)
Date: 2026-10-30

## Context
`issues/queue.md` held the order of every open issue, one line each: `create-issue` inserted a line, `close-issue`
removed one, `prioritize-issues` rewrote the list. Git conflicts on changes of neighbouring lines, so two branches
(two people, or two issue worktrees) that create or close issues near each other conflict on that file. No such
conflict was measured in this repository nor in a consumer (no merge commit, none in the session transcripts), but
the shape of the file makes it certain once a team works on the same repository. A `merge=union` driver keeps the
file but resurrects the lines removed on one side when the other side edited next to them, and the forges' web
merges ignore it.

## Decision
Each open issue carries its rank in its own front matter: `rank: <n>` (gaps of 100, lowest first) and `why: '<why
here>'`. `next-issue` sorts the files, issues of the same rank by stem, reports a missing rank and the ties.
`create-issue` picks a rank between its neighbours, `prioritize-issues` rewrites the ranks, `close-issue` only moves
the file. The format of the source rises to 5 (ADR 0014); `next-issue --convert` turns a format 4 queue into ranks,
and `next-issue` keeps reading `issues/queue.md` while it exists, for the sources of the format 4 (ADR 0016).

## Consequences
A branch touches only the issue files it creates, ranks or closes; two branches that pick the same rank give a tie,
reported, not a conflict. The order is no longer readable in one file: `just next-issue` prints it. A re-ranking
by `prioritize-issues` edits every open issue file, the first lines only, which git merges with a branch that edits
the end of one of them.
