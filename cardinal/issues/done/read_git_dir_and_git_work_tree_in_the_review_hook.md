# Read GIT_DIR and GIT_WORK_TREE in the review hook

# Goal
`require_code_review.rs` must not let `GIT_DIR=/other/.git GIT_WORK_TREE=/other git commit` pass on the review of the
shell directory, or the spec must say plainly why it cannot, and the hook must then fail closed where it can.

# Findings
* The hook reads the text of the command only. `--git-dir`, `--work-tree` and `-C` pick the repository
  (`REV-1` to `REV-5` of `specs/review-gate.md`), the two environment variables do not.
* They are a non-goal of the spec today, because a variable exported earlier in the shell is invisible to the hook.
* A variable written in front of the command (`GIT_DIR=x git commit`, `env GIT_DIR=x git commit`) is visible in
  the text, so a part of the cover is possible.
* Found while working `pick_the_reviewed_repository_from_git_dir_and_work_tree`.

# Approach
1. Add failing cases to `test_require_code_review.rs` for a commit with the variables in front of `git`, with
   `env`, and with `export` in the same line.
2. Candidate solutions, can be combined:
   * read the assignments that precede `git` in the same command and use them as `--git-dir`/`--work-tree`;
   * refuse (fail closed) a commit line that sets one of these variables, since the target cannot be trusted;
   * keep the non-goal and document the limit, if the partial cover is judged to give false confidence.
3. Decide in `issue-workflow` steps 1 to 3, then amend the spec (a new requirement, or the non-goal kept).

# Specs
* `specs/review-gate.md`: a new requirement `REV-7` (`REV-6` is taken), the non-goal narrowed.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`), `files/.claude/hooks/test_require_code_review.rs`
* `specs/review-gate.md`, `docs/architecture.md` (the review gate)

# Hints
* A variable exported by an earlier command of the session is not in the text of the one the hook sees: only
  failing closed on a suspicious environment could cover it, and the hook cannot read the shell's environment.

# Tracking

1. [x] Read the assignments before `git` (`GIT_DIR=x`, `env GIT_DIR=x`, `GIT_WORK_TREE`), options winning (`REV-7`)
   Done: `commit_dirs` reads the assignments (and `env` options) right before `git`; 7 refusals and 2 passes in
   `environment_checks`. Verified: `just check`, `just test-harness`. Not verified: `export` and undeterminable values
   (deliverables 2 and 3). Code review: fixed the reset on any word between the assignment and `git` (`sudo`, `nohup`
   bypassed it; only `;` resets now, tests added), the blank line and scenario order in the spec, a long comment line;
   left to deliverables 2 and 3: `export`, `$X` and `$(…)` (the `(` is split into `;` before the scan, so the
   undeterminable value must be detected on the raw command); not fixed: `env -C dir` / `--chdir` are ignored (rare, noted).
2. [x] Read `export` in the same line; `unset` and `env -u` not refused (`REV-7`)
   Done: `environment_repository` reads the environment of the command: prefix assignments, `export`/`declare`/`typeset`
   until an `unset`, a plain `VAR=x;` exported by a later `export NAME`, `env -u NAME`; a newline separates commands.
   Verified: `just check`, `just test-harness` (6 refusals, 4 passes). Not verified: I wrote the code before running the
   new tests, so I did not see them fail first. Code review: fixed valueless `export NAME` and `declare -x` (bypass),
   `env -u`, newline as separator, the helper extracted from `commit_dirs`; not fixed: `export` inside a subshell
   `( … )` counts as global (over-refuses), `unset -f`, `env -C` (rare, noted). Review 2: fixed the `\` + newline
   continuation (a bypass that predated this issue) and the assignment to an already exported variable; not fixed
   (over-refusals or exotic, noted): `echo export …` read as an export, `declare` without `-x`, `env -i`, `--unset`,
   a `-u` of another command, the state machine kept as arrays instead of a struct.
3. [x] Refuse a value that cannot be determined (`$X`, `$(…)`, backticks) (`REV-7`)
   Done: `undeterminable_variable` reads the raw command (brackets are still there) and refuses a commit line whose
   `GIT_DIR=` / `GIT_WORK_TREE=` value holds `$` or a backtick, outside single quotes; `MY_GIT_DIR=` and `unset` are
   not concerned. Tests written first and seen failing (5 refusals), 4 passes. Verified: `just check`,
   `just test-harness`. Not verified: values inside a `--git-dir=$X` option or `-C $X` (not in the spec).
   Code review: fixed `"lit"$X` (a bypass: the value is now scanned with quotes and escapes), and literal
   `'b$c'` / `\$b` no longer refused; not fixed: `--git-dir=$X`, `-C $X`, fish `set -x` (outside REV-7, to file as an
   issue), over-refusal of `echo GIT_DIR=$X` or a message quoting it (fail closed, the user can waive), two parsers
   kept apart.
4. [x] Docs (architecture, hook doc, this issue), spec `accepted` then `implemented`, `just sync`
   Done: `docs/architecture.md` (review gate), the `//!` of the hook (deliverable 3), `REV` set to `implemented` (the
   seven requirements are traced; it never stood at `accepted` since its tests came with the code). Verified:
   `just check-specs`, `just check`, `just test-harness`. Left over: `--git-dir=$X` / `-C $X` / fish `set -x` are not
   covered, to file as a separate issue if wanted. Code review: the docs now name `env -u`, backticks and the
   cross-call `export` as a known limit; the `draft` to `implemented` jump is kept (the user validated the requirements,
   all are traced, `implemented` is enforced like `accepted`).

## Decisions

* Mode: autonomous.
* Approach: read the variables of the line and refuse an undeterminable value; the non-goal stays for variables exported by an earlier command.
* `unset` / `env -u` are not refused; an undeterminable value is refused unconditionally.
* The new requirement is `REV-7` (`REV-6` was already taken).
