# Limit the directories the spec checker skips

# Goal
`check_specs.rs` must skip the build output and the agent worktrees, and nothing else, so that a test living in a
directory with an unlucky name is still traced.

# Findings (from a code review of the synced revision, to reproduce first)
* `rust_files` skips every directory named `target` or `worktrees` at any depth, `crates/` included. A module
  directory `src/worktrees/` or a `tests/target/` folder holding `// spec: ID` tests is no longer scanned, so a spec
  looks untraced or a citation is silently ignored.

# Approach
1. Add failing tests: a test file under `src/worktrees/` and under `tests/target/` is scanned.
2. Skip only `<root>/target` and `<root>/.claude/worktrees` (plus the project `skipped_dirs`), by path from the root.

# Resources
* `files/.claude/specs/check_specs.rs` (`rust_files`), `files/.claude/specs/test_check_specs.rs`, `specs/parallel-issues.md`

# Hints
* Worktrees of issues are siblings of the repository or under a known directory: check where `issue_worktree.rs`
  creates them before choosing the paths.
