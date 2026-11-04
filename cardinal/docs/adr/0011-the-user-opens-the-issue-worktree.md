# 0011. The user opens the issue worktree

Status: Accepted
Date: 2026-10-27

## Context
ADR 0003 gave each issue a worktree, and `issue-workflow` created it and moved the session there with Claude Code's
`EnterWorktree`. Claude Code then isolates the session: it refuses every Bash command it cannot prove stays in the
worktree (heredocs naming `git`, loops, subshells, chained commands), which refused dozens of ordinary commands in
one issue and cost a call and its context each time. The isolation applies to a session started with `--worktree`,
entered with `EnterWorktree` or resumed in one, and cannot be turned off (Claude Code docs, page "worktrees"). Most
issues are worked one at a time and need no worktree.

## Decision
The worktree is the user's choice, for parallel work only. The agent works where the session was started, the main
worktree or a linked one, and never creates, enters or leaves a worktree. To run an issue in parallel, the user runs
`just start-issue` and launches a plain `claude` in the worktree it prints. In a linked worktree the agent ends with a
rebase and prints the merge, `close-issue` and `finish-issue` commands for the user. ADR 0003 and 0008 stand for the
rest: the place under `.claude/worktrees/`, the mark "taken" from git, the queue edited only on the main branch.

## Consequences
An issue in the main worktree pays no isolation refusal and needs no merge. Two parallel issues need the user to open
the second session and to merge each worktree. A session started with `claude --worktree` or `EnterWorktree` still
gets the isolation; the docs name the plain `claude` launch. An issue worked in the main worktree has no branch, so
`next-issue` still offers it: the user does not start it in parallel, and `issue-workflow` stops when it is handed an
issue whose branch exists. With no `EnterWorktree`, the placement under `.claude/worktrees/` of ADR 0008 loses its
reason (no approval prompt), and a session launched in a nested worktree may also load the main worktree's
`CLAUDE.md` as an ancestor (not measured).
