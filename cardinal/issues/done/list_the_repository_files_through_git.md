# List the repository files through git

# Goal
The issue worktrees now nest in the main one, under `.claude/worktrees/` (ADR 0008). Every script that walks the
repository tree has to skip that directory by hand, and the next one that forgets reads, or rewrites, the files of
every issue in progress. List the files through git instead, which already honours the `info/exclude` line that
`start-issue` writes, and say how a consumer keeps its own tools out of the nested worktrees.

# Findings
* `/code-review high` of the issue `create_issue_worktrees_under_claude_worktrees` (2026-10-27): `close_issue.rs`
  and `check_specs.rs` each hard-code a skip of `.claude/worktrees` of the root they walk, and ADR 0008 records
  that a new walker must do the same.
* `close_issue.rs` also skips `.git`, `target`, `node_modules` and the `skipped_dirs` of `.claude/harness.toml` by
  directory name; `check_specs.rs` skips `target` and `.claude/worktrees` by full path.
* The same review: the tools of a consumer that walk the filesystem without git's ignore rules (test runners,
  linters, formatters, type checkers run on `.`) now see the nested worktrees in the main one. Not measured on a
  consumer yet.

# Approach
1. List every walker of the repository tree in `files/` and `crates/` (the two above, and any other found).
2. Compare a shared helper over `git ls-files -co --exclude-standard` (tracked and untracked, without the ignored
   files, nested worktrees included) with the current walks: untracked documents still found, behaviour outside a
   git repository, the `skipped_dirs` setting kept or dropped.
3. For the consumer tools: measure in a test consumer which tools read `.claude/worktrees/`, then choose between a
   note in the docs for the project owner and a seed default; the shared files stay generic.

# Specs
* `specs/parallel-issues.md`: PAR-14 (the scanners skip `.claude/worktrees/`) may be restated as "the scanners list
  the files through git".

# Resources
* `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`, `specs/parallel-issues.md` (PAR-13, PAR-14).
* `files/.claude/skills/issue-workflow/close_issue.rs` (`text_files`), `files/.claude/specs/check_specs.rs` (test
  file collection) and their tests.

# Hints
* `git ls-files` lists a nested worktree as nothing when it is ignored, but as an embedded repository when the
  `info/exclude` line is missing (a clone that never ran `start-issue`): check both cases.
* `close_issue.rs` rewrites untracked documents too (`-o` is needed, not only the tracked files).

# Tracking

Plan: validated

## Understanding
Replace the two hand-written walks of the repository (`close_issue.rs` `text_files`, `check_specs.rs` `rust_files`)
by one listing through `git ls-files -co --exclude-standard`, so no scanner needs to know `.claude/worktrees/`. Then
measure which consumer tools read the nested worktrees and document how to keep them out. Out of scope: the
walks of fixed directories (`.claude/hooks`, `.claude/skills`, `issues/done`, `specs/`, the `files/` tree of `sync`).

## Plan
1. [x] Shared module `files/.claude/repo_files.rs` (`#[path]` include, ADR 0006, with its sha256 line): the files of
   a root from `git ls-files -co --exclude-standard -z`, dropping the directory entries of an embedded repository
   (`dir/`, a nested worktree when `info/exclude` lacks its line) and the paths that no longer exist; `None` outside
   a git repository. `close_issue.rs` uses it, keeping `.git`, `target`, `node_modules` and `skipped_dirs` as a
   filter by directory name on the listed paths; verified by `test_close_issue.rs` (nested worktree ignored and not
   ignored, untracked document rewritten); makes true PAR-14.
2. [x] `check_specs.rs` uses the module, and falls back to the walk skipping `target/` outside a git repository;
   verified by `test_check_specs.rs` (cases in a git repo with a nested worktree with and without the exclude
   line, an untracked test still traced, the existing cases outside git); makes true PAR-14, PAR-16; spec back to
   `accepted`.
3. [x] ADR 0008 and `docs/architecture.md` restated ("the scanners list the files through git"); measure in a
   throwaway consumer with a nested worktree which tools read it (cargo, rg, and through `npx`/`uvx`: prettier,
   eslint, tsc, vitest, pytest, ruff, mypy), and write the result with the way out in `docs/architecture.md`
   (note for the project owner) or a seed default, per the measure.

## Deliverable 1
- done: `files/.claude/repo_files.rs` lists the files through git; `close_issue.rs` rewrites the git-listed text
  files, minus `.git`, `target`, `node_modules` and `skipped_dirs` by directory name.
- checks: `just check` ok, `just test-harness` ok, `just check-specs` ok (spec in `draft`, old wording warnings only),
  docs: `docs/architecture.md` (parallel issues).
- red: `FAIL - the files of a worktree nested anywhere else are untouched`, `FAIL - --relink leaves the worktree git
  ignores untouched`, `FAIL - close_issue.rs includes the shared module`.
- Code review: 1 finding: a failed `git ls-files` relinked nothing silently; fixed, `relink` now warns (it only
  runs once `git rev-parse --show-toplevel` succeeded, so the case is rare).
- not verified: a consumer with submodules (dropped, not tested).

## Deliverable 2
- done: `check_specs.rs` scans the `.rs` files git lists under `crates/` and `.claude/`, all of them outside git;
  PAR-16 restated without `target/` (the root's `target/` is never under `crates/` nor `.claude/`, so that skip was
  dead); spec back to `accepted`.
- checks: `just check` ok, `just test-harness` ok, `just check-specs` ok; docs: `docs/architecture.md`.
- red: `crates/y/crates/a/src/bad.rs: cites RER-9, which no spec defines` (worktree nested outside
  `.claude/worktrees`), `FAIL - outside git, a test under .claude/worktrees is traced`.
- Code review: no findings (one doubt, paths relative to `root`, ruled out: `repo_files` joins them to `root`, and
  the `in_git` test traces RER-2 only through an untracked file).
- not verified: nothing.

## Deliverable 3
- done: ADR 0009 (the scanners list the files through git, with the measure), a pointer in ADR 0008, `adrs` of the
  PAR spec, `docs/architecture.md`; a README note for the project owner (no seed default: the settings belong to
  each tool's config, which the harness does not own).
- measure (throwaway consumer, nested worktree ignored by `info/exclude`, one bad file per tool): rg, tsc 5, pytest,
  ruff (check and format), mypy skip it; prettier 3, black, eslint 9, vitest 3, jest 29 read it. A `.gitignore`
  line fixes prettier and black; eslint `ignores`, vitest `exclude` and jest `testPathIgnorePatterns` +
  `modulePathIgnorePatterns` fix the others (each checked).
- seen on the way: Claude Code registers the skills of a nested worktree's `files/.claude/skills` as scoped variants
  once a file there is read; harmless (they are named by their path), not acted on.
- checks: `just check` ok, `just test-harness` ok, `just check-harness` ok.
- red: none (docs and a measure).
- Code review: 3 findings on `repo_files.rs` and `check_specs.rs` of the earlier deliverables. Fixed: a file in a
  merge conflict listed once per stage (now deduplicated), a non-UTF-8 name lost by a lossy conversion (now built
  from the bytes on unix). Not fixed: "the fallback outside git no longer skips `target/`": the old skip was
  `root/target`, never under `crates/` nor `.claude/`, so nothing changed (PAR-16).
- not verified: vitest's `configDefaults` form with vitest installed locally (checked with the literal list; the
  probe had no local vitest to import).

## Deliverable 4 (fixes of the `high` review)
- done: `repo_files` returns `Ok(None)` only for "not a git repository" and an error otherwise, which fails
  `check_specs` (no walk of the disk) and exits `close_issue` with 1 naming `--relink`; symbolic links are dropped;
  `check_specs` passes the pathspec `crates .claude`. ADR 0009 says so.
- not fixed, with the reason: a global gitignore hiding `.claude/`, and a root inside an ignored directory, make the
  scan fail loudly (untraced requirements), and ADR 0009 states that an ignored file is no longer scanned;
  `--deduplicate` needs git 2.31 where the sort costs nothing; the copied stamp check became
  `issues/done/share_the_hash_stamp_check_of_the_shared_modules.md`.
- checks: `just check` ok, `just test-harness` ok, `just check-harness` ok.
- red: `crates/a/src/link.rs: cites RER-9`, `FAIL - in git, a failing git ls-files fails the check without walking
  the disk`, `FAIL - a failing git ls-files fails, naming --relink`.
- Code review: 1 finding: "not a git repository" is matched in git's stderr, which git translates; fixed with
  `LC_ALL=C` on the git command (checks rerun under `LANG=fr_FR.UTF-8`).
- not verified: a refused `safe.directory` (simulated by a corrupt index, the same path through the code).

## Decisions
- Mode: autonomous
- Base commit: aa6ca4ef6c635b6b52b6741acd2db37d89ded987
- 1 a: one shared module `files/.claude/repo_files.rs` (ADR 0006 pattern).
- 2 a: `close_issue.rs` keeps `.git`, `target`, `node_modules` and `skipped_dirs` as a name filter on the git list.
- 3 a: the consumer measure may fetch tools through `npx` and `uvx`.
