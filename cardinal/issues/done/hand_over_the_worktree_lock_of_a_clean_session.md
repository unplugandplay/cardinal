---
priority: high
why: 'high: an editor that keeps the process of every old thread alive blocks the next session of a worktree until the editor quits, so the one session per issue flow breaks'
---
# Hand over the worktree lock of a session that left the worktree clean

# Goal
The worktree lock (ADR 0022, `specs/workspace-lock.md`) is freed only by `SessionEnd` or by the death of the process of
its holder. An editor's agent client (Zed through `claude-acp`) starts one `claude` process per thread and keeps it
running when the user opens another thread, so the old thread holds the lock as long as the editor runs and the new
session is refused every edit and command, with no way in the editor to "end that session". The user usually closes an
issue (`close-issue`) and goes straight on with a new session and the next issue. The expected outcome: a new session
takes the lock over when the holder has nothing in progress, while a holder with work in progress stays protected.

# Findings
* 2026-11-01, a consumer project, Zed with `claude-acp`: the lock named session `24ac7975` (process started
  2026-10-31 22:57), whose last action was at 13:31; a new thread (session `4a654ccd`, 16:43) was refused 4 times
  (`Refused: another live session (24ac7975…)`), after the `SessionStart` warning.
* That holder had closed an issue with `close-issue` at 01:05, then committed a `cardinal` update (13:28) and a
  reprioritization of the issues (13:31) outside any issue. A release at `close-issue` alone would not have helped: the
  next `check` of the holder took the free lock back at 13:28.
* At the time of the refusal the worktree had no uncommitted change and no claim ref (`refs/cardinal/claims/`) named
  the holder.
* The same editor kept `claude` processes of threads opened on 2026-10-29 alive on 2026-11-01, in other folders: every
  worktree opened in a second thread hits it.

# Approach
1. **Takeover of a clean holder** (proposed): `check` and `start` take a lock held by another live session when the
   worktree has no uncommitted change to tracked files and no claim in the state `working` names the holder's session.
   The lock moves to the new session: the old one, if it acts again, is refused as WSL-3 says (a transfer, never a
   share). A holder with uncommitted changes or a working claim keeps the lock as today.
2. To settle at step 1: whether untracked files count as work in progress; whether changes under `issues/` alone
   count (CLM-9 ignores them for a new claim); whether `start` already takes over or only warns that the next call will.
3. Weighed and set aside: an expiry after N minutes of inactivity (an arbitrary delay; a long build or a slow review
   looks idle), a release at `close-issue` only (see Findings).

# Specs
* `specs/workspace-lock.md`: amends WSL-1, WSL-3 and WSL-5 (the takeover condition), adds a requirement and a scenario
  for the takeover and for the refusal of the old holder after it.
* An ADR amending ADR 0022 (the lock is held while its session has work in progress, not while its process runs).

# Milestone
Longer autonomy

# Resources
* `docs/adr/0022-a-session-claims-its-issue-and-locks-its-workspace.md`, `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md` (gates for cooperative agents).
* `specs/workspace-lock.md`, `specs/issue-claim.md` (CLM-1 claim lines, CLM-9 the clean check of a new claim).
* `crates/cardinal/src/tools/workspace_lock.rs` (`take`), `crates/cardinal/src/tools/issue_claim.rs`,
  `crates/cardinal/tests/workspace_lock.rs`.

# Hints
* The decision runs under the mutex of `take`, and reads git (status, claim refs) there: keep it within the 2 s wait
  of `Mutex::take`, or a slow `git status` lets the call through unlocked.
* A hook must never block a session when git fails (WSL-7): an unreadable status means "not clean", never a crash.

# Tracking

Plan: validated

## Understanding
A live holder with a clean worktree (`git status --porcelain` empty, untracked and `issues/` included) and no
`working` claim of its session or process hands the lock over at `check` and `start`. No delay, no `close-issue` hook.

## Plan
1. [x] Hand over the lock of a holder with nothing in progress: `take` checks the work in progress of a live holder
   (outside the mutex, re-checked holder unchanged under it), the WSL-3/4/5 tests get a dirty worktree, new tests of
   the handover and of a claimed holder; verified by `crates/cardinal/tests/workspace_lock.rs`; makes true WSL-3,
   WSL-4, WSL-5, WSL-10.

## Decisions
- Mode: autonomous
- Base commit: 1529a70cf5b401608bfb4621ff7ebc430f308830
- Untracked files not ignored count as work in progress (1 a); changes under `issues/` count (2 a); `start` takes
  the lock over as `check` does (3 a).

## Summary of deliverable 1
done: a live holder with a clean worktree and no `working` claim of its session or process hands the lock over at
`check` and `start`; the git reads run before the mutex, and the holder they judged must still hold the lock under it.

| check | result |
|---|---|
| tests | `just check` green (12 tests of `workspace_lock`) |
| lint | clean |
| specs | `specs/workspace-lock.md` accepted, WSL-10 traced |
| docs | ADR 0024, `issue-workflow` details |
| code review | see below |

red: WSL-10: `a clean worktree and no claim: Refused: another live session (a) works in this worktree`
Code review: no findings

## Review of the whole issue (`high`)
2. [x] Fixes of the `high` review: `git status` with `--no-optional-locks` and `--untracked-files=normal`; a holder
   that changed between the two reads is judged under the mutex instead of refused; the guard no longer repeats
   `keeps`; `docs/architecture.md` hook table and ADR 0022 status updated.

Code review high: 8 findings
* fixed: optional index lock, `status.showUntrackedFiles`, changed holder refused, duplicated guard, architecture
  table, ADR 0022 status.
* not fixed: an edit in flight when the lock is handed over (a check passes before its tool runs; the same window as
  the non-goal of two sessions acting at once in a clean worktree, a cooperative gate, ADR 0005); one `git cat-file`
  per claim (read only when the status is clean, and a clone holds few claims).
Code review (fixes): no findings
