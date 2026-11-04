---
id: CLM
status: accepted
adrs: [0011, 0022]
---

# Issue claims

## Intent

A session marks the issue it works as claimed, from any worktree and on any branch, so that `next-issue` never offers
it to another session and two sessions handed the same issue at once never both work it. The claim is a git ref, seen
from every worktree of the clone, created by compare-and-swap, and it lives until the issue is closed on the trunk
(ADR 0022).

## Requirements

- **CLM-1** (deterministic) `cardinal run issue-claim issues/<stem>.md` must create the ref `refs/cardinal/claims/<stem>` only if it does not exist (a compare-and-swap: `git update-ref <ref> <new> ""`), pointing at a blob whose lines `session <id>`, `pid <pid>`, `worktree <path>`, `branch <name>` and `state working` give the session (`CLAUDE_CODE_SESSION_ID`), the process of that session (`CLAUDE_PID`), the top level of the worktree it runs in and the branch checked out there (`HEAD` when detached), then print the claim and exit 0.
- **CLM-2** (deterministic) When the claim exists, `issue-claim` must rewrite it by compare-and-swap with its current value as the old one, and succeed, only when the claim names the same session, or the same worktree and either the same process or a process that no longer runs (a resumed or cleared session, a crashed one); for any other claim it fails with exit 1, changes nothing, and names the worktree, the branch, the state of the claim and whether its session still runs.
- **CLM-3** (deterministic) `issue-claim` must fail with exit 1, creating nothing, when `issues/<stem>.md` is not an open issue file (a `.md` file directly under an `issues/` directory) of the worktree it runs in, when it runs outside a git repository, or when `CLAUDE_CODE_SESSION_ID` or `CLAUDE_PID` is unset (a claim is taken by a Claude Code session).
- **CLM-4** (deterministic) Two `issue-claim` of the same issue run at once by two sessions of two worktrees must leave exactly one of them succeeding and the claim naming it.
- **CLM-5** (deterministic) `next-issue` must count as taken an open issue whose claim is in the state `working`, never offer it, and list it on its taken line with the worktree of the claim, followed by `session ended` when the process of the claim no longer runs.
- **CLM-6** (deterministic) `next-issue` must list an open issue whose claim is in the state `merge pending` on a line `Merge pending: <stem> (<branch>)`, never offer it, and delete that claim (compare-and-swap with the value it read) once the trunk (the branch of the main worktree) holds `issues/done/<stem>.md`, whether or not `<stem>` is still open where it runs.
- **CLM-7** (deterministic) `close-issue issues/<stem>.md` must free the claim of `<stem>` made in the worktree it runs in, and leave, saying so, the claim of another worktree that still exists: delete it when it runs with the trunk checked out, or rewrite it by compare-and-swap to the state `merge pending` with the branch it runs on, keeping its other lines; with no claim it still closes the issue.
- **CLM-8** (deterministic) A claim must never be pushed by the default refspecs: its ref stays under `refs/cardinal/`, outside `refs/heads/` and `refs/tags/`.
- **CLM-9** (deterministic) `issue-claim` must refuse with exit 1, creating nothing, a new claim (no claim of the issue exists) when the worktree has uncommitted changes to tracked files outside `issues/`; taking back a claim as CLM-2 says checks nothing, so a resumed session keeps its work in progress.

## Scenarios

- (CLM-2) Given a claim of `foo` by session `a` whose process no longer runs, in worktree `w`, when session `b` runs `issue-claim issues/foo.md` in `w`, then the claim names `b`; when `b` runs it in another worktree, then it fails and the claim still names `a`.
- (CLM-2) Given a claim of `foo` by session `a` of process `p` in worktree `w`, when a session `c` of the same process `p` (after `/clear`) runs `issue-claim` in `w`, then the claim names `c`.
- (CLM-5) Given `a` and `b` in that order, `a` claimed in worktree `w` by a process that no longer runs, when `next-issue` runs, then it prints `b` as next and lists `a` as taken in `w`, `session ended`.
- (CLM-6) Given a claim of `foo` in the state `merge pending` on branch `ws`, when the trunk does not hold `issues/done/foo.md`, then `next-issue` lists `Merge pending: foo (ws)` and keeps the claim; once a merge brings `issues/done/foo.md` to the trunk, it deletes the claim.
- (CLM-7) Given a linked worktree on branch `ws` with a claim of `foo`, when `close-issue issues/foo.md` runs there, then `issues/foo.md` moves to `issues/done/` and the claim is in the state `merge pending` on `ws`.

## Non-goals

- A claim seen from another clone: the ref is local; a pushed branch or pull request is the shared mark of a team.
- A process id reused by another process after the session died: the claim then counts as live until that process
  ends.
- Refusing to claim a blocked issue: `next-issue` never offers one, and `issue-workflow` warns about it.
- Platforms where `kill -0` cannot test a process.
