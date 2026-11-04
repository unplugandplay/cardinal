# 0009. The scanners list the repository files through git

Status: Accepted
Date: 2026-10-27

## Context
ADR 0008 nests the issue worktrees in the main one, under `.claude/worktrees/`, and made each script that walks the
repository tree skip that directory by hand: `close_issue.rs` and `check_specs.rs` did, and the next walker that
forgot would read, or rewrite, the files of every issue in progress. The skip also missed a worktree nested
anywhere else. Git already knows which files belong to a work tree: it never lists the files of a nested worktree,
whether the `info/exclude` line of `start-issue` ignores it or it shows as an embedded repository (a clone that
never ran `start-issue`).

## Decision
The managed scripts that scan a repository take their files from `git ls-files -co --exclude-standard` (tracked and
untracked, without the ignored ones) through one module, `files/.claude/repo_files.rs`, included by `#[path]` as in
ADR 0006, with the `// repo_files.rs sha256:` line in each including script. `close_issue.rs` keeps its filter by
directory name (`.git`, `target`, `node_modules`, `skipped_dirs`) on that list; `check_specs.rs` walks the disk
only outside a git repository, where no worktree can nest. Any other failure of git (a corrupt index, a refused
`safe.directory`) fails the scan rather than falling back to the disk, and a symbolic link is never followed, since
either would reach a nested worktree. The project's own tools are the project's business:
the harness documents which of them read the nested worktrees, and ships no setting for them.

## Consequences
* A new scanner calls `repo_files` and needs to know nothing of the worktrees; an ignored file (build output, a
  local note listed in `.gitignore`) is no longer scanned, and neither is the content of a submodule.
* A change of `repo_files.rs` edits the hash line of every script that includes it (ADR 0006).
* Measured on a throwaway consumer (2026-10-27): rg, tsc, pytest, ruff and mypy skip `.claude/worktrees/`; prettier,
  black, eslint 9, vitest and jest read it. A `/.claude/worktrees/` line in `.gitignore` is enough for prettier
  and black; eslint, vitest and jest need their own ignore setting (`docs/usage.md`).
