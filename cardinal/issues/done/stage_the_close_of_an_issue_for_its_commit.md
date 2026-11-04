---
priority: medium
why: 'the commit of nearly every close fails once on the same pathspec error, a wasted call and a retry in each session'
---
# Stage the close of an issue for its commit

# Goal
After `close-issue`, the agent commits the close with `git add issues/<name>.md issues/done/<name>.md ...` and git
fails with `fatal: pathspec 'issues/<name>.md' did not match any files` (exit 128). The old path is gone from the work
tree and the index because `git mv` already staged the rename. The agent then retries without it. Expected: the
commit of a close succeeds on the first attempt.

# Findings
* `crates/cardinal/src/tools/close_issue.rs`: the move uses `git mv` (staged), but the documents `relink` rewrites
  are left unstaged. The output says `moved <old> -> <new> (git mv)` and `updated references in <file>`, without
  saying what is staged or which command to run.
* `files/.claude/skills/issue-workflow/SKILL.md`, Rules: "include the move and those edits in the final commit". This
  leads the agent to name both paths of the move.
* The transcripts of this machine (2026-11-02) hold the error `pathspec 'issues/<name>.md' did not match` in about 64
  sessions, the close of `name_the_chained_hook_conflict_in_check` included.

# Approach
1. Write a test of `close-issue` in a git repository that reproduces the error with the command the skill leads to.
2. Candidate fixes, which can be combined:
   1. `close-issue` also stages every file it rewrote (references, milestone state) and prints one line saying that the
      close is staged and the commit to run (`git commit -m <msg>`, no `git add`).
   2. `close-issue` prints the exact `git add` line of the files to add, without the old path.
   3. Reword the Rules of `issue-workflow` (and `details.md`, "Merge back"): the move is already staged, so add only
      what `close-issue` lists.
3. Check that the native review gate and `rule-breaches` (staging every file) still let the close through.

# Specs
* `specs/parallel-issues.md` or the spec of `close-issue` (find which one owns its output): a requirement on what
  `close-issue` stages and prints.

# Resources
* `crates/cardinal/src/tools/close_issue.rs`, `files/.claude/skills/issue-workflow/SKILL.md` (Rules),
  `files/.claude/skills/issue-workflow/details.md` (Merge back), `cardinal-issues.just`.

# Hints
* `git commit -- <old path>` may fail the same way: test the command that is finally recommended.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
`close-issue` stages what it rewrote (unless the file held unstaged changes of the user) and prints that the close is
staged and is committed with a bare `git commit`; the skill texts say the same. Out of scope: `--relink`, a commit
made by the tool itself.

## Plan
1. [x] Stage the close: `close-issue` stages its rewrites and prints the `staged:` / `not staged:` lines; the Rules of
   `issue-workflow` and its "Merge back" say to commit with `git commit`, no `git add` of the old path; verified by a
   test of `crates/cardinal/tests/close_issue.rs` that commits with no `git add` and finds a clean tree; makes true
   CLS-9. (autonomous)
2. [x] `high` review fixes: changes judged against `HEAD`, literal pathspecs, exit 1 when git refuses to stage, the
   files staged before named, one `ls-files` for every rewrite, the skill says to commit the close on its own;
   verified by the extended test of CLS-9; makes true CLS-9 (amended). (autonomous)

## Deliverable 1
- done: `close-issue` stages the moved issue whole and each clean tracked file it rewrote, names the others
  `not staged:`, prints the `staged:` line; `issue-workflow` (Rules, "Merge back") and `docs/workflow.md` say to commit
  with a bare `git commit`.
- red: CLS-9: `FAIL - a bare git commit holds the move and every rewrite of the close`, `FAIL - the close says it is
  staged and names the file of the user it left out`
- checks: `just check` ok; 212 changed lines, within.
- Code review: no findings
- not verified: a close that sets a milestone `done` is staged by the same path, not tested end to end.
- Decision: the moved issue is staged whole, its unstaged last tracking included (CLS-9 amended at step 3.2).

## Decisions
- Mode: autonomous
- Base commit: 4c2e92a
- A rewritten file holding unstaged changes of the user stays unstaged and is named (1 a).
- `--relink` stages nothing (2 a).

## Deliverable 2
- done: the fixes of the `high` review of the whole issue; CLS-9 amended (changes judged against `HEAD`, the
  `staged before the close:` line, exit 1), the Rules of `issue-workflow` say to commit the close on its own.
- red: none (the fixes of the `high` review; the glob case checked by mutation: without `--literal-pathspecs` the test
  fails).
- checks: `just check` ok.
- Code review: no findings (`low`, the fixes)
- Code review (`high`, whole issue): 10 findings
  1. user changes already staged: fixed (judged against `HEAD`, `staged before the close:` lines).
  2. the close commit apart from the code: fixed (the Rules say to commit the close on its own).
  3. names read as globs: fixed (`--literal-pathspecs`, tested).
  4. exit 0 when `git add` fails: fixed (exit 1, no `staged:` line).
  5. a failed `git diff` stages everything: fixed (every rewrite left unstaged).
  6. lossy names: fixed (`repo_files::path_of`, `OsStr` arguments).
  7. one `ls-files` per file: fixed (one call; `git add -u` refuses an untracked path, so the filter stays).
  8. a roadmap written twice: fixed (deduplicated).
  9. `transcripts::git_bytes` duplicated: not reused, it takes `&str` arguments and no literal pathspecs; the new
     helper is the one that needs both.
  10. untested paths: fixed for the milestone, a staged change and a glob name; the refusal of `git add` is not tested
      (no cheap way to make it fail after the `git mv`).
- not verified: the exit 1 when git refuses to stage.
