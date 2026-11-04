# Make the issue worktree optional and opened by the user

# Goal
`issue-workflow` puts every issue in its own worktree: step 0 runs `just start-issue` and moves the session there
with `EnterWorktree`. Claude Code then isolates the session and refuses any Bash command it cannot prove stays in the
worktree, which refused dozens of ordinary commands in the last issue. Yet most issues are worked one at a time, with
no parallel session, and need no worktree. Make the worktree the user's choice: the agent works where the session was
started, the main worktree or a linked one the user opened for parallel work, and never creates or enters a worktree.

# Findings
* Session of `ship_the_harness_scripts_as_prebuilt_binaries` (2026-10-27): refused with "too complex to verify that it
  stays inside the worktree" or "cannot be shown not to run git": heredocs feeding `python3` whose text named `git`,
  `for` loops running `sed` with a variable, a script under `/tmp` running `git -C`, `time ( ... )`, a pipe into the
  launcher, commands chaining `git add -N` with other commands (refused whole, so the following steps silently did not
  happen and a review ran without the new files).
* The workarounds were scripts written to a temporary directory and run with paths as arguments, one plain command
  per call; each refusal cost a call and its context.
* The guard is Claude Code's, not the harness's: it comes with `EnterWorktree`, nothing in this repository or the user
  settings produces it.
* The user works without a worktree most of the time, one issue at a time; a worktree is only worth it for parallel
  sessions.

# Approach
1. Check that a session started directly in a linked worktree (`cd .claude/worktrees/<stem> && claude`) does not have
   the guard: run there one of the refused shapes above. Check also that the hooks (`clippy-gate`, `review-gate`) and
   the memory of the project work in such a session.
2. Step 0 of `issue-workflow`: drop the worktree handling (no `start-issue`, no `EnterWorktree`, no check of where the
   session runs); the agent reads the issue and starts.
3. Closing:
   1. In the main worktree: unchanged (`just close-issue`, commit).
   2. In a linked worktree: the agent commits, rebases on the main branch, runs `just check-harness`, then stops and
      prints the commands left to the user (`git merge --ff-only <stem>`, `just close-issue`, `just finish-issue`),
      without running them. No `ExitWorktree`.
4. Docs: `details.md` ("Worktree of an issue" becomes an optional mode the user opens with `just start-issue`; "Merge
   back"), a new ADR superseding ADR 0003 (worktree optional, opened by the user), ADR 0008 reread, `docs/architecture.md`
   and `CLAUDE.md` (the "one git worktree per issue, started by `issue-workflow`" line).

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (step 0, "Merge back", Rules), `details.md`
* `docs/adr/0003-one-worktree-per-issue.md`, `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`
* `harness.just` (`start-issue`, `finish-issue`), `issue-worktree` tool in `crates/harness/src/tools/`

# Hints
* Keep it simple: less automation rather than a workaround of the guard (no guide of command shapes, no recipes for
  the refused multi-step operations).
* `start-issue` and `finish-issue` stay, as the user's tools; `just next-issue` keeps skipping an issue whose branch
  or worktree exists.
* `close-issue` refuses in a linked worktree on purpose (the queue is never edited on an issue branch): keep it.

# Tracking

Plan: validated

## Understanding
The agent works where the session runs and never creates, enters or leaves a worktree; the user opens one
(`just start-issue`, then `claude` launched in it) only for parallel work, and merges it. Out of scope: the
`issue-worktree` tool and its spec, which stay as they are. Claude Code docs (page "worktrees", "How Claude Code
enforces isolation"): the guard applies to a session started with `--worktree`, entered with `EnterWorktree`, or
resumed in one; a plain `claude` launched in a worktree made by `git worktree add` is not listed.

## Plan
1. [x] Optional worktree: `issue-workflow` step 0, "Merge back" and Rules, `details.md`, ADR 0011 (supersedes the
   "each issue gets a worktree" of 0003/0008), `docs/architecture.md`, `CLAUDE.md`; verified by `just sync`,
   `just check-harness`, `just check` and a reread.
2. [x] Fixes of the `high` review: step 0 stops on an issue taken elsewhere; `check-harness` right after a sync that
   adds a hook; hooks declared by the worktree in a session launched there; rebase again when the fast-forward
   refuses; `low` review of the close commit; ADR 0008 partly superseded; no ADR number in the shared skill.

## Decisions
- Mode: autonomous
- Base commit: f9c188ed424a9b94acca7f75cb588e9afef4fb1d

## Summary of deliverable 1
- done: `issue-workflow` works where the session runs (no `start-issue`, `EnterWorktree` nor `ExitWorktree`); in a
  linked worktree the close ends with a rebase and the commands printed for the user; ADR 0011; `details.md`,
  `docs/architecture.md`, `CLAUDE.md`, `README.md` and the `adrs` of `specs/parallel-issues.md` follow.
- red: none (docs and skill text, no test).
- Code review: 1 finding, fixed (the "Merge back" pointer named the last Rule, which is not the closing one).
- checks: `just check` ok, `just check-harness` ok, `just sync` done.
- not verified: that a plain `claude` launched in an issue worktree has no isolation refusal (taken from the Claude
  Code docs); to see at the next parallel issue.

## Summary of deliverable 2
- done: the fixes of the `high` review listed in the plan.
- red: none (docs and skill text, no test).
- Code review: no findings (`low`, on the fixes). The `high` review: 10 findings; 9 fixed, 1 left (a session in a nested worktree may load the main `CLAUDE.md` as
  an ancestor: written in ADR 0011 as not measured, follow-up to decide with the user).
