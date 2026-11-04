# Create the issue worktrees under .claude/worktrees

# Goal
Entering the worktree of an issue costs the user one approval per issue: `issue-workflow` step 0 calls
`EnterWorktree` on `../<repo>-<stem>`, and Claude Code asks for every path outside the repository's
`.claude/worktrees/`, whatever the permission rules say. Create the issue worktrees at `.claude/worktrees/<stem>`
instead, so that starting or resuming an issue needs no interaction.

# Findings
* Transcript of an issue session of this repository (2026-10-27): the only tool call approved by hand is
  `EnterWorktree` on `../agent-harness-<stem>`, recorded with `permissionDecision` `source: user_temporary`,
  `reasonCode: safetyCheck`; `just start-issue` was accepted by the `Bash(just *)` rule of the settings.
* Claude Code documentation, page "worktrees" (read 2026-10-27): "When Claude enters a path outside the repository's
  `.claude/worktrees/` directory, Claude Code asks for your approval first [...]. An `EnterWorktree` permission rule
  or choosing "don't ask again" doesn't suppress this prompt; only `bypassPermissions` mode skips it."
* The sibling path is fixed by ADR 0003 ("next to the main one (`../<repo>-<stem>`)"), by PAR-1 of
  `specs/parallel-issues.md` ("outside the repository, in the sibling directory `<repo>-<stem>`"), and is computed by
  `sibling()` in `files/.claude/skills/issue-workflow/issue_worktree.rs`. It is also written in
  `files/harness.just` (`start-issue` comment), `files/.claude/skills/issue-workflow/SKILL.md` (step 0) and
  `docs/architecture.md`.
* `check_specs.rs` already skips `.claude/worktrees` when it collects the test files; `close_issue.rs` skips
  `skipped_dirs` of `.claude/harness.toml`. Other scanners of the repository tree were not reviewed.
* This repository's `.gitignore` does not ignore `.claude/worktrees/`, and it un-ignores `.claude/`: an issue
  worktree there would show as untracked in the main worktree.

# Approach
1. `issue_worktree.rs start` creates `<main worktree>/.claude/worktrees/<stem>` (creating the parent directory) and
   prints that path; `finish` and the taken detection follow, since they read the path from `git worktree list`.
2. Ignore `.claude/worktrees/` in every consumer: either `start-issue` writes it to `.git/info/exclude` (nothing to
   sync, works in any project), or the harness ships or checks a `.gitignore` line. Compare both.
3. List every script that walks the repository tree (spec checker, close-issue, transcript readers, `harness
   check`/`sync`, the hooks) and make each skip `.claude/worktrees`, so that a main worktree scan never reads the
   issue worktrees nested in it.
4. Update step 0 of `issue-workflow`, the recipe comments, `docs/architecture.md`, and amend ADR 0003 (or supersede
   it with a new ADR) for the new location.
5. Existing sibling worktrees: keep supporting them until their `finish-issue` (resume by path, with the approval),
   or document a manual move with `git worktree move`.

# Specs
* `specs/parallel-issues.md`: amends PAR-1 (location of the worktree); possibly a new requirement for the ignore
  rule and for the scanners skipping `.claude/worktrees`.

# Resources
* `docs/adr/0003-one-worktree-per-issue.md`, `specs/parallel-issues.md`.
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `SKILL.md` step 0, `details.md`.
* `files/.claude/specs/check_specs.rs` (existing `.claude/worktrees` skip) and its test.
* Claude Code docs: https://code.claude.com/docs/en/worktrees ("Ask Claude to create a worktree", "How Claude Code
  enforces isolation").

# Hints
* A nested worktree holds its own `Cargo.toml` workspace root, so cargo run inside it stops there; check that
  cargo run in the main worktree does not pick up the nested crates (it should not, the workspace members are
  listed explicitly, but verify).
* The hooks receive `CLAUDE_PROJECT_DIR` of the main worktree and the `cwd` of the event (see PAR-12): with the
  worktree nested in the main one, a check "is this path under the project dir" no longer tells the two apart.
* Do not reach for `bypassPermissions`: it removes every prompt, not only this one.

# Tracking

Plan: validated

## Understanding
`start-issue` nests the issue worktree at `.claude/worktrees/<stem>`, so `EnterWorktree` needs no approval; git
ignores it through `info/exclude`, the tree scanners skip it, `finish-issue` still removes the old siblings. Out of
scope: `bypassPermissions`, moving existing worktrees. Hooks reviewed: they resolve the work tree from the event's
`cwd` (`git rev-parse --show-toplevel`), which a nested worktree answers by itself, so they need no change.

## Plan
1. [x] Nested worktree: `issue_worktree.rs start` creates `.claude/worktrees/<stem>` and writes the exclude line,
   `finish` accepts the new path and the legacy sibling; step 0 of `issue-workflow` finds the worktree of the branch
   in `git worktree list`; recipe comment, `details.md`, `docs/architecture.md`, ADR 0008; verified by
   `test_issue_worktree.rs` (and a check that cargo in the main worktree ignores the nested crates); makes true
   PAR-1, PAR-13, PAR-15.
2. [x] Scanners: `close_issue.rs` skips `.claude/worktrees/` of the root it walks (by path, not by name), the
   existing `check_specs.rs` skip gets its citation; verified by `test_close_issue.rs`, `test_check_specs.rs`;
   makes true PAR-14.
3. [x] Transcripts: `repository()` of `transcripts.rs` maps a removed `<repo>/.claude/worktrees/<x>` cwd to
   `<repo>`; verified by `test_session_metrics.rs`; makes true EVA-7 (amended).
4. [x] Fixes of the `high` review: `info/exclude` appended as bytes, written after `git worktree add`; `finish`
   refuses a worktree holding another one; `repository()` falls back to the sibling rule; module doc and test
   name; verified by `test_issue_worktree.rs`, `test_session_metrics.rs`; makes true PAR-7 (amended), PAR-13.

## Decisions
- Mode: autonomous
- Base commit: bd97fda99253dbce610398148e1d580b2ab9be90
- Ignore rule: `info/exclude` of the git common directory, written by `start-issue` (ADR 0008), not a `.gitignore`
  line shipped or checked by the harness.
- ADR 0008 supersedes ADR 0003 for the location only.

## Deliverable 1
- done: `start-issue` creates `.claude/worktrees/<stem>` and adds `/.claude/worktrees/` to `info/exclude` once;
  `finish-issue` also removes a worktree left at `../<repo>-<stem>`; step 0 finds the worktree by its branch.
- red: the test panicked writing `.claude/worktrees/x/work.txt` (NotFound): `start` still made the sibling.
- Cargo: with a nested copy of the workspace, `cargo metadata` in the main worktree lists only its own crate, and in
  the copy only the copy's (each stops at its own workspace root).

## Deliverable 2
- done: `close_issue.rs` skips `.claude/worktrees/` of the root it walks (full path, so `docs/worktrees/` is still
  scanned), for a close and for `--relink`.
- red: `FAIL - the files of a worktree under .claude/worktrees are untouched` (and the same for `--relink`).
- Deviation: the citation of PAR-14 is in `test_close_issue.rs` only; the `check_specs.rs` half is proven by the
  existing cases of `test_check_specs.rs`, which sit in its `main` and carry no citation, like the rest of that file.

## Deliverable 3
- done: `repository()` maps a removed cwd under `<repo>/.claude/worktrees/` to `<repo>`; the scripts that include
  `transcripts.rs` carry its new SHA-256; `specs/parallel-issues.md` and `specs/evaluation.md` back to `accepted`.
- red: `FAIL - a removed worktree under .claude/worktrees counts with its repository`.

## Deliverable 4
- done: findings 1, 2, 3, 4, 6, 7, 8 of `/code-review high` fixed (see the plan line); findings 5 and 10 (walk the
  tree through git; consumer tools that see the nested worktrees) are the new issue
  `list_the_repository_files_through_git`; finding 9 (ADR 0003 marked superseded although only its location
  changed) kept: `docs/adr/README.md` asks to mark the old ADR `Superseded by NNNN`, and ADR 0008 says the rest
  stands.
- red: `FAIL - info/exclude keeps its earlier lines, even not in UTF-8` and `FAIL - finish refuses, even with
  --force, a worktree holding another worktree`; the write order and the `transcripts.rs` fallback are refactors
  under green tests.
- Code review (`low`): 1 finding, fixed: a failing `git worktree list` no longer counts as "no inner worktree".
