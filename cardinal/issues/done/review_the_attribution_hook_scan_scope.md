# Narrow the text the attribution hook scans to the commit or pull request message

# Goal
`block_ai_attribution.rs` must refuse an agent credit in a message and must not refuse a command that only prints the
words; it must also catch the spaced variants of a trailer.

# Findings (from a code review, to reproduce first)
* `attribution` scans the whole command text. A commit with a clean message chained with an `echo` of a credit
  footer would be refused although the message is clean.
* The trailer test is a prefix match: a trailer written with a space before the colon may not be caught.

# Approach
1. Reproduce both with failing cases in `test_block_attribution.rs`.
2. Candidates: scan only the arguments of the commit or `gh` command segment; normalise the space before the colon.

# Resources
* `files/.claude/hooks/block_ai_attribution.rs`, `files/.claude/hooks/test_block_attribution.rs`

# Hints
* A message that only talks about the rule must keep passing.

# Tracking
1. [x] Catch a trailer written with spaces around the colon or inside the key. Done: `attribution` compares the key before the first colon without spaces, tabs and hyphens. Verified: hook test (3 blocked, 2 allowed cases added), check. Code review: spaced keys beyond the colon (fixed), dropped space in the test (fixed), no false-positive test (fixed); the scope findings belong to deliverable 2.
2. [x] Scan only the `git commit` or `gh pr` segment of the command (quote-aware split). Done: `writing_commands` cuts the line at `;`, `&`, `|` and newlines outside quotes, `main` scans the writing commands only; 6 allowed and 10 blocked cases added. Verified: hook test, check. A command with an unquoted heredoc, its own or an earlier one, takes the rest of the line. Not covered: nested quotes inside a quoted `$(...)`, `#` comments. Code review: unquoted heredoc regression (fixed), `$(a; b)` split (fixed), nested quotes / `#` comments / line continuations (not fixed, rare, kept simple); `<<` in a quoted message, backtick substitution and `2>&1` (fixed), heredoc opened before the writer (`cat <<EOF | git commit -F -`) and `<(...)` (fixed). Accepted trade-offs: a heredoc commit followed by a chained echo of a credit is still refused (fail closed), a credit written by an earlier chained command (variable, file) is no longer seen, and a relative `-F` path ignores a preceding `cd`.

## Decisions
* Mode: autonomous.
* An `echo` of a credit chained to a clean commit is allowed; a credit inside the message, including a `$(...)` in it, stays refused.
* Simplest split: walk the command tracking quotes and `$(...)`, cut at unquoted `;`, `&`, `|`, newline. A command holding an unquoted heredoc takes the rest of the line.
* No spec: the hook is not covered by `specs/`.
