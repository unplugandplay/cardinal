# 0005. The gates catch mistakes, not bypasses

Status: Accepted
Date: 2026-10-26

## Context
`require_code_review.rs` read the text of a shell command to find the repository a commit lands in (`-C`,
`--git-dir`, `GIT_DIR`, `export`, fish `set -x`, `cd`) and to spot a write of the review waiver or a run of
`just waive-review`. Each review of the hook found a new command the parser missed, and each became a requirement: 18
commits in two days, about a third of the issues closed so far, and with its tests more than a third of the bytes
under `files/`. ADR 0004 had already made the native `pre-commit` hook the gate that judges the repository git
resolved, so the text rules only duplicated it. The agent the gates guard follows its instructions; it makes mistakes,
it does not look for a way around them.

## Decision
The gates of the harness catch the mistakes of a cooperative agent, not deliberate bypasses. A command that slips
past a gate only because nobody would write it by mistake is a non-goal, not a requirement.

The review gate rests on the native hook alone for the repository (this supersedes the part of ADR 0004 that kept the
Claude hook reading the command for it). The Claude hook keeps what a native hook cannot do: recording the review
(`record`) and refusing `--no-verify`, `-n` and `core.hooksPath`, which would skip the native hook. The waiver and its
recipe are kept from the agent by `permissions.deny` rules of the seeded `.claude/settings.json`, applied by Claude
Code, and by the instruction of the issue workflow.

## Consequences
The hook and its tests shrink to a fraction, and a review no longer turns shell parsing into new work. The agent
learns that a commit waits for a review from the refusal of `git commit` itself, after the call, not from a
`PreToolUse` refusal before it. A repository without the native hook (not synced, `core.hooksPath` set) has no review
gate; `harness check` reports a missing hook, and `sync` says it installs none under `core.hooksPath`. A deny rule only matches the command the agent usually writes (`just -f x
waive-review` or a `touch` of the waiver pass), which this decision accepts. The settings file is seeded, so a project
synced before this decision gets the deny rules only by hand: `harness check` warns while they are missing.
