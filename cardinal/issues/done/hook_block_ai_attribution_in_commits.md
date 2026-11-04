# Block Claude/Anthropic attribution in commit messages and pull request descriptions

# Goal
A hook that stops the agent from creating a commit or a pull request whose text carries a Claude or Anthropic
credit to the agent (a trailer naming it as co-author, a "generated with" footer). The rule is already written
down several times and still gets broken, because the harness default reminder asks for the opposite.

# Findings
* Found by the instruction audit (counts only): the rule exists in the memory of two projects, in the user's
  global `CLAUDE.md`, and as one explicit correction typed by the user; in one project the agent's name still
  showed up among the contributors of the hosting service.
* `commit_check.rs` already rejects the trailer, but only when the agent runs it; nothing stops a bare
  `git commit -m` or a `gh pr create --body`.
* `files/.claude/hooks/` holds `format_rust.rs` and `clippy_gate.rs`; hooks are declared in
  `.claude/settings.json`, which is seeded once (`seed/`), not managed: a new hook reaches the consumers only
  if the declaration is added to their file too.

# Approach
1. A `PreToolUse` hook on `Bash` that reads the command, and for `git commit` / `gh pr create|edit` looks for the
   attribution patterns in the message (inline, heredoc, `-F` file) and exits 2 with the reason so the agent
   rewrites it.
2. Reuse the patterns of `commit_check.rs` rather than a second list.
3. Decide how consumers get the declaration (documented snippet, or a `harness` step), at `issue-workflow`
   steps 1 to 3.

# Resources
* `files/.claude/hooks/`, `files/.claude/skills/commit-message/commit_check.rs`, `AGENTS.md`
* `seed/.claude/settings.json`, `docs/architecture.md`

# Hints
* The hook must not block a commit whose message merely discusses the rule (this repository's own history).
* The shared files hold nothing specific to one project.

# Tracking
1. [x] The hook `files/.claude/hooks/block_ai_attribution.rs` and its Rust test (`git commit` and `gh pr create|edit`: inline, heredoc, `-F`/`--body-file`; passes a message that only discusses the rule)
2. [x] Wire it: declaration in `seed/.claude/settings.json` and this repository's `.claude/settings.json`, snippet for existing consumers, docs (`update-docs`), real-case check
3. [x] A declared-hook check in `test_settings.rs` (asked by the user: the agent that updates the harness must be told)

## Decisions
* Mode: autonomous (review of step 4.5 never skipped, local commits, no push).
* No spec, no ADR: a hook of the harness changes no behaviour of the `harness` binary. `harness check` stays lock-only (SYN-8), so it does not verify consumers' `settings.json`; they get a documented snippet.
* The hook has its own, narrower patterns than `commit_check.rs` (a trailer or "generated with" line naming Claude or Anthropic, `<noreply@anthropic.com>`) so a message that merely discusses the rule is not blocked; no shared list.

## Deliverable 1 summary
* Done: `block_ai_attribution.rs` and `test_block_attribution.rs` (24 cases), synced into this repository.
* Verified: `just check`, `just test-harness`, `just check-harness`. Not verified: the hook in a live session (not wired yet, deliverable 2).
* Code review: fixed `git` with many global options, the bare `<noreply@anthropic.com>` match that blocked discussions, and a trailer naming someone else. Not fixed, by design: a trailer in the middle of a line (git does not read it as a trailer), `$(cat file)`, `-F` paths with spaces or `~`, `gh pr comment`/`git tag -m`, and a line that discusses the rule while starting with "Generated with Claude".

## Deliverable 2 summary
* Done: the `PreToolUse` declaration in `seed/.claude/settings.json` and `.claude/settings.json`; the snippet for existing consumers and the hook described in `docs/architecture.md`.
* Verified: `just check`, `just test-harness` (settings hygiene included), `just check-harness`; the hook run by hand on a faulty `git commit` exits 2 with the reason. Not verified: the hook triggered by Claude Code itself in a fresh session (settings are read at session start).
* Code review: no findings.

## Deliverable 3 summary
* Done: `test_settings.rs` fails for a hook script of `.claude/hooks/` that `settings.json` does not declare, naming the event, matcher and command; each hook header carries a `Declare:` line. Docs updated.
* Verified: removing the declaration makes `test_settings.rs` fail with that message; restored, `just check` and `just test-harness` pass. Not verified: a run on a real consumer.
* Code review: no findings.
