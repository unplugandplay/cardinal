# 0022. A session claims its issue and locks its workspace

Status: Accepted, partly superseded by 0024 (a live session with no work in progress hands its worktree lock over)
Date: 2026-11-01

## Context
An issue is taken only when a branch or a worktree carries its name (ADR 0003, 0011): an issue worked in the main
worktree marks nothing, and a worktree serves one issue and is removed after it, each new one paying a cold build.
The user opens a worktree as an editor project, picks its branch, and may work several issues on it; the editor's
agent client can open several threads on one folder, so two sessions in one worktree is a real case. The commands of
an agent receive `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PID` (Claude Code 2.1.312, measured 2026-11-01), and the hooks
receive `session_id`. `SessionEnd` does not run when the process is killed.

## Decision
A session claims its issue at the start of `issue-workflow` with `cardinal run issue-claim`: the ref
`refs/cardinal/claims/<stem>`, created by compare-and-swap, points at a blob naming the session, its process, the
worktree, the branch and a state. `next-issue` reads the claims next to the branches; the claim of a session whose
process ended stays visible and can be taken back from the same worktree. `close-issue` runs in any worktree: on the
trunk it deletes the claim, on another branch it sets it `merge pending`, and `next-issue` drops it once the trunk
holds the closed issue. A worktree of its own branch ends its issues with a pull request the agent opens and the host
merges; the main worktree works on the trunk, with no branch and no merge.
A lock per worktree (`cardinal-session` in its git directory), taken by the first session that acts there and held
while its process runs, makes a `PreToolUse` hook refuse edits and commands to any other session; `SessionStart`
warns the second session, `SessionEnd` releases the lock, and a dead process frees it. This amends ADR 0003, 0008 and
0011: the queue is no longer edited on the main branch only, and an issue is no longer tied to a worktree.

## Consequences
An issue worked in the main worktree is no longer offered to another session. A worktree outlives its issues and
keeps its build. Two sessions of one worktree stop before they collide, at the cost of a hook on each edit and
command. A claim and a lock are local to the clone: a team still sees only pushed branches and pull requests. A
reused process id keeps a dead claim or lock alive until that process ends. The branches named after an issue still
take it (PAR-3), so `start-issue` keeps working.
