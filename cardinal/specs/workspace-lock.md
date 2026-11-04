---
id: WSL
status: accepted
adrs: [0005, 0022, 0024]
---

# Worktree lock

## Intent

Two Claude Code sessions in the same worktree share its files, its index, its review marker and its build output, and
an editor's agent client opens several threads on one folder. A lock per worktree, held by the session that first starts
or acts there while its process runs, stops the other sessions before they edit or run anything (ADR 0022). An editor
keeps the process of an old thread running, so a holder that left nothing in progress (no uncommitted change, no
issue claimed) hands the lock over to the next session instead of blocking it (ADR 0024). Like the other
gates it catches the mistakes of cooperative agents, not deliberate bypasses (ADR 0005).

## Requirements

- **WSL-1** (deterministic) `cardinal run workspace-lock check`, run as a `PreToolUse` hook, must take the lock of the worktree of the `cwd` of its input when there is none, or when the process it names no longer runs, by writing the `session_id` of its input and the process of the session (`CLAUDE_PID`) to the file `git rev-parse --git-path cardinal-session` gives there, and let the call through.
- **WSL-2** (deterministic) `check` must let through, rewriting the lock with its own session and process, a call of the session the lock names, or of another session of the same process (a cleared session).
- **WSL-3** (deterministic) `check` must refuse, with exit code 2 and a message on stderr that says another live session works in this worktree, names its session and tells the agent to stop and let the user open another worktree or end that session, a call of any other session while the process of the lock runs and the holder has work in progress as WSL-10 says.
- **WSL-4** (deterministic) Two sessions of two processes taking a free or stale lock at once, in a worktree with uncommitted changes, must leave exactly one of them holding it, the other refused as WSL-3 says.
- **WSL-5** (deterministic) `cardinal run workspace-lock start`, run as a `SessionStart` hook, must take the lock as WSL-1, WSL-2 and WSL-10 say or, when another live session with work in progress holds it, take nothing, exit 0 and print as additional context for the agent that the worktree is busy and that its edits and commands will be refused.
- **WSL-6** (deterministic) `cardinal run workspace-lock release`, run as a `SessionEnd` hook, must delete the lock when it names the session of its input, and leave a lock of another session untouched.
- **WSL-7** (deterministic) `workspace-lock` must let the call through and change nothing when its input cannot be read, when its `cwd` is not in a git repository, or when `CLAUDE_PID` is unset, so that a broken hook never blocks a session.
- **WSL-8** (deterministic) The managed entries of `.claude/settings.json` (`entries/.claude/settings.json` of the source, SYN-29) must hold `cardinal run workspace-lock check` as a `PreToolUse` hook on `Bash|Edit|Write|NotebookEdit`, `cardinal run workspace-lock start` as a `SessionStart` hook and `cardinal run workspace-lock release` as a `SessionEnd` hook.
- **WSL-9** (deterministic) The lock of a linked worktree must be its own (the git directory of that worktree), so a session of the main worktree and one of a linked worktree never block each other.

- **WSL-10** (deterministic) `check` and `start` must take, as WSL-1 says, a lock whose process runs when its holder has no work in progress: `git status --porcelain` of the worktree lists nothing (tracked changes and untracked files not ignored, `issues/` included) and no claim in the state `working` (CLM-1) names the session or the process of the lock; when the status cannot be read, the holder counts as having work in progress.

## Scenarios

- (WSL-1) Given a lock naming session `a` and a process that no longer runs, when session `b` calls `Edit`, then the call passes and the lock names `b`.
- (WSL-3) Given a lock naming session `a` whose process runs, when session `b` of another process calls `Bash`, then the call is refused and the lock still names `a`.
- (WSL-10) Given a lock naming session `a` whose process runs, a clean worktree and no claim of `a`, when session `b`
  calls `Bash`, then the call passes and the lock names `b`; when `a` then calls `Edit` after `b` changed a file,
  then `a` is refused.
- (WSL-10) Given a lock naming session `a` whose process runs and a claim in the state `working` naming `a`, when
  session `b` calls `Bash` in the clean worktree, then the call is refused.
- (WSL-6) Given a lock naming session `a`, when session `b` ends, then the lock still names `a`.

## Non-goals

- Refusing the read-only tools (`Read`, `Grep`, `Glob`) to a second session: it can still read, never change.
- A process id reused after the holder died, and platforms where `kill -0` cannot test a process.
- A lock seen from another clone or machine.
- Two sessions acting at once in a clean worktree: each call may hand the lock over, until the first change makes one
  of them the holder with work in progress.
