---
priority: medium
depends_on: [tell_the_needed_oversized_results_from_the_waste]
why: 'the review reviewers waste 18 KB per session, mostly the whole-issue diff, a fifth of it with no code to review'
---
# Leave the issue file and the synced copies out of the issue review

# Goal
The `high` review of a whole issue (`skill:issue-workflow`, after the last deliverable) reads `base..HEAD`, issue
file and synced copies of Cardinal included. Its reviewers' results are the largest token waste measured
(`oversized-result-review`). The outcome: the review reads the code, tests, specs and docs, not the tracking of the
issue nor a second copy of a managed file, and the waste per review session falls.

# Findings
* Measured 2026-11-03 (`issues/done/tell_the_needed_oversized_results_from_the_waste.md`, deliverable 3):
  `oversized-result-review` 18,078 bytes per session over 262 sessions, against 3,708 for the session's own results.
* The reviewers' results over 25,000 bytes are the diff: 102 `git diff`, 75 whole `Read`s of the `tool-results/`
  file Claude Code saved the diff to, 13 others; 21 of the `git diff` already left paths out.
* By changed lines, the whole-issue diffs of 204 closed issues of 7 projects: code and tests 64%, `issues/` 13%,
  `.claude/` 9% (synced copies), `files/` 8%, `specs/` and `docs/` 4%.

# Approach
1. Check how the built-in `/code-review` takes a path filter (a pathspec after the range, `:!issues`, or words in
   its arguments the reviewers follow).
2. Name in `skill:issue-workflow` the target of the `high` review: `base..HEAD` without `issues/` and the managed
   paths of `.claude/cardinal.lock`; keep `specs/` and `docs/`, the requirements the code is read against.
3. Decide whether the `low` reviews of step 3.5 take the same filter (the commit gate must still record every
   file of the diff as seen).

# Milestone
Lean agent loop

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (step 3, the last paragraph; step 3.5), `specs/review-gate.md`.
* `specs/rule-breaches.md` BRE-22.

# Hints
* The commit gate (`cardinal run review-gate native`) refuses a commit whose diff holds a file the last review never
  saw: a filtered `low` review could block the commit.
* Verdict: `Expected-effect: bytes_per_session:oversized-result-review down`. (autonomous)

# Specs
* [specs/review-target.md](../../specs/review-target.md) (RVT-1 to RVT-5), FEA-39 of [specs/features.md](../../specs/features.md).

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
The reviews of `skill:issue-workflow` get a target that leaves out `issues/` and the managed copies of the lock, built
by a new tool `cardinal run review-target [<base>]`, since `/code-review` passes its arguments verbatim as the
`Review target:` its reviewers diff (they already add `':!issues'` on their own). Out of scope: lock files, untracked files.

## Plan
1. [x] Tool `review-target`: prints `<base>..HEAD -- ':/' ':(top,exclude)issues' ':(top,exclude)<managed>'...`, or
   `HEAD -- ...` with no argument; verified by tests through the binary; makes true RVT-1 to RVT-5, FEA-39. (autonomous)
2. [x] `skill:issue-workflow`: step 3.5 invokes `/code-review low <the line of review-target>`, the whole-issue review
   `/code-review high <the line of review-target <base>>`; docs (`docs/architecture.md`, CLAUDE.md index if needed);
   verified by `just check-cardinal` and a reread; `Expected-effect: bytes_per_session:oversized-result-review down`.
3. [x] Fixes of the `high` review: every revision quoted, `<base>` alone (the working tree against it, so the
   uncommitted docs edits are reviewed), `literal` pathspecs, the lock excluded, an unborn `HEAD` refused, an untracked
   file refused with its `git add -N` (RVT-6); verified by the tests of `review_target.rs`. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 513aae292a5ffc35640c5d7b33606d316bdd33f6
- Lock files of the dependencies stay in the target (question 1 a): out of scope.
- `/code-review` (built-in) takes no path filter of its own: its arguments become the `Review target:` line of the
  reviewer's prompt, which runs the diff of that target, so a pathspec after the range is followed.
- The `low` reviews take the same filter: `review-gate record` snapshots the whole diff whatever the reviewer read
  (`record` in `crates/cardinal/src/tools/review_gate.rs`), so a filtered review cannot block the commit.

## Deliverable 1
- done: `cardinal run review-target [<base>]` prints the review target without `issues/` and the managed files of the
  lock; `managed_files` of `lib.rs` reads them; `docs/usage.md` documents it; spec `accepted`.
- red: RVT-1, RVT-2, RVT-3, RVT-4, RVT-5, FEA-39: `cardinal: unknown tool `review-target`` (exit 3 where 0 was expected).
- Code review: 1 findings: the base was printed unquoted, so `@{u}` broke in a shell; fixed, quoted when it holds a
  character a shell reads (RVT-5, test `quotes_a_base_a_shell_would_expand`).
- not verified: what a reviewer does with the line (deliverable 2, measured by `cardinal-report` after use).
- Follow-up for deliverable 2: `git diff HEAD` leaves the untracked files out, so step 3.5 marks the new files
  intent-to-add (`git add -N`) before the `low` review.

## Deliverable 2
- done: step 3.5 marks the new files intent-to-add and hands `/code-review low` the line of `review-target`; the
  whole-issue review hands `/code-review high` the line of `review-target <base>`; `docs/workflow.md` says so; synced.
- red: none (a skill text, checked by `just check-cardinal`).
- Code review: no findings
- not verified: the fall of `oversized-result-review`, read by `skill:evaluate-harness` once sessions use it.

## High review (whole issue)
- Code review: 8 findings: 6 fixed in deliverable 3 (zsh `extendedglob` on `^`/`~`, untracked files unreviewed but
  recorded, uncommitted docs edits out of `<base>..HEAD`, the lock's hash lines, glob in managed paths, unborn `HEAD`);
  2 left as non-goals of the spec (the lock read from the work tree, not the revisions; an edit of a managed copy in a
  consumer, which `cardinal check` reports).

## Deliverable 3
- done: the fixes above; spec RVT-1 to RVT-6 rewritten; skill step 3.5 and `docs/usage.md` follow.
- red: RVT-1, RVT-2, RVT-3, RVT-4, RVT-5, RVT-6, FEA-39: 9 of 11 tests failed on the old output (`HEAD..HEAD` unquoted,
  no `literal`, no refusal of untracked files).
- Code review: no findings
