# Skip the value of every git global option in the review hook

# Goal
`require_code_review.rs` must see a `git commit` whatever global options precede it, so that the review gate cannot be
bypassed by `git --git-dir X commit`.

# Findings
* `commit_dir` skips `-C <dir>`, `-c <kv>`, `SKIPPED_OPTIONS` and the joined forms `--git-dir=` and `--work-tree=`. In
  `git --git-dir /x/.git commit -m foo` the path word is taken for the subcommand, so no commit is seen and the commit
  is not gated. Same for `--namespace`, `--exec-path`, `--super-prefix` given as two words.
* `block_ai_attribution.rs` already lists these options in `GIT_VALUE_OPTIONS`: the two hooks disagree.

# Approach
1. Add failing cases to `test_require_code_review.rs` for each two-word option.
2. Use the same list as the attribution hook, or share the parsing if single-script files allow it.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`), `files/.claude/hooks/block_ai_attribution.rs` (`GIT_VALUE_OPTIONS`)

# Hints
* A hook script is a single file run by `rust-script`: a shared module is not free.

# Tracking
1. [x] Skip the value of every git global option in `commit_dir`, with failing cases first

## Decisions
* Mode: autonomous.
* The list is duplicated in both hooks (a hook is a single file) with a cross-reference comment; no spec covers the hook.

## Summary
* Done: `require_code_review.rs` skips `--git-dir`, `--work-tree`, `--namespace`, `--exec-path`, `--super-prefix` as two words (quoted values included) or `--opt=value`; the new cases, which failed first, now pass; `-c` and `-C` also take a quoted value with spaces.
* Verified: `just check`, `just test-harness`. Not verified: nothing else.
* Code review: found the same quoted-value bypass for `-c`/`-C` (fixed with `take_value`); `--exec-path`/`--super-prefix` kept as two words on purpose (over-skipping is harmless, same list as the attribution hook); `--git-dir`/`--work-tree` not used to pick the repository to check (not fixed here, follow-up); unbalanced quote and duplicated code (single-file hooks) accepted.
