# Share one harness.toml reader between the skill scripts

# Goal
The hand-rolled `config_list` TOML parser is copied verbatim into three scripts. A fix must be made once.

# Findings
* Copies in `files/.claude/skills/commit-message/commit_check.rs`, `files/.claude/skills/issue-workflow/close_issue.rs`
  and `files/.claude/skills/test_skills.rs`.
* The copies will drift, and each script would then read `harness.toml` differently.

# Approach
1. Use the `toml` crate through the `rust-script` dependency header (`//! ```cargo`) in each script, which removes
   the parser; or
2. keep a single `config.rs` helper synced into the project and pulled in with `#[path]` / `include!`.
3. Compare the compile time of the scripts before choosing: `rust-script` caches builds, a new dependency costs the
   first run in CI.

# Resources
* The three files above; `crates/harness/` for how the tool itself parses `harness.toml`.

# Hints
* Related: `match_config_keys_exactly` and `fail_loudly_when_harness_toml_is_unreadable`. Option 1 fixes the
  first one for free.
