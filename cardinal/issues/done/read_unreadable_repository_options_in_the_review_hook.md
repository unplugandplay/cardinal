# Refuse unreadable repository values in the review hook

# Goal
`require_code_review.rs` must not let a commit pass on the review of the shell directory when the repository it lands in
is named by a value the text of the command does not show: `--git-dir=$X`, `--work-tree=$X`, `-C "$(mktemp -d)"`, or a
fish `set -x GIT_DIR $X`. The hook must refuse such a line (fail closed), as `REV-7` does for `GIT_DIR=$X`.

# Findings
* `REV-7` refuses an unreadable value of `GIT_DIR` and `GIT_WORK_TREE` assigned in the line
  (`undeterminable_variable`). The options `--git-dir`, `--work-tree` and `-C` are read by `commit_dirs`, which takes
  a literal `$X` as a relative path: the target is then a missing directory and the commit passes, as a target that
  is not a repository does.
* `set -x GIT_DIR $X` (fish, the shell of the user) sets the environment without a `NAME=value` word, so neither the
  environment reader nor the unreadable-value check sees it.
* A variable exported by an earlier command of the session stays out of reach (non-goal of the spec).
* Found by the code review of the last deliverable of `read_git_dir_and_git_work_tree_in_the_review_hook`.

# Approach
1. Add failing cases to `test_require_code_review.rs`: each form above with every repository reviewed, expecting exit 2;
   and literal values (`--git-dir='$X'`, a path without `$`) still passing.
2. Candidate solutions, can be combined:
   * apply the check of `undeterminable_variable` (quotes, escapes, `$`, backticks) to the value of the three options;
   * read the fish `set -x` / `set --export` forms into `environment_repository`, with `set -e` as the `unset`;
   * keep one tokenizer for the assignments and the options, since two parsers can disagree.
3. Decide in `issue-workflow` steps 1 to 3, then amend the spec (a requirement `REV-8`, or `REV-7` widened).

# Specs
* `specs/review-gate.md`: a new requirement `REV-8`, or `REV-7` widened; the non-goal stays for earlier commands.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dirs`, `environment_repository`, `undeterminable_variable`),
  `files/.claude/hooks/test_require_code_review.rs`
* `specs/review-gate.md`, `docs/architecture.md` (the review gate)

# Hints
* The hook must not refuse `git commit -m "$msg"`: only values that name the repository count.
* Editing a hook test with `sed` or a heredoc that holds both `git commit` and `GIT_DIR=$X` is itself refused by the
  installed hook: write the test with the editing tools.

# Tracking

1. [x] Refuse an unreadable value of `-C`, `--git-dir` and `--work-tree` (REV-8, options). Verified: `just check`, `just test-harness`, `just check-harness`; not verified: the fish forms (deliverable 2, so the spec stays `draft`). Code review: the fish half of REV-8 is deliverable 2 (not fixed here); a `commit` word after a substitution also refuses a line that does not commit (accepted, fail closed; this very note was refused while it was written in a shell command); the duplicated refusal message was merged, the redundant `unwrap_or_default` removed, the doc rewrapped.
2. [x] Read the fish `set -x` / `set --export` / `set -e` forms of `GIT_DIR` and `GIT_WORK_TREE`. Verified: `just check`, `just test-harness`, `just check-harness`; REV-8 traced and the spec set to `implemented`; not verified: fish forms beyond `set` with flags, a name and a first value (value lists, `set -U` persistence). Code review: no bug confirmed; the false positives suspected after a literal value are covered by new tests (the value stops at the first separator); a second raw-text scanner for `set` next to the word reader is kept, as `undeterminable_variable` already is; doc rewrapped (REV-8, fish)

## Decisions

* Mode: autonomous.
* New requirement `REV-8` rather than a widened `REV-7`.
* A fish `set` with a readable value is read as `NAME=value`; a `$` in a `-C` value refuses the line.
* The native git `pre-commit` hook is a separate issue: `install_a_git_pre_commit_review_gate`.
