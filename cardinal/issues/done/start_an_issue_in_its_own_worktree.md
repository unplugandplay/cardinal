# Start an issue in its own worktree

# Goal
The worktree isolation of ADR 0003 is never used: an agent handed an issue runs `issue-workflow` in the main worktree,
so two sessions share files, index and `target/`, and `just next-issue` still offers the issue as free. Make starting
an issue in its own worktree the default path of `issue-workflow`, and make the hooks act on the worktree the agent
works in.

# Findings
* Transcripts of the user's projects (2026-10-27): 188 sessions name `issue-workflow` across 5 projects; no session
  ran `just start-issue` as a command, and no Claude Code project directory exists for an issue worktree
  (`<repo>-<stem>`), so no session was ever opened in one.
* Observed on 2026-10-27: a session drove a newly created issue in the main worktree; its untracked draft spec made
  the review gate refuse another session's commit of `issues/` files only (the snapshot covers untracked files).
* `issue-workflow` `SKILL.md`: step 0 never mentions the worktree; the section "Parallel agents" says two agents
  "can" work in worktrees and sends to `details.md`, itself titled "the rare cases". `details.md` expects the user to
  run `just start-issue` and to "open the agent there".
* `start-issue` refuses an issue that is not committed on the main branch (PAR-2): an issue just written by
  `create-issue` and handed over at once cannot get a worktree.
* `format_rust.rs` and `clippy_gate.rs` run `cargo fmt` and `cargo clippy` in `CLAUDE_PROJECT_DIR`, the directory
  the session was opened in: an agent that moves into an issue worktree would format and lint the main worktree,
  where another agent works. `require_code_review.rs record` uses the `cwd` of the event, so it follows the agent.

# Approach
Measure first: in a real session, check what Claude Code passes to the hooks (`cwd`, `CLAUDE_PROJECT_DIR`) after
the agent changes directory, and whether its built-in worktree tool (`EnterWorktree`) can enter an existing
directory or only creates its own.
1. Step 0 of `issue-workflow`: in the main worktree, when the issue is not taken, run `just start-issue` and do all
   the work in the worktree (every command run from it); when the issue is uncommitted, ask the user to commit it
   first, or commit it alone.
2. The hooks resolve the work tree from the `cwd` of the event, as `record` does, rather than from
   `CLAUDE_PROJECT_DIR`.
3. Or keep the user's launch in the worktree, but make `issue-workflow` refuse to start in the main worktree when
   another issue is taken or the tree is dirty with files of another issue, telling the user the command to run.

# Specs
* `specs/parallel-issues.md`: requirements for the start of an issue and for the directory the hooks act on;
  `specs/clippy-gate.md` for the gate's directory.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (step 0, "Parallel agents"), `files/.claude/skills/issue-workflow/details.md`
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/hooks/format_rust.rs`, `files/.claude/hooks/clippy_gate.rs`
* `docs/adr/0003-one-worktree-per-issue.md`

# Hints
* The worktree has its own `.claude/` copy (tracked), so hooks invoked with `CLAUDE_PROJECT_DIR` still run the main
  worktree's scripts: the same revision only while both are on the same commit of `files/`.
* A session opened in the worktree gets its own Claude Code project directory, which the transcript readers must map
  back to the repository (`transcripts.rs` resolves a project from its directory).

# Tracking

Plan: validated

Status: done (2026-10-27)

## Understanding
An agent handed an issue in the main worktree starts it in its own worktree (`just start-issue`, then Claude Code's
`EnterWorktree` with `path`), and the hooks that run cargo act on the worktree of the event's `cwd`. Out of scope:
refusing to start in a dirty main worktree (approach 3, no longer needed), the transcript readers (they already map
a linked worktree to its repository), running the worktree's own copy of the hook scripts.

## Measures (2026-10-27, Claude Code session in this repository)
- A Bash `cd` to a directory outside the launch directory is reset after the command ("Shell cwd was reset"): an
  agent cannot "run every command from the worktree" by `cd`.
- `EnterWorktree` with `path` enters an existing worktree listed by `git worktree list` (here a detached one in
  `/tmp`): the session's working directory and the `cwd` of the events become the worktree, the tool refuses a
  `git -C <main worktree>`, and the transcript continues in a new project directory named after the worktree.
  `ExitWorktree` with `keep` returns to the launch directory and leaves the worktree on disk.
- After `EnterWorktree`, an unformatted edit of a Rust file of the worktree stayed unformatted: `format_rust.rs`
  ran `cargo fmt` in `CLAUDE_PROJECT_DIR`, the launch directory.

## Plan
1. [x] Hooks follow the agent: `format_rust.rs` formats from the edited file's directory and `clippy_gate.rs`
   resolves the work tree of the event's `cwd` (deliverable 3 settled both); verified by
   `test_hooks.rs` cases where `CLAUDE_PROJECT_DIR` points at another clean project; makes true PAR-12, CLG-6.
2. [x] `issue-workflow` starts in the worktree: step 0 runs `just start-issue` then `EnterWorktree` with `path` when
   the session is in the main worktree and the issue is free (taken: stop), commits an uncommitted issue alone
   first (decision 1); the end of the issue merges back from the main worktree (decision 2); "Parallel agents" and
   `details.md` rewritten as the default path, `docs/architecture.md` updated (ADR 0003 unchanged, see Decisions); verified by
   `just test-harness`, `just check-harness` and the review (skill text, no automated test).
3. [x] Fixes of the `high` review: the gate keeps the place of `CLAUDE_PROJECT_DIR` in its work tree (a crate in a
   subdirectory) and falls back to it outside git; the format hook follows the edited file; step 0 detects the main
   worktree with absolute paths and commits the issue with its queue line and nothing else staged; verified by
   `test_hooks.rs`; makes true PAR-12, CLG-6 as amended.

## Decisions
- Mode: autonomous
- Base commit: 8a31a5df0f4e8c2026f25a8e9ff8774502c46dc1
- 1a: an uncommitted issue is committed (with its queue line, nothing else staged) by the agent (`docs: open the issue ...`) before `start-issue`.
- ADR 0003 left as is: its decision (one worktree per issue) is unchanged; who starts it is in `docs/architecture.md`.
- 2a: once the user agrees to close, the agent rebases the branch in the worktree, leaves it (`ExitWorktree` keep), fast-forwards from the main worktree, then runs `close-issue` and `finish-issue` (the rebase moved into the worktree: a branch checked out there cannot be rebased from the main one).

## Deliverable 1: hooks follow the event's `cwd`
- done: `format_rust.rs` and `clippy_gate.rs` resolve the top-level of the event's `cwd` (fallback
  `CLAUDE_PROJECT_DIR`); PAR-12 and CLG-6 traced, both specs back to `accepted`; `docs/architecture.md` says so.
- red: `FAIL - format hook formats the worktree of the event's cwd`, `FAIL - format hook leaves CLAUDE_PROJECT_DIR
  alone when the event has a cwd`, `FAIL - gate checks the worktree of the event's cwd`.
- check: `just check` ok, `just test-harness` ok, specs traced, docs updated, code review low run.
- Code review: no findings
- not verified: a live session where the hooks run from the main worktree's `.claude/` against an issue worktree
  (covered by the hook tests with a real linked worktree).

## Deliverable 2: `issue-workflow` starts in the worktree
- done: step 0 enters the issue's worktree (existing one: `EnterWorktree`; free issue: commit it alone if needed,
  `just start-issue`, `EnterWorktree`; branch without worktree: stop); "Parallel agents" became "Merge back";
  `details.md` describes the start and the merge back (rebase, `ExitWorktree`, fast-forward, `close-issue`,
  `finish-issue`); `docs/architecture.md` and `CLAUDE.md` updated.
- check: `just check` ok, `just test-harness` ok, `just check-harness` ok, code review low run.
- red: none (skill text, no automated test).
- Code review: no findings
- not verified: a whole issue driven through this step 0 and merge back in a live session.

## Deliverable 3: fixes of the `high` review
- done: findings 1 to 6 and 9 of the `high` review fixed (see plan line 3); PAR-12 and CLG-6 amended, specs back to
  `accepted`.
- red: `FAIL - format hook formats the worktree of the edited file`, `FAIL - format hook leaves the project of the
  session alone`, `FAIL - gate checks the crate's subdirectory in the worktree of the event's cwd`, `FAIL - gate
  checks CLAUDE_PROJECT_DIR when the event's cwd is outside git`.
- not fixed: 7 (three copies of the work-tree lookup: each hook is a standalone script, and the format hook no
  longer has one); 8 (sibling worktrees left in the temp directory when a test panics, unescaped paths in the test
  JSON: the existing temp project has the same behaviour, the temp paths hold no quote).
- check: `just check` ok, `just test-harness` ok, `just check-harness` ok, code review low run.
- Code review: no findings
