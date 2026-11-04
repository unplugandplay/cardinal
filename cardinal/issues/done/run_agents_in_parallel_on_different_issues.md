# Run two code agents in parallel on different issues without polluting each other's work

# Goal
Let the user run two agents at once, each driving its own issue with `skill:issue-workflow`, without one
agent's edits, build artifacts, commits or queue changes leaking into the other's work. The isolation must
rely on git (a worktree or a branch per issue) and the merge back must stay a deliberate, validated step.

# Findings
* `issue-workflow` and `just next-issue` assume a single agent on one checkout: both agents would get the same
  "next" issue from `issues/queue.md`, and `just close-issue` edits the shared queue file.
* Two agents in one working tree share uncommitted files, the `target/` directory and the git index, so a commit
  or a formatter run by one captures or rewrites the other's changes.
* No measure yet of how often queue edits conflict when two branches close or create issues.

# Approach
Decide at `issue-workflow` steps 1 to 3; the candidates can be combined.

1. One `git worktree` per issue, on a branch named after the issue stem: separate working tree and index,
   shared object store. Compare with plain branches in a single checkout, which do not isolate uncommitted work.
2. A recipe or skill step that creates the worktree and branch for a given issue and removes them once the
   branch is merged, so the isolation is not left to the user's memory.
3. Mark an issue as taken so `just next-issue` skips it for the second agent, and decide where that mark lives
   (queue line, a file per worktree, or the branch itself) given that worktrees do not share uncommitted files.
4. Avoid queue conflicts: decide whether closing an issue edits `issues/queue.md` on the issue branch (merge
   conflicts possible) or at merge time.
5. Per-worktree build state: separate `target/` directories versus a shared `CARGO_TARGET_DIR`, trading disk and
   compile time against lock contention.

# Specs
* `specs/parallel-issues.md` (`PAR`): isolation requirements for `start-issue`, `finish-issue`,
  `next-issue`, `close-issue` and the hooks.

# Resources
* `files/.claude/skills/issue-workflow/` (`next_issue.rs`, `close_issue.rs`, `SKILL.md`)
* `issues/queue.md`, `docs/architecture.md`, `.claude/harness.toml`
* `files/.claude/hooks/` (hooks run per working tree; check they behave inside a worktree)

# Hints
* `.claude/harness.lock` and `.claude/harness.toml` are tracked, so each worktree has its own copy: `just sync`
  in one must not be needed in the other.
* A branch cannot be checked out in two worktrees at once.
* The shared files hold nothing specific to one project.

# Tracking

1. [x] Hooks resolve the git directory per worktree (PAR-9): test in a temporary repo with two worktrees, fix what fails.
   Done: `require_code_review.rs` already used `git rev-parse --git-path`, so no code change; the test
   `worktree_checks` in `test_require_code_review.rs` (`// spec: PAR-9`) proves a review and a waiver are per
   worktree. Verified: `just check`, `just test-harness`, `just check-harness`. The other hooks do not use the git
   directory.
   Code review: 6 findings. Fixed: the fixture now lives in its own temp directory, the main git directory is
   resolved like the linked one, the test asserts the main waiver is left alone. Not changed: the `// spec` tag
   stays above the helper that holds the checks (the checker needs a `fn`); the issue wording notes are style only.
2. [x] `next-issue` skips taken issues, branch or worktree named after the stem (PAR-3, PAR-4, PAR-5).
   Done: `next_issue.rs` reads `git for-each-ref` and `git worktree list --porcelain`, skips taken issues, prints
   `Taken: <stem> (worktree <path>)` and `Every issue of the queue is taken.`; unchanged outside git or when
   nothing is taken. Test `taken_checks` (`// spec: PAR-3, PAR-4, PAR-5`). Docs: `SKILL.md`, `harness.just`.
   Verified: `just check`, `just test-harness`, `just check-harness`. The branch of the main worktree is never
   counted as an issue. Not handled: a merged branch left behind keeps its issue taken (`finish-issue`, deliverable 3,
   deletes it). Code review: git now runs in the repository of the issues directory, the queue position is kept
   in the listing, tests assert the exact `Taken:` lines and cover a middle entry, a plain directory and the
   trunk branch; the fixture sets `commit.gpgsign=false` and a branch name. Not changed: the `// spec` tag
   stays above the helper.
3. [x] `start-issue` and `finish-issue` recipes and script (PAR-1, PAR-2, PAR-6, PAR-7).
   Done: `issue_worktree.rs` (`start`, `finish`), recipes `start-issue` and `finish-issue`, section "Parallel
   agents" in `SKILL.md`. `start` also refuses an issue file the main branch does not commit (the worktree would
   not hold it), with uncommitted edits, or when the main worktree is detached or bare. Stems starting with `-`
   and `queue` are refused. `finish` prunes a worktree directory deleted by hand. Test `test_issue_worktree.rs`,
   one function per requirement. Verified: `just check`, `just test-harness`, `just check-harness`.
   Not covered by a test: the detached or bare refusals and the pruned directory.
   Code review: 6 findings; fixed the pruned directory, the leading `-` and `queue` stems, the uncommitted issue
   file, the detached and bare main worktree. Not changed: a branch with no commit of its own counts as merged
   (it cannot be told from a fast-forwarded one); `git branch -D` discards an abandoned issue.
4. [x] `close-issue` refuses to run outside the main worktree; `SKILL.md` documents the merge sequence: rebase, fast-forward, `close-issue`, `finish-issue` (PAR-8).
   Done: `close_issue.rs` refuses (move and `--relink`) in a linked worktree; `SKILL.md` "Parallel agents" gives
   the merge sequence. Test `refuses_a_linked_worktree` (`// spec: PAR-8`). Verified: `just check`,
   `just test-harness`, `just check-harness`. Code review: 6 findings;
   fixed the bare repository case, the guard now runs after the usage check, `SKILL.md` Rules point at
   "Parallel agents" for the closing. Not changed: the guard allows the run when git fails (as outside a repository), the test does not cover a
   detached main worktree or a subdirectory of the linked one.
5. [x] Measure `target/` per worktree (disk, cold build), record the decision; ADR if warranted; docs (`architecture.md`, `CLAUDE.md`), `just sync`; spec `implemented`.

   Done: measured a cold `cargo test --no-run` in a fresh worktree of this repository: 4 s wall (20 s CPU, 24
   cores), 239 MB; kept one `target/` per worktree. ADR 0003, `docs/architecture.md` section "Parallel issues",
   `CLAUDE.md`; spec `PAR` set to `implemented` (PAR-2 now names the uncommitted and detached refusals).
   The spec checker now also scans `.claude/` for `// spec:` citations (the tests of the managed scripts live
   there), with two cases in `test_check_specs.rs`. Not verified: a real two-agent session end to end, a bare
   main repository. Code review: 6 findings; fixed the detached test, the ADR wording on the merge, the duplicated
   figures, the `CLAUDE.md` bullet, the `.claude/` assumption in the architecture note. Not changed: the spec
   stays `implemented` since every requirement is traced; the two-agent session is the user's to run.

## Decisions

* Mode: autonomous (local commits, no push).
* A spec is needed (`PAR`): `next-issue`, `start-issue` and `finish-issue` change the behaviour of the tool.
* Worktrees live outside the repository: `../<repo>-<stem>`, branch `<stem>`.
* Merge: rebase onto the main branch, then fast-forward.
* "Taken" is derived from git (branch or worktree), no file.
* Main branch detection: `main`, then `master`, then the current branch of the main worktree.
* `close-issue` refuses outside the main worktree (PAR-8).
* One `target/` per worktree by default, measured in deliverable 5.
