---
id: SIZ
status: accepted
adrs: [0009]
---

# Size of a deliverable

## Intent

`deliverable-size` counts the changed lines of a deliverable, or of a whole issue, and says whether they exceed the
size limit of `skill:issue-workflow`, so the review step stops on a number the agent did not estimate.

## Requirements

- **SIZ-1** (deterministic) With no argument, `deliverable-size` must count the added plus removed lines of the working tree (staged and unstaged) against `HEAD` (the empty tree when the branch has no commit yet), plus every line of each untracked file git does not ignore, as `git diff` counts a new file.
- **SIZ-2** (deterministic) With an argument `<rev>`, `deliverable-size` must count the working tree and its untracked files against `<rev>` instead of `HEAD`.
- **SIZ-3** (deterministic) With an argument `<a>..<b>`, `deliverable-size` must count the changes between the commits `<a>` and `<b>` alone, leaving out the working tree and the untracked files.
- **SIZ-4** (deterministic) `deliverable-size` must not count the files under `issues/` (unless the project turns off the feature `issue-files`, FEA-37), nor the lock files: a file named `*.lock`, `package-lock.json`, `npm-shrinkwrap.json`, `pnpm-lock.yaml` or `go.sum`.
- **SIZ-5** (deterministic) A file renamed with its content unchanged must count 0 lines, whether it was moved by `git mv` or by a plain `mv` that left the new path untracked.
- **SIZ-6** (deterministic) A file renamed whose content also changed must count every line of its old content as removed and every line of its new content as added, also when the rename and the edit are in different commits of the range.
- **SIZ-7** (deterministic) A file git reads as binary must count 0 lines, tracked or untracked.
- **SIZ-8** (deterministic) `deliverable-size` must compare the count with `deliverable_size_limit` of the `[project]` table of `.claude/cardinal.toml` when given no argument, else with `issue_size_limit`, defaulting to 1000 and 3000 when the file or the key is absent.
- **SIZ-9** (deterministic) `deliverable-size` must print one line `<lines>\t<path>` per counted file with a non-zero count, largest first then by path, then a last line `<total> changed lines, limit <limit>: within` or `...: over`, and exit 0 when the total is at most the limit, 1 when it exceeds it.
- **SIZ-10** (deterministic) When git fails (no repository, an unknown revision), the argument starts with `-`, the configuration cannot be read or a limit of it is not a positive integer, `deliverable-size` must print the reason on stderr and exit 3, printing no count.
- **SIZ-11** (deterministic) `deliverable-size` must leave the index of the repository unchanged.

## Scenarios

- (SIZ-1) Given a repository with no commit and an untracked file of 3 lines, when `deliverable-size` runs, then the total is 3.
- (SIZ-1) Given a commit, then a tracked file with 2 lines changed and an untracked file of 3 lines, when `deliverable-size` runs, then the total is 7 (2 added, 2 removed, 3 added).
- (SIZ-2) Given commits `A` then `B` adding 4 lines, and 1 untracked line, when `deliverable-size A` runs, then the total is 5.
- (SIZ-3) Given commits `A` then `B` adding 4 lines, and 1 untracked line, when `deliverable-size A..B` runs, then the total is 4.
- (SIZ-4) Given changes to `issues/a.md`, `Cargo.lock` and `src/a.rs` of 1 added line each, when `deliverable-size` runs, then only `src/a.rs` is listed and the total is 1.
- (SIZ-5) Given a committed file of 10 lines moved by `git mv`, when `deliverable-size` runs, then the total is 0.
- (SIZ-5) Given a committed file of 10 lines moved by `mv`, when `deliverable-size` runs, then the total is 0 and `git status` is unchanged (SIZ-11).
- (SIZ-6) Given a commit `A`, a commit moving its file of 10 lines, then a commit changing 1 of them, when `deliverable-size A..HEAD` runs, then the total is 20.
- (SIZ-7) Given an untracked file holding a NUL byte, when `deliverable-size` runs, then the total is 0.
- (SIZ-8) Given `deliverable_size_limit = 5` and 6 changed lines, when `deliverable-size` runs, then the last line ends with `limit 5: over` and it exits 1.
- (SIZ-9) Given 3 changed lines in `b.rs` and 3 in `a.rs`, when `deliverable-size` runs, then it prints `3\ta.rs`, `3\tb.rs`, then `6 changed lines, limit 1000: within`, and exits 0.
- (SIZ-10) Given `issue_size_limit = "big"`, when `deliverable-size HEAD` runs, then it exits 3.
- (SIZ-10) Given any repository, when `deliverable-size --output=x` runs, then it exits 3 and writes no file `x`.

## Non-goals

- Counting the deliverables of an issue (at most 6): the plan of the issue lists them.
- Estimating a deliverable before its code exists: step 1 of `skill:issue-workflow` still estimates the plan.
- A configurable list of left-out files: the lock files named above cover the usual stacks.
