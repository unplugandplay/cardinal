---
priority: medium
why: 'fixes_after_per_issue feeds the quality verdicts, and its noise hides any effect'
---
# Count only the fixes of the lines an issue changed

# Goal
`delivered-quality` counts in `fixes_after` every `fix` commit of the next 30 days that touches any file of the issue
(QUA-4). In a repository whose issues share a few large files, each fix lands on many issues, so
`fixes_after_per_issue` reads noise: no trend can show through it, and an `Expected-effect: fixes_after_per_issue`
gets no usable verdict. The outcome: a fix counts against an issue only when it plausibly fixes what the issue
delivered.

# Findings
* Measured 2026-11-03 over three repositories with `issues/done/`: 1,567 `fixes_after` for 154 issues closed by the
  main setup, 10.18 per issue.
* Per group (last 17 with issues): from 4.45 to 29.36 per issue, swinging up and down with no two-group trend.
* `reverts_after` is 0 in every group.
* Measured 2026-11-03 on this repository (168 done issues, 30-day window, local `main`): the file-level count gives
  2,693 `fixes_after`, 16.0 per issue; counting a fix only when `git blame -M -C` of its first parent attributes a
  line it removes or changes to a commit of the issue gives 223, 1.33 per issue (89 issues with at least one);
  also counting, for a hunk that only adds lines, the line right above it gives 284, 1.69 per issue. The blame alone
  removes ~92% of the count: the file threshold and a shorter window are left out. (A script measured these, over
  every commit touching the issue file and the files of the issue only; the tool, with the commits of QUA-2 and the
  renames and moves followed, counts 380, 2.26 per issue.)

# Approach
1. Measure first: for a sample of issues of this repository, how many counted fixes change lines the issue added or
   changed (`git blame` of the fix's removed lines), against the file-level count.
2. Candidates, to combine:
   1. count a fix when a line it removes or changes was written by a commit of the issue;
   2. leave out the files most issues touch (a threshold on how many issues share the file);
   3. shorten the default window.
3. Amend QUA-4 and keep the old count as a column if the verdicts of past changes need it.

# Specs
* `specs/delivered-quality.md` QUA-4; `specs/cardinal-report.md` REP-6 if the column changes.

# Milestone
Lean agent loop

# Resources
* `specs/delivered-quality.md`, `crates/cardinal/src/tools/` (the `delivered-quality` tool).

# Hints
* Blame must run on the parent of the fix commit, and a renamed file must keep its history.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
A `fix` counts in `fixes_after` of an issue only when it changes a line a commit of the issue wrote (`git blame` of
its first parent, renames and moves followed), not any line of a file it touched; `reverts_after` and the window stay.
Out of scope: the file-sharing threshold and a shorter window (the measure makes them needless).

## Plan
1. [x] Count the fixes by blame: one blame per fix commit (its removed lines, all paths but `issues/`, `docs/` and
   the lock), cached and matched against the commits of each issue; spec QUA-4 amended; verified by behaviour tests
   of `crates/cardinal/tests/delivered_quality.rs` (two issues sharing a file, a renamed file, a pure insertion,
   the window); makes true QUA-4. (autonomous)
2. [x] Fixes of the `high` review of the issue: the paths of a fix read from `--name-status -z`, not from the diff
   headers (`diff.noprefix`, quoted names), blame run without the user's global configuration
   (`blame.ignoreRevsFile`), SHA-256 hashes read, the fixes of unrelated paths only skipped before any diff; verified
   by `blame_checks`; makes true QUA-4. (autonomous)

## Deliverable 1
- done: `fixes_after` counts a fix only when `git blame -M -C` of its first parent gives one of the lines it removes
  or changes (or the line right above a pure insertion) to a commit of the issue; on this repository, 380 fixes for
  168 issues (2.26 per issue) against 2,693 (16.0) by file; run time 8.0 s to 9.3 s.
- red: QUA-4: `alpha.fixes_after: expected "2", got Some("1")`, `beta.fixes_after: expected "1", got Some("3")`
- Code review: 1 finding: a removed line starting with `-- ` read as a file header (`--- `) dropped the later hunks
  of its file; fixed (a header only between `diff --git` and the first hunk), proven by a `-- x0` line in the fixture.
- not verified: the other repositories of the Findings (only this one was rerun).

## Decisions
- Mode: autonomous
- Base commit: 555b84dc768c5397f015d7e753dda935474719be
- A fix that only adds lines counts when the line right above its hunk was written by the issue (1 a).
- The former file-level count is not kept as a column (2 a).

## Deliverable 2
- done: the 9 findings of the `high` review of `555b84d..e811eb2` settled below; same count on this repository
  (380), 9.4 s.
- red: QUA-4: with `diff.noprefix` and a missing `blame.ignoreRevsFile` in the global configuration, and a quoted
  file name, `alpha.fixes_after: expected "2", got Some("0")`, `beta.fixes_after: expected "1", got Some("0")`
- High review: 9 findings: 1 `noprefix`, 2 `ignoreRevsFile`, 3 SHA-256, 4 quoted names, 6 duplicated header parser,
  7 path kept across sections: fixed by reading the paths from `--name-status -z` and blaming without the global
  configuration; 5 cost: the fixes of unrelated paths only are skipped, no parallel blame (9.4 s against 8.0 s
  before the issue); 8 a fix moving a file into `docs/` is read by its old path, kept (blame reads the old path);
  9 the two numbers of the Findings explained there.
- Code review: no findings
- not verified: a repository whose own configuration sets `blame.ignoreRevsFile` (kept: the repository then holds
  the file); a SHA-256 repository (no test).
