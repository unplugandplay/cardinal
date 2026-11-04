# 0023. Worktrees open next to the main one, with no issue

Status: Accepted
Date: 2026-11-01

## Context
ADR 0008 nested the issue worktrees under `.claude/worktrees/` to spare the approval prompt of `EnterWorktree`;
since ADR 0011 the agent never enters a worktree, so that reason is gone, while the nesting still costs a line of
`info/exclude`, a setting in each tool that walks `.` (prettier, eslint, vitest, jest), and a session launched there
may load the main worktree's `CLAUDE.md` as an ancestor. Since ADR 0022 a session claims its issue by a ref and the
worktree lock keeps one session per worktree, so a worktree no longer needs to carry an issue's name: it can serve
several issues in turn, on a branch the user picks, and keep its build (a cold build per new worktree).

## Decision
`just start-issue` and `just finish-issue` give way to `just open-worktree <name>` and `just close-worktree <name|path>`.
`open-worktree` creates the worktree at `<main worktree>-<name>`, the place of ADR 0003, or at the path given, on a
branch `<name>` or the one given, cut from the trunk, and copies the personal configuration; it needs no issue file and
writes nothing to `info/exclude`. `close-worktree` removes a worktree and its branch under the safety of `finish-issue`
(merged, clean, holding no worktree, not from inside, `--force` for unmerged work), and still removes the worktrees an
earlier `start-issue` left under `.claude/worktrees/`. A branch named after an issue still takes it (PAR-3), for the
branches of the former recipe. This supersedes ADR 0008; ADR 0009 stands (the scanners read git's files).

## Consequences
A worktree outlives its issues and keeps its `target/`; the user names it after a line of work, not an issue. No
tool needs a setting to skip the worktrees, which are out of the main one. Nothing ties a worktree to an issue any
more: the claim (ADR 0022) is the only mark of an issue in progress there. The renamed recipes are declared in
`migrations/6.toml`, so a consumer's documents that cite the old names get a warning at `update`.
