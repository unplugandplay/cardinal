# Fix the commit and pull request detection of the attribution hook

# Goal
`block_ai_attribution.rs` must refuse an agent credit in every commit or pull request text it is meant to cover,
and must not scan commands that only mention the words.

# Findings
* Found by the code review of the review-gate hook (two runs): `writes_message` only matches `gh` when `pr` directly
  follows it, so `gh -R owner/repo pr create --body "...<a co-author trailer naming the agent>..."` or `gh --repo x pr edit`
  is not scanned and the credit is not blocked.
* It also matches the word `commit` anywhere in the 12 words after `git`, so `git log --grep commit` is scanned
  as a commit.
* `require_code_review.rs` already parses the subcommand after the global options of `git` (`commit_dir`); the
  same idea applies to `gh` (global flags before `pr`).

# Approach
1. Add failing cases to `test_block_attribution.rs`: `gh` with a flag before `pr`, `git log --grep commit` and
   `git show commit` (must pass through).
2. Find the subcommand after the global options for `git` and `gh`, instead of a fixed window.
3. Consider sharing the parsing with `require_code_review.rs` if the shared files stay single-script.

# Resources
* `files/.claude/hooks/block_ai_attribution.rs`, `files/.claude/hooks/test_block_attribution.rs`
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`)

# Hints
* The shared files hold nothing specific to one project.
* A hook script is a single file run by `rust-script`: a shared module is not free.

# Tracking

1. [x] Find the `git` and `gh` subcommand after the global options in `writes_message` (tests first)

## Decisions

* Mode: autonomous.
* No spec: the hooks have no spec (`specs/sync.md` covers the `harness` tool).
* The option parsing follows `commit_dir` (`require_code_review.rs`) but is not shared: a hook stays a single file.
* Changed after the code review (a fail-open regression): every flag is skipped and a value is consumed for the known value options (`git`: `-C -c --git-dir --work-tree --namespace --exec-path --super-prefix`; `gh`: `-R --repo --hostname`), quoted values included; newline is a separator.

## Deliverable 1 summary

* Done: `writes_message` finds the subcommand after the global options of `git` and `gh`; tests added for `gh -R/--repo/--hostname`, bracketed and multi-line commits, quoted option values and subcommand, `--git-dir`, `git log --grep commit`, `git show commit`, `gh pr view`, `gh issue create`. Copies synced.
* Verified: `just check`, `just test-harness`, `just check-harness`.
* Not verified: real use in a session; an unknown option that takes a value in a separate word before the subcommand hides it (no scan).
* Code review: 8 findings. Fixed: the fail-open on `--git-dir x`-style options and on quoted values (all flags skipped, value options listed, quoted values consumed), the quoted subcommand, the newline separator, the missing tests, the docs overstating the guarantee; the duplicated option list is gone (the hook no longer needs it). Not fixed: `gh` unknown value options (listed in the docs; the list is the extension point).
