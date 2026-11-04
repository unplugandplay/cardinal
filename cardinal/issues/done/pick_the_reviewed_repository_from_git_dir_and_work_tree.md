# Pick the reviewed repository from --git-dir and --work-tree

# Goal
`require_code_review.rs` must check the repository a `git commit` really targets, so that
`git --git-dir /other/.git --work-tree /other commit` cannot pass on the review of the current directory.

# Findings
* `commit_dir` now skips the value of `--git-dir` and `--work-tree` (two words or `--opt=value`) but only `-C` sets
  the directory. The commit is checked against the repository of the shell directory.
* If that repository was reviewed and is unchanged, a commit in another repository passes unreviewed.
* Found by the review of `match_git_value_options_in_the_review_hook`.

# Approach
1. Add failing cases to `test_require_code_review.rs`: a commit given `--git-dir`/`--work-tree` of an unreviewed
   repository while the current one is reviewed.
2. Make `commit_dir` return the work tree (or the parent of the git directory) named by these options, as it does for `-C`.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`, `toplevel`), `files/.claude/hooks/test_require_code_review.rs`

# Hints
* `GIT_DIR` and `GIT_WORK_TREE` set in the environment of the command are not seen either; decide whether they are in scope.

# Specs
* `specs/review-gate.md` (`REV-1` to `REV-6`).

# Tracking
1. [x] Target `--git-dir` and `--work-tree` (REV-1, REV-3, REV-4): tests per requirement, failing on the old `commit_dir`
2. [x] Relative paths and accumulated `-C` (REV-2, REV-5); trace every requirement, spec `accepted` then `implemented`
3. [x] Docs (`update-docs`) and follow-up issue for `GIT_DIR` / `GIT_WORK_TREE`
4. [x] `--git-dir` alone also checks the shell directory (REV-6), and a git directory inside `.git` (REV-3), decided by the user after the first review

## Decisions
* Mode: autonomous.
* `GIT_DIR` and `GIT_WORK_TREE` of the environment are out of scope (a variable exported earlier is invisible to the hook, a partial cover would give false confidence): non-goal of the spec, follow-up issue.
* A target that is not a repository is let through, as with `-C`.
* `--git-dir` without `--work-tree`: the commit passes only when both the shell directory and the repository of the git directory are reviewed (user decision after the review; REV-6). A git directory is resolved to the parent of its first `.git` component.
* The code and one doc sentence were written before the spec; they are kept and the tests are brought in line with the spec.

## Summary
* Done: `commit_dirs` (was `commit_dir`) returns the repositories whose review a commit depends on: the work tree of `--work-tree`, else the parent of the innermost `.git` of `--git-dir` (the directory itself without one) plus the directory the command runs in, else the directory the `-C` options lead to. The `-C` accumulate; a relative `--work-tree`/`--git-dir` is taken from that final directory wherever the `-C` stands (checked against the real tool); two-word, `=value` and quoted values, also with spaces. Each repository is checked once. Spec `specs/review-gate.md` (REV-1 to REV-6, `implemented`), one test function per requirement with `// spec: REV-n`; the cases that depend on each change failed first. Docs: `docs/architecture.md`, `CLAUDE.md`. Follow-up issue `read_git_dir_and_git_work_tree_in_the_review_hook`.
* Deliverables 1 and 2 were done together (all tests written at once); 4 came from the user's decision on the first review.
* Verified: `just check`, `just test-harness`, `just check-harness`. Not verified: a real commit through the hook outside the test fixtures.
* Process: the code and a doc sentence were written before the spec and the review; kept, then the tests were brought in line with the spec.
* Code review, round 1 (7 findings): fixed the relative paths taken from the directory before the `-C` instead of the final one (real bug, REV-5 was wrong), the missing positive tests, long comment lines, the stale tracking; no change for a `-C` without value (only possible at the end of the line, where no commit is seen) and the `--git-dir=` arms next to the generic one (list shared with the attribution hook). The open finding (`--git-dir` alone, a git directory inside `.git`) was decided by the user: REV-6 and REV-3 amended, fixed.
* Code review, round 2 (8 findings): fixed the quoted `=value` spanning several words (the commit was let through; tests first), the empty path of a bare `--git-dir .git`, the repository checked twice, the "first" vs "innermost" `.git` wording of spec and docs, the misplaced paragraph of `docs/architecture.md`, the long line of `CLAUDE.md`, this summary. No change: a missing option value (same reason as above); the waiver stamped on the first repository when the second refuses (the file stays usable, the commit does not happen).
