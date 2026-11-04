---
id: REV
status: accepted
adrs: [0004, 0005]
---

# Code review gate

## Intent

A `git commit` waits until the diff has been through a code review. The native git `pre-commit` hook (installed by
`cardinal`, see `specs/sync.md`) runs `cardinal run review-gate native` and judges the repository git resolved; the Claude
hook `cardinal run review-gate` only records the review and refuses what would skip the native hook or stage every file with the deliverable. The gates catch the
mistakes of a cooperative agent, not deliberate bypasses (ADR 0005).

## Requirements

- **REV-1** (deterministic) withdrawn: the native hook judges the repository git resolved (REV-9); the command text is no longer read for it.
- **REV-2** (deterministic) withdrawn: see REV-1.
- **REV-3** (deterministic) withdrawn: see REV-1.
- **REV-4** (deterministic) withdrawn: see REV-1.
- **REV-5** (deterministic) withdrawn: see REV-1.
- **REV-6** (deterministic) withdrawn: see REV-1.
- **REV-7** (deterministic) withdrawn: see REV-1.
- **REV-8** (deterministic) withdrawn: see REV-1.
- **REV-9** (deterministic) `cardinal run review-gate native` (run by the native `pre-commit` hook) must judge the repository git resolved for the commit (the `GIT_DIR` and work tree git passes, or the current directory when it passes none), never the text of a command, and must refuse a commit while the diff holds a file the last recorded review never saw, except when there is nothing to commit, when the diff is under `issues/` only, when what the commit records (the index git commits, against `HEAD`) is not empty and lies under `issues/` only, whatever else the work tree holds, or as REV-20 says (both `issues/` exceptions only while the project keeps the feature `issue-files` on, FEA-36), or when the waiver of the repository lets it through.
- **REV-10** (deterministic) `native` must exit non-zero when it refuses and zero otherwise, and its refusal must tell the agent to run `/code-review low` on the diff. A waiver it cannot read or cannot stamp must let nothing through: `native` must judge the commit as if there were no waiver, and a refusal must then also tell the agent to ask the user to fix that waiver, naming its path.
- **REV-11** (deterministic) `native` must judge a linked worktree by its own marker and waiver (the git directory of that worktree), and a repository reached through `--git-dir` by that git directory.
- **REV-12** (deterministic) `check` must refuse a `git commit` line carrying `--no-verify` or `-n` (also in a bundle of short options such as `-nm`; `-am` and a `-n` inside a quoted message are not one), and a line that sets `core.hooksPath` (`-c core.hooksPath=…`, `--config-env=core.hooksPath=…`, `git config core.hooksPath …`), since they skip the native hook; the message must say why.
- **REV-13** (deterministic) withdrawn: the deny rules of the seeded settings (REV-19) and the instruction of the issue workflow replace the reading of the command text.
- **REV-14** (deterministic) withdrawn: see REV-13.
- **REV-15** (deterministic) The path `git rev-parse --git-path claude-review-waived` gives from a linked worktree must be the waiver `native` reads for that worktree, not the one of the main worktree.
- **REV-16** (deterministic) withdrawn: see REV-1.
- **REV-17** (deterministic) The recipe `waive-review` of `cardinal-review.just` must create the waiver at the path `git rev-parse --git-path claude-review-waived` gives from the directory it runs in, so that `native` finds it for that worktree (the main one or a linked one, as REV-15), and must print that path. It must change nothing else in the repository.
- **REV-18** (deterministic) withdrawn: see REV-13.
- **REV-19** (deterministic) The managed entries of `.claude/settings.json` (`entries/.claude/settings.json` of the source, SYN-29) must hold the `permissions.deny` rules `Bash(just waive-review)`, `Bash(just waive-review *)` and `Edit(//**/claude-review-waived)`, so that Claude Code refuses the agent the recipe and an edit of the waiver.
- **REV-20** (deterministic) `native` must let through a commit whose recorded diff (the index git commits, against `HEAD`) holds, outside `issues/`, only modified regular files, of the text files `close-issue` rewrites and with their mode kept, each equal, in the index, to its `HEAD` content once the references of CLS-1 and CLS-2 are rewritten for the stems `<stem>` whose `issues/<stem>.md` the index deletes and whose `issues/done/<stem>.md` it adds, and, in a `roadmap` document, with or without the milestones of CLS-8 set `done` for those stems, the existence of each listed issue read in the index, never in the text of the roadmap alone; any other change outside `issues/` (an added, deleted or binary file, a submodule, a changed mode, a file `close-issue` never rewrites, an edit on a line or elsewhere in such a file) must leave the commit to REV-9. It applies only while the project keeps the feature `issue-files` on (FEA-36).
- **REV-21** (deterministic) `check` must refuse a line holding a `git add` that stages every file, with `-A` (alone or in a bundle of short options), `--all`, or `.`, `./` or `:/` among its arguments and no `-n` nor `--dry-run` (its global options skipped, in any command of a chained line such as `git add -A && git commit`), since it stages personal and unrelated files with the deliverable; a quoted string, the body of a here-document and a comment are no command; its message tells the agent to add the files of the deliverable by name.

## Scenarios

- (REV-9) Given a commit whose repository the command text does not name readably, when that repository holds an unreviewed change, then the native hook refuses it, since git resolved the repository.
- (REV-9) Given a modified file nobody reviewed, when only an issue file is staged and committed (`git add issues/x.md` then `git commit`, or `git commit -- issues/x.md`), then the native hook lets it through; `git commit -a` is still refused.
- (REV-20) Given `issues/foo.md` moved to `issues/done/foo.md` by `close-issue` and `ROADMAP.md` rewritten by it, when both are staged and committed with no review, then the native hook lets the commit through; with `ROADMAP.md` also setting `done` the milestone whose last open issue was `foo` (CLS-8), it passes too; with `ROADMAP.md` setting `done` a milestone that still lists an open issue or that does not list `foo`, or one rewritten link next to an edited word, it is refused.
- (REV-11) Given a linked worktree reviewed and the main worktree not, then a commit in the linked worktree passes.
- (REV-12) Given `git commit -m "fix -n"` or `git commit -am x`, then neither counts as `-n`: the Claude hook lets it through.

## Non-goals

- A deliberate bypass, or an unlikely command of a cooperative agent that slips past a gate (ADR 0005): `just -f x waive-review`, a waiver written by `touch` or a script, a variant of `--no-verify` the Claude hook does not read.
- Stopping `--no-verify` or `core.hooksPath` outside the Claude hook, a commit made by another tool, or an edited or deleted `.git/hooks/pre-commit`: the native hook is a last resort, not a barrier.
- A repository without the native hook: no review is required there; `cardinal check` reports a missing hook (SYN-23), `sync` says it installs none under `core.hooksPath` (SYN-22).
