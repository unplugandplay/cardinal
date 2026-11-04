# Match the keys of harness.toml exactly in the skill scripts

# Goal
`config_list` in the skill scripts finds a key with a raw `strip_prefix(key)`, so any key that merely starts with the
requested name matches. It must match the whole key, and a list value must be parsed robustly.

# Findings
* `files/.claude/skills/commit-message/commit_check.rs`: a line `commit_scopes_extra = [...]` placed before
  `commit_scopes` is read as `commit_scopes`; same for `skipped_dirs_old` and `skipped_dirs`.
* The parser splits values on `"` only: an inline comment holding a quote or a `]` corrupts a multi-line list
  without any error.

# Approach
1. Match `key`, optional spaces, then `=`; reject any other continuation of the name.
2. Stop a list at the closing `]` that is outside a string, and ignore comments.
3. Add tests with a look-alike key before the real one and with a comment inside a list.

# Resources
* `files/.claude/skills/commit-message/commit_check.rs`, `files/.claude/skills/test_skills.rs`
* `files/.claude/skills/issue-workflow/close_issue.rs`

# Hints
* Do it together with `share_the_harness_toml_reader` so the fix lands once.
