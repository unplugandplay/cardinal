# 0024. A session with nothing in progress hands over its worktree lock

Status: Accepted
Date: 2026-11-01

## Context
ADR 0022 holds the worktree lock while the process of its session runs. An editor's agent client (Zed with
`claude-acp`, measured 2026-11-01) starts one `claude` process per thread and keeps it running after the user opens
another thread, for days: the old thread holds the lock and every edit and command of the new one is refused, with
nothing in the editor to end the old session. The user closes an issue and goes on in a new session; a release at
`close-issue` alone does not help, as the old session often commits something afterwards and takes the lock back.

## Decision
A lock whose process runs is held only while its session has work in progress: uncommitted changes or untracked files
in the worktree, or an issue claimed in the state `working` by its session or process. Without either, the next
session that starts or acts takes it over; the old session, if it acts again while the new one has work in progress,
is refused. An unreadable status counts as work in progress. This amends ADR 0022.

## Consequences
A new editor thread works at once after the previous one committed its work. A session with nothing in progress is no
longer protected: two sessions acting at once in a clean worktree pass in turn until one of them changes a file. Each
refused call costs one `git status` and one read of the claims.
