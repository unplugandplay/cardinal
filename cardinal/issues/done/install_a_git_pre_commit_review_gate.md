# Install a native git pre-commit hook as the last-resort review gate

# Goal
`harness init` and `harness sync` must install in each consumer a native git `pre-commit` hook that refuses a commit
whose repository holds a change the last code review never saw. Git runs it with `GIT_DIR` and the work tree already
resolved, so the gate no longer depends on reading the repository out of the text of the command, which
`require_code_review.rs` can only approximate.

# Findings
* `require_code_review.rs` parses the command line to find the repository a `git commit` lands in (`-C`, `--git-dir`,
  `--work-tree`, `GIT_DIR=`, `export`). Every new way to name a repository (`$X`, `$(…)`, fish `set -x`) is a bypass
  until the parser learns it.
* A `pre-commit` hook runs in the repository that really receives the commit, whatever the command looked like.
* It does not replace the Claude hook: `--no-verify` and `core.hooksPath` skip a native hook, and only the Claude hook
  can refuse them.
* Follow-up of `read_unreadable_repository_options_in_the_review_hook`, where validating the commit after its creation
  and rolling back was considered and rejected (the post-commit hook cannot tell which repository to reset).

# Approach
1. Decide how the hook reads the review marker of its repository (the same marker and snapshot as
   `require_code_review.rs`, shared code or a single binary entry such as `harness check-review`).
2. Install it from `harness init` / `harness sync`: a managed file under the git hooks directory, chained with a
   `pre-commit` the user already has (never overwritten silently), and re-installed when it drifts.
3. Cover the repositories reached through `--git-dir` and linked worktrees (hooks directory of the common git dir).
4. Make the Claude hook refuse `--no-verify`, `-n` and a `core.hooksPath` change on a `git commit` line.
5. Write an ADR: the gate moves from parsing the command to the repository, with the bypasses that remain.

# Specs
* `specs/review-gate.md`: new requirements for the native hook; `specs/sync.md` for its installation.

# Resources
* `files/.claude/hooks/require_code_review.rs`, `specs/review-gate.md`, `specs/sync.md`
* `crates/harness/` (`init`, `sync`, `check`), `docs/architecture.md` (the review gate), `docs/adr/README.md`

# Hints
* `.git/hooks` is not versioned and not copied by `git clone`: the consumer needs `harness sync` to install it.
* A consumer may set `core.hooksPath` itself; say what the tool does then instead of ignoring it.

# Tracking

1. [x] `native` mode of `require_code_review.rs` (REV-9, REV-10, REV-11). Done: tests (direct runs, a real `git commit` through an installed `pre-commit`, a linked worktree), `GIT_INDEX_FILE` ignored so `commit -a` sees the real index, `just check` / `test-harness` / `check-harness` green. Not verified: a `--git-dir` commit from a non-trivial cwd through real git (covered by `GIT_DIR` env runs only). Code review: see the commit.
2. [x] The Claude hook refuses `--no-verify`, `-n` and `core.hooksPath` on a `git commit` line (REV-12). Done: `skips_verification` (long abbreviations, bundles, option values skipped, `--`), `skips_hooks` (every commit of the line, each command read apart) and `sets_hooks_path` (per command: `-c`, `--config-env`, `git config key value`, `GIT_CONFIG_KEY_n`, `GIT_CONFIG_PARAMETERS`; `--get`, `--unset`, a bare read and message words excepted); 36 cases in the test. Code review: 8 findings, fixed the `--get` scope, the later commits of a line, the displaced doc comment, `--trailer`, the false positives (unset, read, heredoc body) and the missing cases; the other ways of moving the hooks stay open (below). Not covered: `git config` through an alias or a shell variable, `GIT_CONFIG_GLOBAL` pointing at another file, `git -c include.path=…`, editing `.git/config` with a file tool; to cite in the ADR.
3. [x] `sync` installs the stub, chains a foreign hook, reinstalls on drift (SYN-18, SYN-19, SYN-20). Done: `crates/harness/src/gate.rs`, `Outcome::Moved`, 8 tests (the script is run with a fake `rust-script`: exit code, arguments, warning without review hook or `rust-script`, chaining order, `--dry-run`, idempotence); this repository now carries the gate (`just sync`). `init` alone does not install it (the review hook is not there yet): SYN-18 says so. Code review: 9 findings; fixed the mark read on the second line only, the install order (new hook staged first, gate installed before the managed files), a relative symlink moved with the user's hook, the git environment of the subprocess, `$0` without a slash and a loose test assertion; `core.hooksPath` (SYN-22) and the module claims are item 4. Not verified: Windows (the hook is `sh`).
4. [x] Linked worktrees, `core.hooksPath` set by the consumer, project without git (SYN-21, SYN-22). Done: `State::Redirected` / `Outcome::Skipped` (message names the line to add to the user's own `pre-commit`), 3 tests (linked worktree, `core.hooksPath`, no repository / nested repository / no review hook). Not verified: a `core.hooksPath` set only in the global git config on this machine (the check uses `git config --get`, which reads every level, but the test sets it locally). Code review: 6 findings; fixed the line shown to the user (now with `rust-script`), an unreadable config (treated as set: no gate rather than one in the wrong directory) and a relative-path test; not fixed: a gate installed before `core.hooksPath` was set stays in `.git/hooks` and is neither checked nor updated (`sync` only says it installs nothing; to mention in the ADR), the number of `git` processes (four per run, negligible), `Outcome::Skipped` carrying prose (one skip reason today).
5. [x] `check` reports a missing or modified native gate (SYN-23). Done: `gate::problem`, one test over missing / edited / not executable / foreign hook / `core.hooksPath` / no review hook / no repository. SYN-23 amended during the review: `check` skips the gate when `CI` is set (a fresh clone of a CI run has no git hooks, so the shared workflow would be red for ever) and expects it from the managed files of the lock, not from the file on disk. Code review: 7 findings; fixed the CI failure, the deleted review hook, the duplicated decision (`inspect`, shared by `sync` and `check`) and the doc; not fixed: a gate left in `.git/hooks` after `core.hooksPath` was set is not checked (same gap as item 4), four `git` processes per `check` (negligible), the long sequential test. Not verified: a CI that does not set `CI`. Second review (after the SYN-23 amendment): fixed the tests that called the public `check` (red under `CI=true`, now `check_with(…, false)`; `just check` run with and without `CI=true`), `CI=`/`0`/`false` no longer count as CI and the help text; not fixed: the ambient variable instead of a flag (deterministic enough, a flag can come later), the hint that `sync` fixes a foreign hook, the long test.
6. [x] ADR 0004, docs, `just sync`, specs `accepted` then `implemented`. Done: ADR 0004 (decision, remaining bypasses, the `CI` exception), `specs/review-gate.md` and `specs/sync.md` set to `implemented` with `adrs` updated (every deterministic requirement is traced, `just check-specs` clean), architecture links the ADR. `just check`, `just test-harness`, `just check-harness` green, also with `CI=true`. Checked by hand in a scratch consumer (`init` + `sync` from this repository, a user `pre-commit` already there): the hook was moved to `pre-commit.user`, the gate installed, a real commit refused with "no code review has run", `harness check` clean. Not verified: Windows (stated in the ADR), a commit accepted through the native hook after a review in that scratch consumer (covered by the unit-level test with a real `git commit`). Code review: 4 findings, all documentation (Windows and the status of REV-1 to REV-8 added to the ADR; the ADR is added to the commit; the manual check above).

## Decisions

* Mode: autonomous (commit locally per deliverable, no push, review never skipped).
* The stub runs `rust-script .claude/hooks/require_code_review.rs native`: one code for the marker and the snapshot.
* `core.hooksPath` set by the consumer: install nothing, say so, `check` stays silent about it.
* A foreign `pre-commit` moves to `pre-commit.user` and is chained after the gate; never overwritten.
* Stub with no review hook or no `rust-script`: warn on stderr, let the commit through.
