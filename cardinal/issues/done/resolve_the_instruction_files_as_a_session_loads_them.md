# Resolve the instruction files as a session loads them

# Goal
`memory.rs` resolves the memory files from the root of one tree: `audit-sources` (AUD-1) treats each session's
working directory as that root, and `fixed-context` (REP-2) the repository. Two cases differ from what a session
loads, so the audit can miss a rule the user wrote and propose it again:
* a session started in a subdirectory also loads the `CLAUDE.md` files of its ancestor directories, which the audit
  does not list;
* the imports of a memory file reached through a symbolic link are resolved against the directory of the link, not
  of the file it points to.

# Findings
* 2026-10-28, `high` review of `issues/done/list_the_always_on_rules_in_the_instruction_audit.md`: both reported on
  `audit_sources.rs` (one `WorkTree` per working directory) and `memory.rs` (`resolve(dir, …)` with `dir` the
  parent of the queued path).
* Not measured: which directory Claude Code resolves the import of a linked memory file against.

# Approach
1. Measure first: a session in a scratch repository with a linked `CLAUDE.md` importing a relative path, and one
   started in a subdirectory, to see what the session loads.
2. For the audit: walk the ancestors of the working directory up to the repository root (or the filesystem root)
   and list their memory files.
3. For the imports: resolve against the directory of the resolved file, in `memory.rs`, so that `fixed-context`
   follows; amend REP-2 if the measure says so.

# Specs
* `specs/instruction-audit.md` AUD-1, `specs/harness-report.md` REP-2.

# Resources
* `crates/harness/src/memory.rs`, `crates/harness/src/tools/audit_sources.rs`

# Tracking

Plan: validated (done)

## Understanding
Make the audit list the instruction files of the ancestors of each working directory, and resolve the imports of a
linked memory file against the directory of its target, for the audit and `fixed-context` alike. Out of scope: the
approval Claude Code asks before loading an import outside the working directory.

## Measure (2026-10-28, Claude Code 2.1.312, `claude -p` in scratch repositories)
* A linked `CLAUDE.md` (`CLAUDE.md -> sub2/target.md`) importing `@inc.md` and `@./inc2.md` loads `sub2/inc.md`
  and `sub2/inc2.md`, not the files next to the link: imports resolve against the directory of the target.
* A session in `repo/a/b` loads the `CLAUDE.md` of `repo/a/b`, `repo/a`, `repo` and of the directory above the
  repository, and the unscoped rules of `repo/a/.claude/rules` and `repo/.claude/rules`; the walk does not stop at the
  repository root. `repo/a/AGENTS.md` was not loaded (`repo/a/CLAUDE.md` exists).
* From `repo/a/b`, the import of `repo/sub2/target.md` (outside the working directory) was not loaded: an external
  import waits for an approval a `-p` session never gives. The audit still lists it: the user wrote it.

## Plan
1. [x] Linked imports: `memory.rs` resolves an import against the directory of the resolved file; amended REP-2
   (scenario of the linked `CLAUDE.md`); verified by a `fixed-context` test on a commit and an `audit-sources` test on
   a work tree; makes true REP-2, AUD-1. ~60 lines.
2. [x] Ancestors: `audit-sources` lists the instruction files of each ancestor of the working directory up to the
   root of its repository, by absolute path, the home directory's `.claude/` excepted; amended AUD-1; verified by an `audit-sources` test with `HOME` set to a
   temporary directory; makes true AUD-1. ~120 lines.
3. [x] Fixes of the `high` review: the ancestors read in the tree of the repository (an import may climb to any of
   its directories), the home directory's `.claude/` left out only from the listing of the home directory, AUD-1 worded with absolute
   paths; verified by the ancestors test of `audit_sources`; makes true AUD-1.

## Decisions
- Mode: autonomous
- Base commit: 301f4979b0d39eed4fb4fafcafc85af6bb26970b
- Ancestors: the walk stops at the repository root (nearest `.git`; the working directory alone without one),
  though a session loads the files above it too: reading outside the repository is a security and privacy concern
  (user, 2026-10-28).

## Deliverable 1
- done: an import of a linked memory file resolves against the directory of its target (`memory.rs`), for
  `fixed-context` and `audit-sources`; REP-2 amended, AUD-1 amended for deliverable 2 too.
- | check | result |
  |---|---|
  | tests | pass |
  | lint | pass |
  | specs | pass |
  | docs | n/a: no document states the directory an import resolves against |
  | code review | see below |
- red: REP-2, AUD-1: `fixed_context` counted 51 bytes instead of 54 for `r1`; `audit_sources_lists_every_instruction_file_once` lacked `docs/x.md`.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `audit-sources` lists the instruction files of each working directory and of its ancestors up to the
  repository root (nearest `.git`), by absolute path, sorted, the home directory's `.claude/` left out; the `audit-instructions` skill
  says so.
- | check | result |
  |---|---|
  | tests | pass |
  | lint | pass |
  | specs | pass |
  | docs | updated: `files/.claude/skills/audit-instructions/SKILL.md` (synced) |
  | code review | see below |
- red: AUD-1: `audit_sources_lists_the_ancestors_up_to_the_repository_root` listed only the `CLAUDE.md` of the working directory.
- Code review: no findings
- not verified: a worktree nested in the main one (ADR 0011) stops the walk at its own `.git` file, though the session
  also loads the main worktree's `CLAUDE.md`; not measured.
- decisions needed: none

## Deliverable 3 (fixes of the `high` review)
- done: `memory.rs` reads the memory files of a directory inside a tree; `audit-sources` reads every ancestor in the
  tree of the repository; the the home directory's `.claude/` filter only applies to the home directory's own listing; AUD-1 states the
  absolute paths.
- | check | result |
  |---|---|
  | tests | pass |
  | lint | pass |
  | specs | pass |
  | docs | n/a: the skill already says "ancestors up to the repository root" |
  | code review | see below |
- red: AUD-1: the ancestors test lacked `docs/up.md` (imported with `../../`) and the rule linked into `.claude/shared/`.
- Code review: no findings
- High review, not fixed: the walk stops at the `.git` file of a nested worktree (decision: the repository bounds the
  walk); the imports of a linked target outside the repository count nothing (REP-2); the ancestors are printed
  again for each working directory (AUD-1 lists per working directory); a relative `cwd` (transcripts record absolute
  ones); `CLAUDE.local.md` never listed (`issues/done/list_the_local_memory_files_in_the_audit.md`).
- not verified: none
- decisions needed: none
