---
priority: medium
depends_on: [order_the_issues_by_priority_and_dependencies]
why: 'claims among the ready issues that the new order lists; before driving several issues in a row, which waits on the user''s choice'
---
# Claim an issue from the session that works it

# Goal
Today an issue runs in parallel only in a worktree made for it: the user commits the issue, runs `just start-issue`
(worktree and branch named after the issue), launches `claude` there, and later merges, closes and runs
`just finish-issue`, which removes the worktree. The issue is "taken" only because a branch or a worktree carries its
name. The user wants a simpler flow:

1. The user opens a session in a worktree (a workspace, not made for one issue).
2. They hand the agent an issue; the agent starts `issue-workflow`.
3. The issue is marked taken.
4. If another agent already works in that workspace, the agent stops there, to avoid conflicts.
5. The agent works the issue as usual.

The outcome: a session claims its issue from any worktree, atomically and visibly from every other worktree, and two
sessions never work in the same worktree at once; the workspace outlives the issue.

# Findings
* `next-issue` marks an issue taken when a branch named after its stem is checked out in a worktree or holds commits
  off the main branch (PAR-3, `next_issue.rs` `branches`). An issue worked in the main worktree has no branch: nothing
  marks it, and `next-issue` still offers it to another session (ADR 0011, Consequences).
* The "main branch" every tool compares with is the branch checked out by the main worktree (`trunk` in
  `next_issue.rs`, the same in `issue_worktree.rs` and the merge back of `issue-workflow`): an issue branch checked out
  in the main worktree would become the trunk.
* `issue-workflow` step 0 stops when the issue is taken elsewhere: in the main worktree a branch `<stem>` exists, in a
  linked one its branch is not `<stem>`. So a linked worktree serves one issue, named after it.
* Nothing detects two sessions in the same worktree: they share the files, the index, the review marker (per worktree,
  PAR-9) and `target/`.
* Each new worktree pays a cold build (measured on this repository: 239 MB of `target/`, 4 s on 24 cores; a larger
  project pays much more), and `finish-issue` removes the worktree after each issue.
* Claude Code (docs, pages "worktrees" and "hooks", read 2026-10-31):
  - A session started with `claude --worktree <name>` (worktree `.claude/worktrees/<name>`, branch `worktree-<name>`),
    or entered with `EnterWorktree`, is isolated: Claude Code refuses every command it cannot prove stays in the
    worktree, and this check cannot be turned off. A plain `claude` launched in a worktree made with `git worktree add`
    is not isolated (ADR 0011).
  - Claude Code holds a `git worktree lock` on the worktrees it creates while their agent runs, and a sweep releases the
    lock of a session whose process exited: prior art for a "busy" mark with a liveness check.
  - `SessionStart` receives `session_id`, `cwd` and `source` (`startup`, `resume`, `clear`, `compact`, `fork`); it
    cannot block, but can add context for the agent. `SessionEnd` cannot block, has a 1.5 s budget by default, and
    does not run when the process is killed. `PreToolUse` can refuse a tool call.
  - Agent teams claim tasks from their shared list "with file locking to prevent race conditions".

# Weak points of the proposed flow
1. **Order**: marking the issue taken before checking the workspace leaves a stale mark when the check then stops the
   session. Check the workspace first, then claim.
2. **Atomic claim**: two sessions handed the same issue at once both see it free, then both claim it, unless the claim
   is a compare-and-swap. Creating a git ref is one: `git branch <stem>` and `git update-ref <ref> <new> ""` (the empty
   old value requires the ref not to exist) fail when it exists, and a ref is seen from every worktree.
3. **"Another agent in the workspace" is not a moment**: a check at the start of the workflow misses a second session
   opened later in the same worktree, or one that never starts an issue. The guard belongs to the session (a lock held
   while it lives), not to the workflow.
4. **Stale locks**: a crash or a kill skips `SessionEnd`, so a lock needs a liveness check (the process id it records,
   a heartbeat each hook refreshes) and must let the same `session_id` back in on a resume.
5. **The main worktree**: claiming by a branch there turns that branch into the trunk of every tool. Either the main
   worktree stays the integration checkout (issues worked on the trunk, claimed by a ref that is not a branch), or the
   trunk is read elsewhere (the default branch of the remote, a key of `.claude/cardinal.toml`).
6. **A reused workspace**: before claiming, the workspace must be clean, on no unmerged issue branch, and cut from the
   current trunk; `finish-issue` must then free the issue without removing the worktree.
7. **The end of an issue is heavy**: today the agent of a linked worktree rebases, then prints four commands the user
   runs in the main worktree (`git merge --ff-only`, `close-issue`, its commit, `finish-issue`). The reasons:
   - `close-issue` refuses in a linked worktree (PAR-8) so that the queue was edited on the main branch only (ADR
     0011, Decision). With the ranks of ADR 0019 a branch touches only its own issue files: that reason is gone, and the
     close can be the last commit of the issue branch.
   - A linked worktree cannot move the branch the main worktree has checked out: `git update-ref` would leave that
     checkout behind, and `git push . HEAD:<trunk>` is refused (`receive.denyCurrentBranch`). Whether
     `updateInstead` updates a branch checked out in another worktree with git 2.56 is not measured.
8. **A team**: a local ref or lock is invisible from another clone. Pushing the claim (the branch, a draft pull
   request) is the only shared mark; out of scope unless the user wants it.
9. **The launch is not the user's**: the user runs Claude Code through an editor's agent client (ACP), which starts
   the agent in the folder the editor opened, with no launch option. Opening a worktree as an editor project is then
   the plain launch ADR 0011 asks for (no `--worktree`, no isolation). The same client can open several agent threads
   on one project: two sessions in one worktree is a real case, not a hypothesis (weak point 3).

# Approach
Measure first: two plain `claude` sessions in one worktree, and two sessions handed the same issue from two
worktrees, to see what breaks today and what the hooks receive (`session_id`, the process id of the hook's parent).

Candidates, to combine at step 1 of `issue-workflow`:
1. **Claim by the issue branch**: in a linked workspace, step 0 runs a tool (`cardinal run issue-claim <issue>`) that
   checks the workspace is clean and cut from the trunk, then creates and checks out `<stem>` from the trunk; the
   branch is the mark PAR-3 already reads. In the main worktree the issue is worked on the trunk and claimed by
   candidate 2.
2. **Claim by a dedicated ref**: `refs/cardinal/claims/<stem>`, created by compare-and-swap, pointing at a blob that
   names the worktree and the session; `next-issue` reads it next to the branches, `close-issue` or `finish-issue`
   deletes it. Never pushed by the default refspecs.
3. **Workspace lock**: a file in the per-worktree git directory (`git rev-parse --git-dir`) holding `session_id`,
   process id and heartbeat; a `SessionStart` hook takes it or tells the agent the workspace is busy, a `PreToolUse`
   hook refuses edits and commands to a session that does not hold it while the holder is alive, `SessionEnd` releases
   it.
4. **Keep the workspace**: `start-issue` becomes the creation of a workspace (any name), `finish-issue` frees an issue
   (deletes its branch and claim) and leaves the worktree.
5. **Trunk**: read the trunk from the remote's default branch or a key, so the main worktree can host an issue branch
   too.
6. **The agent ends the issue**: on the user's agreement to close, the agent of the workspace runs `close-issue` on
   the issue branch, rebases on the trunk, then moves the trunk to its branch by compare-and-swap (`git update-ref
   refs/heads/<trunk> <branch tip> <old trunk>`), frees the claim and leaves the workspace clean on the new trunk. It
   needs the trunk checked out by no worktree (the main worktree detached, or kept for integration and moved by the
   user), or `updateInstead` if a measure shows it works across worktrees. With a remote and pull requests instead,
   the agent pushes the branch and opens the pull request, and the merge is the host's.
7. **Merge by the user's way of working** (the user, 2026-10-31): in the main worktree, issues are worked directly on
   the trunk, with no branch and no merge, as today; a workspace on another branch ends with a pull request to the
   trunk, which the agent opens and the host merges. Candidate 6's local move of the trunk is then not needed.
8. **The user picks the workspace's branch** when opening the worktree (the user, 2026-10-31): several issues may share
   it, so the branch name cannot be the claim and candidate 2 (a claim ref) is needed. The user suggests the session id
   as the ref. A compare-and-swap only collides on the same ref name, so the name must stay the issue
   (`refs/cardinal/claims/<stem>`) and the session id goes in what it points at, with the worktree: a resumed session
   is then recognized as the owner. The agent's commands get no session id; the hooks do, so the `SessionStart` hook of
   candidate 3 can write it in the workspace lock, where the claim tool reads it.
9. **The claim is freed at `close-issue`** (the user, 2026-10-31), the one end every issue has, with or without a pull
   request. Trap: on a workspace branch the close is a commit of that branch, so until the pull request is merged the
   trunk still holds the issue in `issues/`, unclaimed, and `next-issue` would offer it again. Candidate: on a branch
   other than the trunk, `close-issue` turns the claim into the state `merge pending` (the user's name, 2026-10-31:
   what is true of the issue, the branch kept as a detail) instead of deleting it; `next-issue` lists it as
   `merge pending (<branch>)` and never offers it, and drops the claim once the trunk has the issue in `issues/done/`.
   On the trunk the close is the merge: the claim is deleted at once.

A new ADR records the choice, amending ADR 0003, 0008 and 0011. Likely over the size limit: split at step 2 (claim
and `next-issue`, workspace lock, reusable workspace and docs).

# Specs
* `specs/parallel-issues.md`: PAR-1, PAR-2 (start), PAR-3, PAR-4 (taken), PAR-6, PAR-10 (finish), PAR-8 (close only
  in the main worktree) amended; new requirements for the claim, the workspace lock and the end of an issue.
* `specs/next-issue.md`: NXT-1 if a claim ref joins the branches.
* A hook spec for the workspace lock if candidate 3 is kept.

# Milestone
Longer autonomy

# Resources
* `docs/adr/0003-one-worktree-per-issue.md`, `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`,
  `docs/adr/0011-the-user-opens-the-issue-worktree.md`.
* `crates/cardinal/src/tools/next_issue.rs` (`branches`), `issue_worktree.rs`, `close_issue.rs`; `issue-workflow`
  step 0 and `details.md` ("Worktree of an issue", "Merge back").
* `docs/workflow.md` (Parallel issues), `docs/usage.md` (Parallel issues in worktrees), `docs/architecture.md`
  (Parallel issues).
* https://code.claude.com/docs/en/worktrees.md, https://code.claude.com/docs/en/hooks,
  https://code.claude.com/docs/en/agent-teams.md
* `issues/done/make_the_issue_worktree_optional_and_opened_by_the_user.md`: why the agent no longer creates worktrees.

# Hints
* Weak point 4 also covers a killed editor: an agent client closed with its editor may skip `SessionEnd`.
* The agent still never creates, enters or leaves a worktree (ADR 0011): claiming checks out a branch inside the
  worktree the user opened, which is not entering another one.
* `issues/done/drop_the_format_4_issue_queue.md` also edits `next_issue.rs`: land it first.

# Tracking

Plan: validated

## Understanding
A session claims its issue by a git ref created by compare-and-swap (`refs/cardinal/claims/<stem>`, holding session,
process, worktree, branch, state), freed at `close-issue`; a per-worktree lock held while the session's process runs
refuses edits and commands to a second session. Out of scope: a claim shared across clones; the reusable workspace
recipes (question 2). Means beyond the issue: the agent's commands already receive `CLAUDE_CODE_SESSION_ID` and
`CLAUDE_PID` (measured), so the claim tool reads the session itself and needs no hook to pass it.

## Plan
1. [x] Issue claims: `cardinal run issue-claim` and the claims read by `next-issue` (taken, `session ended`,
   `Merge pending:`, dropped once merged); verified by tests through the binary on temporary repositories; makes true
   CLM-1 to CLM-6, CLM-8, CLM-9 (~700 lines).
2. [x] Close frees the claim: `close-issue` runs in any worktree, deletes the claim on the trunk, sets it
   `merge pending` elsewhere; verified by tests through the binary; makes true CLM-7, PAR-18 (PAR-8 withdrawn)
   (~250 lines).
3. [x] Workspace lock: measure what a hook receives (`CLAUDE_PID`, the `session_id` after `/clear`), then
   `cardinal run workspace-lock check|start|release` and its hooks in `entries/.claude/settings.json`; verified by
   tests feeding hook inputs; makes true WSL-1 to WSL-9 (~650 lines).
4. [x] Workflow: `issue-workflow` step 0 claims instead of reading branches, the end of an issue on a workspace
   branch opens a pull request, `details.md`, the Rules, `docs/workflow.md`, `docs/usage.md`,
   `docs/architecture.md`, ADR 0022 accepted; verified by `just check` and `just check-cardinal` (~300 lines).

## Decisions
- Mode: autonomous
- Base commit: 3d848c8a74d8f117ee68301357b6d15dc343e6b8
- Q1 (a): a new claim is refused on uncommitted tracked changes outside `issues/`; taking a claim back checks nothing (CLM-9).
- Q2 (a): `start-issue` and `finish-issue` stay as they are; the reusable workspace is a follow-up issue.
- Q3 (a): on the user's agreement to close, an issue of a workspace branch is closed on that branch, pushed, and its
  pull request opened with `gh` (or the branch's open one updated).
- High review fixes: one more deliverable (5), below.
- Q4 (a): a `PreToolUse` hook refuses `Bash`, `Edit`, `Write` and `NotebookEdit` to a second session.

## Deliverable 1: issue claims

- done: `cardinal run issue-claim` takes `refs/cardinal/claims/<stem>` by compare-and-swap; `next-issue` lists
  working claims as taken (`session ended` when the process is gone) and `Merge pending:` ones, dropped once merged.
- checks: tests pass (`tests/issue_claim.rs`, 8), clippy clean, specs checked, docs (`docs/architecture.md`).
- red: CLM-1, CLM-2, CLM-3, CLM-4, CLM-5, CLM-6, CLM-8, CLM-9: `cardinal: unknown tool `issue-claim``.
- not verified: a claim on macOS (`kill -0` assumed).

## Deliverable 2: close frees the claim

- done: `close-issue` runs in any worktree (refusing an issue of another worktree of the same repository), deletes
  the claim on the trunk and sets it `merge pending` on another branch; `specs/issue-claim.md` and
  `specs/parallel-issues.md` accepted.
- checks: tests pass (`tests/issue_claim.rs`, `tests/close_issue.rs`), clippy clean, specs checked, docs
  (`docs/architecture.md`, `docs/workflow.md`, `details.md`).
- red: CLM-7, PAR-18: `FAIL - a linked worktree closes its own issue`.

## Deliverable 3: workspace lock

- done: `cardinal run workspace-lock check|start|release` and its three hooks in `entries/.claude/settings.json`;
  the lock (`cardinal-session` in the git directory of the worktree) changes under a mutex directory;
  `specs/workspace-lock.md` accepted.
- checks: tests pass (`tests/workspace_lock.rs`, 9), clippy clean, specs checked, docs (`docs/architecture.md`,
  `docs/gates.md`, `README.md`).
- red: WSL-1, WSL-2, WSL-3, WSL-4, WSL-5, WSL-6, WSL-7, WSL-8, WSL-9: `cardinal: unknown tool `workspace-lock``, then
  the WSL-8 assertion on the entries.
- measure: once synced here, the live `PreToolUse` hook wrote the lock with this session's `session_id` and the
  `CLAUDE_PID` the agent's commands see: a hook receives both.
- not verified: whether `/clear` gives a new `session_id` (the same process is let through either way); macOS.

## Deliverable 4: workflow

- done: `issue-workflow` step 0 claims the issue and stops on a refused claim or lock; the merge back of a workspace
  branch closes on the branch, pushes and opens or updates the pull request; `details.md` (Workspace, Merge back),
  `docs/workflow.md`, `docs/usage.md`, `docs/architecture.md`; ADR 0022 accepted.
- checks: `just check` passes, `just check-cardinal` passes, docs reread.
- red: none (instructions and docs, no test).
- not verified: the pull request path end to end (no workspace branch closed yet).

## Deliverable 5: docs of the whole issue and fixes of the high review

- done: `CLAUDE.md` and `docs/architecture.md` name the claim and the lock; a `merge pending` claim is dropped once
  merged even where its issue is no longer open; `issue-claim` accepts only a file directly under `issues/` and names
  a ref git refuses; `close-issue` leaves the claim of another worktree; the merge back names no remote nor host
  (`origin`, `gh` only as the GitHub example); CLM-3, CLM-6, CLM-7 and the intent of WSL reworded.
- checks: tests pass, clippy clean, specs checked, docs reread.
- red: CLM-3, CLM-6, CLM-7: `assertion failed: NOTES.md is no issue file`, `dropped although a is no longer open`,
  the claim of `ws` rewritten by a close in the main worktree.
- Code review: 10 findings (high, `3d848c8..HEAD`)
  - merge pending claim leaked after the merge: fixed.
  - same process takes the lock or the claim: kept, measured: the editor's agent client runs one `claude` process per
    thread (two under its adapter), so one process is one session lineage (`/clear`).
  - any `.md` claimable, `:(exclude).` disables the dirty check: fixed.
  - stale mutex break races: not fixed, needs a hook killed inside its few milliseconds under the mutex plus two hooks
    at once; a gate for cooperative agents (ADR 0005).
  - close overwrites another worktree's claim: fixed.
  - `origin` and `gh` hard-coded in the shared skill: fixed.
  - `SessionStart` takes the lock of an idle session: kept, the guard belongs to the session (weak point 3, Q4);
    intent reworded.
  - `update-ref` failure reported as a race: fixed message.
  - one `cat-file` per claim: not fixed, the claims are dropped once merged, so they stay few.
  - duplicated git helpers, `trunk` on a detached main: not fixed, a detached main worktree has no trunk for any tool.
- Code review: 2 findings (low, on this deliverable)
  - the claim of a removed worktree was never freed: fixed, `close-issue` frees it.
  - a bare file name run from `issues/` was refused: fixed, the directory is canonicalized.
