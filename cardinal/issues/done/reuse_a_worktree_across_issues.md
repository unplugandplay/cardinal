---
priority: low
depends_on: [claim_an_issue_from_the_session_that_works_it]
why: 'a comfort: a worktree already serves several issues through git worktree add; the recipes only lag behind'
---
# Reuse a worktree across issues

# Goal
The user's recipes `just start-issue` and `just finish-issue` tie a worktree to one issue: the first creates
`.claude/worktrees/<stem>` on a branch `<stem>`, the second removes it once that issue is merged. Since
`issues/done/claim_an_issue_from_the_session_that_works_it.md`, a session claims its issue by a ref and the worktree
lock keeps one session per worktree, so a worktree can serve several issues in turn, on a branch the user picks, and
keep its build. The recipes should open and remove a worktree with no issue attached, at a plain git path rather
than nested under `.claude/worktrees/`. The worktree stays the user's:
the agent still never creates, enters or leaves one (ADR 0011); only these two recipes change.

# Findings
* `start-issue` refuses an issue not committed on the main branch, creates `.claude/worktrees/<stem>` on a branch
  `<stem>`, adds `/.claude/worktrees/` to `info/exclude` and copies `.claude/cardinal.local.toml` (PAR-1, PAR-2,
  PAR-13, PAR-17).
* `finish-issue <stem>` removes the worktree and deletes the branch only once merged, `--force` otherwise (PAR-6,
  PAR-7, PAR-10, PAR-11, PAR-15).
* A branch named after an issue still takes it in `next-issue` (PAR-3); a branch named otherwise takes
  nothing, the claim does (CLM-5).
* Each new worktree pays a cold build (239 MB of `target/`, 4 s on 24 cores on this repository; much more on a larger
  project).
* ADR 0008 nested the worktrees under `.claude/worktrees/` only to spare the approval prompt of `EnterWorktree`;
  since ADR 0011 the agent never enters a worktree, so that reason is gone (ADR 0011, Consequences). The nesting
  still costs the `info/exclude` line (PAR-13), and a session launched in a nested worktree may load the main
  worktree's `CLAUDE.md` as an ancestor.
* `git worktree add` has no default path: it takes the one it is given.
* Decision Q2 of the claim issue left this work to a follow-up, to keep that issue under the size limit.

# Approach
1. `start-issue` becomes the opening of a worktree: any name, at the path the user gives (default: next to the main
   worktree, `../<repo>-<name>`, the place of ADR 0003), a branch the user names (default: the name) cut from the
   trunk, with the copy of `cardinal.local.toml` kept; no issue file, no `info/exclude` line.
2. A new ADR supersedes ADR 0008: worktrees no longer nest under `.claude/worktrees/`; PAR-13 is withdrawn, PAR-14
   stays (the scanners still read only git's files).
3. `finish-issue` becomes the removal of a worktree on demand, under the same safety (merged, clean, not from
   inside), with no issue stem, and still removes the worktrees left under `.claude/worktrees/`.
4. Rename the recipes if the names mislead (`open-worktree`, `close-worktree`); a rename goes to
   `migrations/<n>.toml` (ADR 0017) and the docs that name them.
5. Keep PAR-3 for the branches made by the old `start-issue` until none is left, or withdraw it.

# Specs
* `specs/parallel-issues.md`: PAR-1, PAR-2, PAR-6, PAR-10, PAR-11, PAR-15 amended, PAR-13 withdrawn, maybe PAR-3.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/issue_worktree.rs`, `files/cardinal.just` (`start-issue`, `finish-issue`).
* `docs/adr/0022-a-session-claims-its-issue-and-locks-its-workspace.md`, `docs/adr/0011-the-user-opens-the-issue-worktree.md`,
  `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`, `docs/adr/0003-one-worktree-per-issue.md`.
* `files/.claude/skills/issue-workflow/details.md` ("Worktree"), `docs/usage.md` (Parallel issues in worktrees),
  `docs/workflow.md` (Parallel issues).

# Hints
* The agent still never creates, enters or leaves a worktree (ADR 0011): these recipes stay the user's.

# Tracking

Plan: validated

## Understanding
The recipes open and remove a worktree with no issue attached: `open-worktree <name>` at `../<repo>-<name>` (or a
given path and branch), `close-worktree <name|path>` under the safety of `finish-issue`. Out of scope: the claim, the
lock, the agent's rules (ADR 0011 stands). ADR 0023 supersedes ADR 0008.

## Plan
1. [x] Move the tool: `issue_worktree.rs` becomes `worktree.rs` (source and test), no other edit, the tool keeps
   its name `issue-worktree` until deliverable 2 (an edit of a moved file counts it twice); verified by `just check`;
   makes true nothing new (a pure move, counted apart).
2. [x] `open-worktree` replaces `start-issue`: any name, sibling path or `--path`, branch `<name>` or `--branch`,
   no issue file, no `info/exclude` line, personal configuration copied; ADR 0023, `migrations/6.toml`, docs;
   verified by the behavior test of the tool; makes true PAR-1, PAR-2, PAR-17 (PAR-13 withdrawn).
3. [x] `close-worktree` replaces `finish-issue`: a path or a name (sibling, `.claude/worktrees/`, branch checked out,
   bare branch), same safety, detached worktree; `Stale:` names it; `migrations/6.toml`, docs, spec and ADR accepted;
   verified by the behavior tests of the tool and of `next-issue`; makes true PAR-4, PAR-6, PAR-7, PAR-10, PAR-11,
   PAR-15, PAR-19.
4. [x] Docs of the whole issue and the fixes of its `high` review; verified by the behavior test of the tool;
   makes true PAR-2, PAR-6, PAR-10, PAR-19 (amended).

## Decisions
- Mode: autonomous
- Base commit: 18385b986653a0ceb7e988b70256510f64ba33e3
- Q1 a: `open-worktree` checks out an existing branch checked out nowhere as it is.
- Q2 a: PAR-3 stays for the branches of the former `start-issue`.
- Q3 a: `start-issue` and `finish-issue` are removed, declared in `migrations/6.toml`, no alias.
- Q4 a: `warn_when_starting_a_blocked_issue` is withdrawn in deliverable 2 (moved to `issues/done/`, marked withdrawn).

## Deliverable 1
- done: `issue_worktree.rs` moved to `worktree.rs` (source and test, exact renames); spec draft and ADR 0023 draft
  committed with it.
- red: none (a move, no new test).
- not verified: nothing.
- Code review: 1 findings; the `issue-worktree` entry had moved in `TOOLS`: put back in place.

## Deliverable 2
- done: `just open-worktree <name> [--path] [--branch]` (tool `worktree open`, `[no-cd]` recipe) replaces
  `start-issue`; no issue file, no `info/exclude` line; `migrations/6.toml` names `start-issue`; docs; the issue
  `warn_when_starting_a_blocked_issue` withdrawn to `issues/done/`.
- red: PAR-1, PAR-2, PAR-17: `cardinal: unknown tool `worktree``.
- not verified: the recipe through `just` from a subdirectory (`[no-cd]`), only the tool is tested.
- follow-up: `finish-issue` still names the branch; deliverable 3.
- Code review: 1 findings; `--path` or `--branch` with no value was ignored: now refused with the usage.

## Deliverable 3
- done: `just close-worktree <name|path> [--force]` (tool `worktree close`, `[no-cd]`) replaces `finish-issue`: a
  path, or a name (`<repo>-<name>`, `.claude/worktrees/<name>`, the worktree of branch `<name>`, the branch alone);
  refuses the main worktree; a detached worktree loses no branch; `Stale:` names it; `migrations/6.toml` names
  `finish-issue`; spec `accepted`, ADR 0023 `Accepted`.
- red: PAR-4, PAR-6, PAR-7, PAR-10, PAR-11, PAR-15, PAR-19: `cardinal run worktree close` printed the usage (no
  `close` subcommand); `next-issue` named `finish-issue a`.
- not verified: the recipes through `just` (`[no-cd]`), only the tool is tested.
- Code review: 1 findings; a detached worktree with commits off trunk was removed without `--force`: its HEAD now gets the merge check (test added).

## Deliverable 4
- done: `update-docs` over `18385b9..HEAD` (no stale page left); fixes of the `high` review: `close-worktree <name>`
  refuses a name designating both a reused worktree and a branch (the `Stale:` hint would have removed the worktree),
  compares `refs/heads/<branch>` (a tag of the same name stood for it), checks the HEAD of a detached worktree whose
  directory is gone, deletes the branch with `-D` after its own merge check; `open-worktree --path` refuses a path
  inside the main worktree; one `git worktree list` per run. PAR-2 and PAR-19 amended.
- red: PAR-2, PAR-6, PAR-10, PAR-19: on the code of deliverable 3 the test panicked at `worktree.rs:486` after
  `open --path .claude/worktrees/q` was accepted.
- Code review (high): 8 findings; 1, 3, 4, 5, 6, 8 fixed; 2 became
  `issues/done/count_the_sessions_of_a_closed_worktree_with_its_repository.md` (EVA-7, outside PAR); 7 not fixed: the
  renames sit in `migrations/6.toml`, format 6 being unreleased, and a recipe rename raises no format (ADR 0014), so
  a consumer already synced to an unreleased format 6 gets no warning.
- not verified: the recipes through `just` (`[no-cd]`).
- Code review (low): 1 findings; the unused `head` of a branch-only target dropped.

Done: deliverables 1 to 4 committed (a8ad997, b2c827f, 58c2094, 7c21f89); left: the follow-up issue `count_the_sessions_of_a_closed_worktree_with_its_repository`.
