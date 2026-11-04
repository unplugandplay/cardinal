# Fail loudly when a skill script cannot read harness.toml

# Goal
`commit_check.rs` must read `.claude/harness.toml` once, from the repository root, and report a missing or
unreadable file instead of silently falling back to defaults.

# Findings
* `scopes()` re-reads and re-parses `harness.toml` on every `check_subject` call, through a path relative to the
  current directory.
* A failed read goes through `unwrap_or_default`: only `docs` and `harness` stay accepted, so a valid
  `feat(pipeline): ...` is rejected as an unknown scope with no hint that the config was not read.
* This happens when the script runs from a directory other than the repository root.

# Approach
1. Resolve the root (`git rev-parse --show-toplevel`, or the script's own location) and build the path from it.
2. Read the file once and pass the scopes to `check_subject`.
3. When the file is missing, print an explicit error and exit non-zero; keep the defaults only if the project has
   no `harness.toml` by design.
4. Test: run the check from a subdirectory; test the missing-file message.

# Resources
* `files/.claude/skills/commit-message/commit_check.rs`, `files/.claude/skills/commit-message/test_commit_check.rs`

# Tracking
1. [x] Read `harness.toml` once from the git root in `commit_check.rs`, fail with exit 2 and an `error:` line when it cannot be read; tests from a subdirectory, with a missing file, an invalid file and outside a repository.

## Decisions
* No fallback to the built-in scopes: a missing `harness.toml` is always an error.
* Outside a git repository the checker fails instead of using the current directory.
* No spec: the change touches a skill script, not the behaviour of `specs/sync.md`.
