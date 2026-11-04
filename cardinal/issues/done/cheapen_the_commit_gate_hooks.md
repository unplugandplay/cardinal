# Make the commit gate hooks cheap on commands that are not commits

# Goal
The two `PreToolUse Bash` hooks run on every shell command of the agent. They must cost almost nothing when the
command is not a commit, and a commit gate must not read large untracked files.

# Findings (from a code review, to confirm by measure)
* `require_code_review.rs` `main` runs `git rev-parse --show-toplevel` before knowing that the command is a commit;
  the call is only needed after `commit_dir` returns `Some`.
* Each hook is a separate `rust-script` start: two per Bash call.
* `snapshot` reads the whole content of every untracked file and pipes it to `git hash-object`, on each commit
  attempt: slow and memory heavy with large untracked files that are not ignored.

# Approach
1. Measure the added latency of a plain `ls` with and without the hooks, and a commit with a large untracked file.
2. Candidates: test `commit_dir` first; hash untracked files by `git hash-object --stdin-paths`, or by path, size
   and mtime; one script for both hooks if the cost is the start.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`main`, `snapshot`), `files/.claude/hooks/block_ai_attribution.rs`

# Hints
* The marker must still change when an untracked file changes content.

# Tracking
1. [x] Measure the latency of a plain `ls` (no hook, each hook, both) and of a commit with a large untracked file; decide on deliverable 4
2. [x] Test `commit_dirs` before `toplevel` in `main` (a non-commit command runs no `git`)
3. [x] Hash untracked files without loading them (`git hash-object --stdin-paths`); the marker still changes with the content
4. [x] (dropped, see Decisions) Merge the two hooks into one script, only if the measure shows the start dominates (else tick as dropped with the figures)

## Decisions
* Mode: autonomous.
* No spec: the issue changes no behaviour of `specs/review-gate.md`.
* The hash of the marker may change value (a review in progress is redone once); its meaning stays.
* Measure (2026-10-25, hooks already compiled in the `rust-script` cache, plain `ls`, mean of 20 runs): `require_code_review.rs check` 4.1 ms,
  `block_ai_attribution.rs` 2.5 ms (a bare fork is 0.07 ms), so 6.7 ms per Bash call; a single `git rev-parse --show-toplevel` is about 1 ms.
  `check` on `git commit` with one 1 GB untracked file: 2.65 s and a 1.9 GB peak memory. Command: pipe a `PreToolUse` event
  (`{"tool_name":"Bash","cwd":…,"tool_input":{"command":…}}`) into `rust-script files/.claude/hooks/<hook>.rs check`.
* Deliverable 4 dropped: merging would save about 2.5 ms per Bash call, which is not worth a coupling of two hooks, a change of the
  declaration and of `harness sync`. The real costs are the early `toplevel` (about 1 ms of the 4.1) and the memory of `snapshot`.
* Merging `block_ai_attribution` into `require_code_review` is accepted if the measure justifies it.
* Deliverable 2: `toplevel` is now asked in `record` (after the `code-review` skill) and in `check` (per directory of a commit) only; `native`
  is unchanged. Test `cheap_checks` (fake `git` logging its calls). Verified: `just check`, `just test-harness`. Not measured again.
  Code review: 5 findings, all fixed (test fails when the hook fails to run, quoted log path, record path tested, redundant binding removed, directories deduplicated before `toplevel`).
* Deliverable 3: `snapshot` hashes the untracked files with `git hash-object --stdin-paths` (`untracked_hashes`); a name with a line break is
  hashed alone, a non-file (nested repository, dangling link) counts as `-`, a batch git cannot finish falls back to one call per file. Same
  1 GB untracked file, `check` on a commit: 1.36 s and 11 MB peak (was 2.65 s and 1.9 GB; the 1.36 s is git reading the file). Tests
  `streaming_checks` (nothing of the content goes through the hook) and `untracked_content_checks` (the marker follows a same-size content change).
  Verified: `just check`, `just test-harness`, `just check-harness`. Not verified: a file unreadable for permissions (falls back, untested).
  Code review: 7 findings. Fixed: `--no-filters` (the content is hashed raw, as before), no git process when nothing is hashable, nested
  repository and unreadable file now tested. Not fixed: one process per file when a batch fails (rare, only on an unreadable or vanished file);
  a symlink is hashed by its target as before (the old code had the same blind spot); the new format of the hash retires the pending
  markers once, accepted in the decisions above.
