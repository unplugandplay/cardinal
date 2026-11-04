---
priority: medium
why: 'the cheapest features: entries only, no new mechanism, and each removes a hook call per turn or per edit'
---
# Declare the features made of hooks only

# Goal
`lint-gate` (a Stop hook up to 600 s), `format-code` (after each edit) and `language-hint` (a Stop hook) run in
every consumer. A project whose lint runs in CI, whose editor formats, or that works in English only cannot drop
them. The outcome: three features in `features.toml`, each made of its hook entry, that a project turns off with
`disabled`.

# Findings
* The mechanism handles entries already (FEA-1, FEA-3): a feature of entries only needs no code in `sync`.
* The hooks are declared in `entries/.claude/settings.json`; `docs/architecture.md` (Hooks and their declaration)
  lists them with their spec.

# Approach
1. Declare `lint-gate`, `format-code` and `language-hint` in `features.toml`, one feature each, with their `entries`
   as `sync` prints them.
2. Decide whether rule `user-language` of `rule-breaches` stops applying when `language-hint` is off (the rule is
   still the user's), and amend `specs/features.md` if so.
3. A test of the source that each feature names an entry of the fragment (FEA-6 already fails `sync` otherwise).

# Specs
* `specs/features.md` (FEA-n added for any check of the binary a feature turns off).

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).
* `entries/.claude/settings.json`, `specs/lint-gate.md`, `specs/format-hook.md`, `specs/language-hint.md`.

# Hints
* A feature's entry is matched on the exact `hook <event> <matcher> <command>` text: a renamed command in the fragment
  must be renamed in `features.toml` in the same commit, or `sync` fails (FEA-6).

# Tracking

Plan: validated

## Understanding
Declare `lint-gate`, `format-code` and `language-hint` in `features.toml`, each its single hook entry. No code in
`sync`; rule `user-language` keeps applying when `language-hint` is off (non-goal in `specs/features.md`).

## Plan
1. [x] Hook-only features: the three tables in `features.toml`, a test of the source that each names exactly the hook
   entry of its command, the docs (`docs/architecture.md` hooks table); verified by the source test and a `sync`
   dry-run with the three turned off; makes true FEA-16.
2. [x] Fixes of the `high` review: the `docs/architecture.md` hooks table, the two off switches of the lint gate and
   the wording of `docs/gates.md`; verified by `just check`.

## Decisions
- Mode: autonomous
- Rule `user-language` keeps applying when `language-hint` is off (question 1 a): a non-goal of `specs/features.md`, no code.
- Base commit: 36d9afe9f2be68b3990fac6d10ffe9a30572d4ec

## Deliverable 1

- done: `lint-gate`, `format-code` and `language-hint` declared in `features.toml`; docs in `docs/usage.md`,
  `docs/gates.md`, `README.md` (`docs/architecture.md` added by deliverable 2).
- red: FEA-16: `features.toml declares no lint-gate`.
- Verified by hand: a fresh project with the three in `disabled` gets the three hooks `removed` by `sync`, `check` passes.

## Deliverable 2

- done: the docs findings of the `high` review fixed.
- red: none (docs only).
- Code review (high): 8 findings. Fixed: architecture table, tracking line, the two lint switches, gates wording.
  Not fixed: the test rebuilds the entry text (a drift of `Entry`'s format fails `sync` on this repo through FEA-6, in
  `just check-cardinal`); a matcher renamed without `features.toml` fails `sync` (the issue's Hint, caught here by
  FEA-16); the three names hard-coded (FEA-16 names them); `hinted` user-language breaches turning `final` when
  `language-hint` is off (decision 1 a: the measure counts the rule, the hook off is the project's choice).
