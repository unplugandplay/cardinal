---
priority: low
why: 'a comfort: the lock costs a hook call per tool call but harms no one working in a single session'
---
# Turn off the workspace lock

# Goal
The workspace lock runs on every tool call (`PreToolUse`), at the start and at the end of each session, to keep
one session per worktree. A user who never runs agents in parallel pays the calls for nothing. The outcome: a feature
`workspace-lock` made of its three hook entries.

# Findings
* The three hooks are `workspace-lock check`, `start` and `release` in `entries/.claude/settings.json`.
* `issue-workflow` claims each issue with `cardinal run issue-claim`, which also refuses uncommitted changes:
  whether the claim belongs to the feature is open.

# Approach
1. Declare the feature with the three entries.
2. Decide whether `issue-claim` stays (it orders `next-issue` across worktrees) or becomes a no-op when the lock is
   off, and amend `specs/issue-claim.md` if so.

# Specs
* `specs/features.md`, `specs/workspace-lock.md`, `specs/issue-claim.md` if the claim changes.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* ADR 0022, ADR 0024, `docs/architecture.md` (Parallel agents).

# Tracking

Plan: validated

## Understanding
A feature `workspace-lock` made of its three hook entries, so a user who never runs parallel sessions drops the hook
call per tool call. Out of scope: `issue-claim`, which stays on (one call per issue; it orders `next-issue` across
worktrees and refuses a start on uncommitted changes), so `specs/issue-claim.md` is unchanged.

## Plan
1. [x] Declare the feature: the table `workspace-lock` in `features.toml` with the three entries; the hook-only test
   of FEA-16 extended to it (three entries, no file, no `needs`); docs (`docs/architecture.md`, the features list of
   the README if any); verified by `just check`; makes true FEA-35.
2. [x] Fixes of the `high` review: the test names the three exact commands of FEA-35 and tolerates a hook with no
   command; `docs/workflow.md` says the lock may be off; verified by `just check`.

## Decisions
- Claim: `issue-claim` stays on with the lock off (answer 1 a).
- Mode: autonomous
- Base commit: 96cbd50dd203e545c16f571402cb4b9aa78ff45c

## Summaries

### 1. Declare the feature
- done: `features.toml` declares `workspace-lock` (its three hook entries); the hook-only features test covers it;
  README, `docs/usage.md`, `docs/gates.md`, `docs/architecture.md` and the details of `issue-workflow` say so.
- red: FEA-35: `features.toml declares no workspace-lock`
- not verified: a real consumer synced with `disabled = ["workspace-lock"]` (FEA-3 covers the removal of hook entries).
- Code review: no findings

### 2. Fixes of the high review
- done: the test checks the exact commands; `docs/workflow.md` mentions the feature.
- red: none (the test was green before, it now proves FEA-35 by name).
- High review: 8 findings. Fixed: the exact commands, the `unwrap` on a hook with no command, the prefix allocated per
  hook, `docs/workflow.md`. Not fixed: the wording of `issue-workflow` (a non-goal of FEA), the lock file left by a
  sync with the feature off (a reused process id is a non-goal of WSL), ADR 0022 (ADR 0025 decides the features, as
  for `review-gate`), `CLAUDE.md` and the README Gates row (this repository keeps the lock; the other gates carry no
  caveat there either).
- Code review: no findings
