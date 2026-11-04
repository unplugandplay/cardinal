# 0008. Issue worktrees nest under .claude/worktrees

Status: Superseded by 0023
Date: 2026-10-27

## Context
ADR 0003 put the worktree of an issue next to the main one (`../<repo>-<stem>`). Claude Code asks for an approval
each time a session enters a path outside the repository's `.claude/worktrees/`, and no permission rule or "don't
ask again" suppresses it (Claude Code docs, page "worktrees"; only `bypassPermissions` does, which removes every
prompt). Every issue started or resumed from the main worktree therefore cost one manual approval.

## Decision
`just start-issue` creates the worktree at `<main worktree>/.claude/worktrees/<stem>`, the place Claude Code itself
uses for its worktrees. It adds `/.claude/worktrees/` to the `info/exclude` of the git common directory, once,
rather than to a `.gitignore`: it needs nothing from `sync`, works in any project, and is shared by every worktree.
The scripts that walk the repository tree skip `.claude/worktrees/` of the root they scan, so a scan of the main
worktree never reads or rewrites a nested worktree. `just finish-issue` still removes a worktree left at the old
sibling path. Everything else of ADR 0003 stands: one branch per issue, the mark "taken" from git alone, the queue
edited only at merge time, one `target/` per worktree.

## Consequences
Starting or resuming an issue needs no interaction. Being ignored, a worktree nested in an issue worktree (an
agent's) escapes `git status` there, so `finish-issue` checks `git worktree list` before it removes one. A new script that walks the tree must skip
`.claude/worktrees/`, or it reads every issue worktree too. The ignore rule lives outside the versioned files: a
fresh clone gets it at its first `start-issue`. Cargo stops at the nested worktree's own workspace root, and the
main workspace lists its members explicitly, so neither picks up the other's crates. ADR 0009 replaces the skip of
`.claude/worktrees/` in the scripts by a listing of the files through git.
