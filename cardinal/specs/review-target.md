---
id: RVT
status: accepted
adrs: []
---

# Target of a code review

## Intent

`review-target` prints the target `skill:issue-workflow` hands to `/code-review`: the diff of a deliverable, or of a
whole issue, without the tracking of the issues nor the copies `cardinal sync` manages, so the reviewers read the code,
tests, specs and docs once, and not the 18 KB per session of `oversized-result-review` that tracking and copies cost.

## Requirements

- **RVT-1** (deterministic) With an argument `<base>`, `review-target` must print one line `'<base>' -- ':/'`, then the exclusions of RVT-3, each a word separated by one space, and exit 0: the working tree against `<base>`, so the review of a whole issue also reads its uncommitted edits.
- **RVT-2** (deterministic) With no argument, `review-target` must print the line of RVT-1 with `HEAD` as `<base>`: the diff of a deliverable before its commit.
- **RVT-3** (deterministic) The exclusions must be `':(top,exclude,literal)issues'` (unless the project turns off the feature `issue-files`, FEA-39), then `':(top,exclude,literal)<path>'` for `.claude/cardinal.lock` and each managed file it lists, sorted by path; none when the project has no lock.
- **RVT-4** (deterministic) When the argument starts with `-`, the revision (`HEAD` with no argument) is not one git resolves, more than one argument is given, the directory is not in a git repository, or the lock cannot be read, `review-target` must print the reason on stderr and exit 3, printing nothing on stdout.
- **RVT-5** (deterministic) Each word of the line but `--` must be quoted for a POSIX shell, a `'` written `'\''`, so a shell splits the line into the words of RVT-1 unchanged.
- **RVT-6** (deterministic) When the work tree holds an untracked file git does not ignore outside the exclusions of RVT-3, `review-target` must print on stderr its path, with the command `git add -N` that marks it intent-to-add, and exit 3, printing nothing on stdout: `git diff` leaves an untracked file out, and the commit gate would record it as reviewed.

## Scenarios

- (RVT-1) Given a lock listing `.claude/rules/a.md` and `AGENTS.md`, when `review-target abc123` runs in a repository where `abc123` resolves, then it prints `'abc123' -- ':/' ':(top,exclude,literal)issues' ':(top,exclude,literal).claude/cardinal.lock' ':(top,exclude,literal).claude/rules/a.md' ':(top,exclude,literal)AGENTS.md'`.
- (RVT-2) Given no lock, when `review-target` runs with no argument, then it prints `'HEAD' -- ':/' ':(top,exclude,literal)issues'`.
- (RVT-4) Given `review-target --stat`, then it exits 3 and prints nothing on stdout.
- (RVT-6) Given an untracked `src/new.rs` and an untracked `issues/x.md`, when `review-target` runs, then it exits 3 naming `src/new.rs` only; after `git add -N src/new.rs` it prints the line.

## Non-goals

- What the reviewers read: `/code-review` takes the line as its target and runs the diff itself; a reviewer may still read more.
- What the commit gate records: `review-gate record` keeps the whole diff as seen (REV-9), whatever the target.
- The lock files of the dependencies (`Cargo.lock`): the target keeps them as `git diff` shows them.
- The managed files of the revisions under review: the lock is read from the work tree, so a file a sync of the range stopped or started managing is judged as it is now.
- An edit of a managed copy in a consumer, against its rules: `cardinal check` (`just check-cardinal`) reports the drift, not the review.
