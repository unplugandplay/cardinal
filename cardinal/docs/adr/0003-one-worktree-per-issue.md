# 0003. One worktree per issue

Status: Superseded by 0008
Date: 2026-10-25

## Context
Two agents in one working tree share uncommitted files, the git index and `target/`, so a commit or a formatter
run by one captures the other's changes. `just next-issue` gave both the same issue, and `just close-issue` edits
the shared queue file. A plain branch does not isolate uncommitted work.

## Decision
Each issue gets a git worktree next to the main one (`../<repo>-<stem>`) on a branch named after the stem
(`just start-issue`). An issue is taken while that branch or worktree exists: the mark comes from git alone, so no
file has to be shared or cleaned. The queue is edited only on the main worktree, at merge time (`close-issue`
refuses elsewhere). The merge itself stays a deliberate step of the user, from the main worktree: the skill
recommends a rebase then a fast-forward, which keeps the history linear. `just finish-issue` removes the worktree
and branch once the branch is an ancestor of the main branch (a fast-forward or a merge commit, not a squash) and
the worktree is clean. Each worktree keeps its own `target/`, since sharing one would add lock contention for a
small saving (figures in `docs/architecture.md`).

## Consequences
Two agents cannot disturb each other's files, index or builds, and the queue never conflicts between branches.
Disk grows by one `target/` per worktree and each worktree compiles from cold. A branch with no commit of its own counts as merged, so `finish-issue` run before any work discards the
worktree (uncommitted changes still block it). A merged branch left
in place keeps its issue taken until `finish-issue` deletes it. A bare repository and a detached main worktree
are refused.
