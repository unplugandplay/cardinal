---
priority: medium
why: 'the gate a solo user or a team reviewing in pull requests most wants to drop, and the hardest: it installs a git hook outside the lock'
---
# Turn off the review gate

# Goal
The review gate refuses a commit whose diff no code review saw: the `review-gate` hooks, the native git
`pre-commit` hook and the deny rules of the waiver. A project that reviews in pull requests, or a solo user, cannot
turn it off. The outcome: a feature `review-gate` whose removal also uninstalls the native gate `sync` installed.

# Findings
* The native gate is installed by `sync` into the git hooks directory, outside the lock (SYN-18 to SYN-23);
  `gate::manages` decides it from the managed entries, so leaving the entries out already stops the install.
* `check` reports `missing: native review gate` from the lock (SYN-23); `check` warns of the deny rules (SYN-24).

# Approach
1. Declare the feature with the two hooks and the three deny rules.
2. Measure what `sync` does today with an installed gate once its entries are left out, then specify its removal
   (a gate left alone blocks every commit with no review hook to record one).
3. `check` and the SYN-24 warning quiet when the feature is off (they follow the lock: verify).

# Specs
* `specs/features.md` (the uninstall), `specs/review-gate.md` and `specs/sync.md` SYN-22 to SYN-24 if they change.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* `crates/cardinal/src/gate.rs`, `docs/gates.md`, ADR 0004, ADR 0005.

# Hints
* A chained hook of the user (SYN-19) must be restored, not deleted, when the gate goes.

# Tracking

Plan: validated

## Understanding
A feature `review-gate` (the two `review-gate` hooks, the three deny rules of the waiver, `cardinal-review.just`) a
project turns off; `sync` then removes the native gate it installed and gives a chained `pre-commit` of the user back.
Out of scope: the refusals of the `check` hook kept apart (REV-12, REV-21 go with it), rewording the skills.
Measured today: with the entries left out, `gate::plan` returns nothing, so an installed gate stays and refuses every
commit (nothing records a review any more).

## Plan
1. [x] Uninstall: `sync` removes a marked `pre-commit` when the source, as turned on, manages no gate, and moves
   `pre-commit.user` back (dry-run reports, writes nothing); verified by tests of `tests/gate_cli.rs`; makes true SYN-60.
2. [x] Feature: declare `review-gate` in `features.toml`, docs (`docs/gates.md`, `specs/README`-level mentions);
   verified by a test of the declared feature and a `sync`/`check` test with it off (no missing gate, no deny-rule
   warning, gate back once on again); makes true FEA-23, FEA-24, FEA-25.

## Decisions
- Mode: autonomous
- Base commit: 811286c97808adde7c8ea2560f766e83453c472e
- The `check` hook goes with the feature (REV-12, REV-21 with it).
- A marked gate is removed whether up to date, edited or of an older revision.

## Deliverable 1

done: `sync` removes a marked `pre-commit` the source no longer manages and moves `pre-commit.user` back.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | `just check-specs` clean on SYN-60 |
| docs | `docs/gates.md`, `docs/architecture.md` |
| code review | `/code-review low` |

red: SYN-60: `sync_removes_the_gate_once_the_source_no_longer_manages_it` failed with `left: None, right: Some("removed")`
(the gate stayed in place).

Code review: 1 findings
- An edited gate is removed (`State::Modified`): kept, as decided at validation; the install already rewrites an
  edited gate (SYN-20), its script says `do not edit`, and the gates of older revisions read as edited.

not verified: none. decisions needed: none.

## Deliverable 2

done: `features.toml` declares `review-gate`; `check` and the SYN-24 warnings follow the lock, as the issue expected:
no code change for them. Specs `sync.md` and `features.md` accepted.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | traced, accepted |
| docs | README, `docs/usage.md`, `docs/gates.md`, `docs/architecture.md` |
| code review | `/code-review low` |

red: FEA-23: `the_review_gate_feature_is_its_hooks_deny_rules_and_waiver_recipe` panicked on `features.toml declares no
review-gate`; FEA-24, FEA-25: none, their test passed at once (the checks follow the lock).

Code review: no findings

not verified: none. decisions needed: none.

## Deliverable 3: docs and `high` review fixes

done: `docs/usage.md` says `sync` removes the gate; the gate is removed after the lock is written and only when that
lock records no gate (settings left alone keep it); `gate::Plan` is an enum; tests cover the older mark, settings left
alone, and read the deny rules from the entries; `specs/sync.md` scenario and non-goal (shared hooks of worktrees).

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | traced |
| docs | `docs/usage.md`, `specs/sync.md` |
| code review | `/code-review high` on the issue, then `/code-review low` |

red: SYN-60: none, the test of settings left alone was written with the fix.

Code review (`high`): 10 findings
1. `rule-breaches` still applies the gate's rules: outside the requirements, issue
   `issues/done/quiet_the_review_rules_of_rule_breaches_when_the_gate_is_off.md`.
2. Gate removed while the lock of left-alone settings records it: fixed (removal judged on the lock written).
3. Worktrees share the hooks: not fixed, as the install already does; non-goal in `specs/sync.md`.
4. Removal before a failing write: fixed (removed after the lock is written).
5. A symlink restored absolute: not fixed, the install already made it absolute; SYN-60 says the target it reaches.
6. Three `git` calls per sync with no gate: not fixed, negligible next to a sync.
7. `Plan` with dead fields: fixed (an enum).
8. Dangling `pre-commit.user` at install: pre-existing, issue `issues/done/guard_a_dangling_chained_hook_at_the_gate_install.md`.
9. The older mark untested: fixed.
10. The deny rules hardcoded in the test: fixed (read from the entries).

Code review (`low`): no findings

not verified: none. decisions needed: none.

Done: every deliverable committed; nothing left in this issue (follow-ups filed above).
