---
priority: medium
why: 'the features of skills that call one another cannot be declared safely without it'
---
# Let a feature need another

# Goal
`issue-workflow` calls `skill:commit-message` and `skill:update-docs`; `create-issue` calls `commit-message`. A
project that turns off a feature another one left on needs is left with a skill that names a missing one. The pilot
left this out (a non-goal of `specs/features.md`). The outcome: a feature declares the features it needs, and `sync`
refuses a `disabled` list that turns off a needed one, naming both.

# Findings
* `features.toml` holds `files` and `entries` only; any other key fails `sync` (FEA-7).

# Approach
1. A `needs` list per feature in `features.toml`, read by `features::left_out`; an unknown name in it fails `sync`
   like FEA-6.
2. `sync` fails (exit 3, writing nothing) when `disabled` turns off a feature that one left on needs, naming both.
3. Amend the non-goal of `specs/features.md` into requirements.

# Specs
* `specs/features.md`: new FEA-n for `needs`; its non-goal on dependencies withdrawn.

# Milestone
Optional features

# Resources
* `specs/features.md`, ADR 0025, `features.toml`, `crates/cardinal/src/features.rs`.
* `issues/done/let_a_project_turn_off_a_feature_of_the_harness.md` (the pilot `no-ai-attribution`).

# Hints
* A cycle in `needs` is harmless for this check (each feature is judged against the ones left on), but say so in the
  spec.

# Tracking

Plan: validated

## Understanding
A feature of `features.toml` lists in `needs` the features it needs; `sync` refuses an unknown name there, and a
`disabled` list that turns off a feature one left on needs (exit 3, writing nothing). Out of scope: declaring `needs`
in Cardinal's own `features.toml` (no feature of it needs another yet; the skill features come with their issues), so
no `cardinal-format` raise: an older `cardinal` refuses the key (FEA-7) rather than misreading it.

## Plan
1. [x] `needs` in `features::left_out`: parse it (FEA-7 shape), refuse an unknown name, refuse a needed feature
   turned off, spec amended and `docs/architecture.md` updated; verified by binary tests in
   `crates/cardinal/tests/features_cli.rs` (cycle and self-need included); makes true FEA-17, FEA-18.

## Decisions
- Mode: autonomous
- Base commit: d6dd9e9f04e90e8bf1a6c48e091cff16b62f5fd1

## Deliverable 1
done: `needs` in `features.toml`, read by `features::left_out`; `sync` refuses an unknown needed feature and a
`disabled` that turns off a feature one left on needs; spec accepted, usage and architecture docs updated.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | FEA-17, FEA-18 traced, spec accepted |
| docs | `docs/usage.md`, `docs/architecture.md`, `features.toml` header |
| code review | see below |

red: FEA-17, FEA-18: `features.toml: a.needs is not \`files\` nor \`entries\``
Code review: no findings

not verified: none. decisions needed: none.

## Whole-issue review (high, d6dd9e9..3465376)
done: ADR 0025 says how `needs` is handled and when it raises the format; the FEA-17 error says "which the source
does not declare"; the doc comment of `of_source` cites FEA-17.

Code review: 8 findings
- fixed: ADR 0025 stale; format raise when `needs` is first used recorded in ADR 0025; FEA-17 wording; doc comment.
- not fixed: `turned_off` swallowing `left_out` errors (pre-existing, FEA-5 alike; installing the named `cardinal`
  then shows the real error); one error per `sync` for a chain of needs (FEA-18 names the direct pair, enough to fix
  the list); binary checks reading `disabled` without `needs` (FEA-8 reads the configuration by design, `sync`
  refuses the list); `BTreeSet` for `disabled` (its order gives the order of the FEA-5 error).

red: none
Code review (low): no findings
