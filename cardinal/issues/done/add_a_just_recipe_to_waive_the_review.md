# Add a `just` recipe for the user to waive the review of the next commit

# Goal
The user waives the review of the next commit with one short recipe (`just waive-review`) instead of typing
`touch "$(git rev-parse --git-path claude-review-waived)"`, from the main repository or from a linked worktree.
The agent must still not be able to waive its own review.

# Findings
* The only documented way to waive is the long `touch "$(git rev-parse --git-path claude-review-waived)"` command
  (`issue-workflow/SKILL.md`, `docs/architecture.md`); the user has to remember it and the file name.
* `require_code_review.rs` refuses a Bash command that writes the waiver file (REV-13) by reading its text. A recipe
  run as `just waive-review` does not contain the file name, so the deterrent would not see it: whether the agent may
  run the recipe is not decided yet.
* REV-13 also refuses an interpreter (`python3`, a shell) whose script text merely names the waiver file, because the
  hook cannot tell what the script does. While editing `specs/review-gate.md` with a `python3` heredoc, the agent was
  refused though it did not write the waiver, and went on with the `Edit` tool, which the hook does not watch. In this
  repository the waiver is the subject of specs, tests and docs, so the false positive will recur.

# Approach
1. Add a recipe to `harness.just` (managed content, so under `files/`) that creates the waiver at the path
   `git rev-parse --git-path` gives for the current directory, and prints that path.
2. Decide how the deterrent treats the recipe, and propose rather than assume: refuse a command whose text runs the
   recipe (same family of checks as REV-13), or accept that the recipe is a deliberate user entry point and only
   document that the agent must not run it.
3. Point `issue-workflow/SKILL.md` and `docs/architecture.md` to the recipe instead of the `touch` command.
4. Reduce the false positive of REV-13 on interpreters, measuring first which commands it refuses today. Candidates
   (propose, the choice is made at the spec step): refuse an interpreter only when its text writes the file (a
   redirection, `touch`, `open(..., 'w')`-like calls are out of reach, so this stays a heuristic); or stop refusing
   interpreters and state in the spec that the deterrent covers shell-level writers only, the native hook and the
   user's waiver remaining the real barrier.

# Specs
* `specs/review-gate.md`: REV-17 (the recipe) and REV-18 (refusal of a command running it), drafted; plus an amendment
  of REV-13 for step 4, to be written once its approach is chosen.

# Resources
* `files/harness.just`, `files/.claude/hooks/require_code_review.rs` (`writes_waiver`), `files/.claude/hooks/test_require_code_review.rs`
* `specs/review-gate.md` (REV-11, REV-13 to REV-15), `docs/architecture.md`, `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)
* Done issues `make_the_review_waiver_usable` and `keep_the_review_waiver_until_the_commit_happens`

# Hints
* The hook refuses any Bash command that writes the waiver file: write the tests and this work without that literal
  in a shell command.
* The waiver is per worktree (REV-11) and stamped with `HEAD` once a commit goes through: the recipe only creates it.

# Tracking

1. [x] The hook stops refusing a shell, `eval`, `source` or an interpreter that only names the waiver; the writers stay refused (REV-13, REV-14).
2. [x] The recipe `waive-review` in `files/harness.just` creates the waiver at the `git rev-parse --git-path` path and prints it (REV-17).
3. [x] The hook refuses a command running the recipe; `SKILL.md` and `docs/architecture.md` point to it; spec `accepted`, then `implemented` (REV-18).

## Decisions

* Mode: autonomous (local commits after the review, no push).
* REV-13: interpreters are no longer refused for naming the waiver; the deterrent covers the writers only.
* The agent may not run the recipe: the hook refuses it (REV-18).

## Deliverable 1: summary

* Done: `WRITERS` of `require_code_review.rs` shrinks to `touch`, `tee`, `dd`, `truncate`, `curl`; a shell, `eval`, `source` or an interpreter that only names the waiver is no longer refused (REV-13, REV-14). Tests moved: the shell cases now sit with the mentions that pass, plus `python3`, `perl`, `eval` cases and a `curl -o` refusal. Hook header and `docs/architecture.md` updated; `just sync` done.
* Verified: `just check`, `just test-harness`, `just check-harness` green.
* Not verified: a live session.
* Accepted consequence: `sh -c 'touch <waiver>'` and a script that writes the file now pass (stated in REV-13).
* Code review: 6 findings. Fixed: the comment on heredoc bodies no longer claims they are never code; the list of refused writers in `docs/architecture.md` is complete; a missing space in a test tuple. Not fixed, by decision: shells, `eval` and interpreters passing (the user chose it; REV-13 states that the deterrent covers the writers only, the native hook and the user's waiver being the barrier); the spec staying `draft` with REV-17 and REV-18 untraced (their deliverables are 2 and 3); `curl` kept in the list (it is a writer, `wget` and the like are left to the deterrent's known limits).

## Deliverable 2: summary

* Done: `waive-review` in `files/harness.just` creates the waiver at the absolute `git rev-parse --git-path` path of the worktree it runs in (also from a subdirectory) and prints it (REV-17). Test `recipe_checks` runs it in a main and a linked worktree: path printed, nothing else changed in the repository, the other worktree untouched, the hook honours the waiver. The docs (`SKILL.md`, `docs/architecture.md`) still give the `touch` command: they move to the recipe in deliverable 3, with its refusal.
* Verified: `just check`, `just test-harness`, `just check-harness` green.
* Not verified: a live session.
* Code review: 7 findings. Fixed: the summary no longer points to a missing review. Accepted: until deliverable 3 the hook does not refuse `just waive-review` (the next deliverable, local commits only, no push in between); the test needs `just` and git 2.31+ (the project already requires both for its recipes); a spent waiver is consumed by the hook (stamp on `HEAD`), contrary to one finding. Not fixed: temp directory cleanup on a panic and a future `import` in `harness.just` (same pattern as the other checks, speculative).

## Deliverable 3: summary

* Done: `runs_waive_recipe` in `require_code_review.rs` makes `check` refuse a command where `just` stands as a word the shell runs and `waive-review` (quoted or not) follows, with any options, `sudo`, `env` or a path in between (REV-18); a `grep`, a commit message or a heredoc that only names the recipe passes. `SKILL.md`, `docs/architecture.md` and the hook header point to `just waive-review`; the spec is `implemented`. Tests are in `waiver_write_checks` and `waiver_mention_checks`.
* Verified: `just check`, `just test-harness`, `just check-harness` green.
* Known limit: a quoted `"just"`, a name built from quote pieces and an alias or a variable are not seen; a deterrent, like REV-13.
* Not verified: a live session.
* Code review: 6 findings. Fixed: `just` in braces, parentheses or a substitution was missed (words are now split like `writes_waiver`); `just --show/--dry-run/--summary/--list/--dump waive-review` was refused though it runs nothing (REV-18 amended to say so); `echo just waive-review` was refused (`just` must now be the command word, after wrappers, assignments and options); tests for each; a missing space in a test tuple. Accepted: name built from quote pieces, alias or variable (deterrent); long lines of `docs/architecture.md` (as the rest of the file).
