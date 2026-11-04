# Seed no cargo permissions in a project of another stack

# Goal
The seeded `.claude/settings.json` allows `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` and `cargo check`
whatever the stack of the project. A Python or TypeScript project gets permissions it does not use and none for its
own tools, so the agent asks before each test run. Seed only stack-neutral permissions, and let the stack ones come
from the project.

# Findings
* `seed/.claude/settings.json` holds the five `cargo` rules next to the `just` and read-only `git` rules.
* The file is seeded (copied once, then owned by the project); the managed entries of
  `entries/.claude/settings.json` are the ones `sync` keeps (ADR 0007, ADR 0013).
* `just` already wraps the project's commands: an agent that runs `just check` or `just test` needs no `cargo`
  rule.

# Approach
1. Drop the `cargo` rules from the seed and rely on the `just` rules, the project adding its own.
2. Or seed by stack: `cardinal init` detects the stack (`Cargo.toml`, `package.json`, `pyproject.toml`) or takes it
   as an option, and writes the matching rules.
3. Either way, existing projects keep the file they own: say in the docs what to remove by hand, if anything.

# Specs
* `specs/sync.md` for the seeding of `init` (SYN-4) if option 2 is taken.

# Resources
* `seed/.claude/settings.json`, `entries/.claude/settings.json`, `crates/cardinal/tests/settings_hygiene.rs`.
* `docs/adr/0007-sync-manages-entries-of-the-seeded-settings.md`, `docs/adr/0013-sync-manages-the-allow-rules-too.md`.

# Tracking

Plan: validated

## Understanding
The seeded `.claude/settings.json` allows only stack-neutral commands (`just`, read-only `git`); the stack rules
come from the project. Option 1 of the issue: no stack detection in `init`, so no spec change. Out of scope: the
settings of existing projects (owned), which the docs tell what to remove by hand.

## Plan
1. [x] Drop the five `cargo` rules from `seed/.claude/settings.json`; `tests/settings_hygiene.rs` checks that every
   `Bash` allow rule of the seed is a `just` or `git` one; `docs/usage.md` says a project of another stack may drop
   the `cargo` rules it was seeded with and add its own (this repository keeps its own); verified by the hygiene
   test (red with the current seed); ~40 lines.
2. [x] Fixes of the `high` review of the issue: the seed check reports an invalid seed, runs the dangerous and
   machine path checks, and accepts only `Bash(just)`, `Bash(just …)` and the read-only `git` subcommands (any
   tool kind); the docs paragraph names the rules `sync` adds and sits after the command table; verified by the
   hygiene test.

## Decisions
- Mode: autonomous
- Option 1 (drop the `cargo` rules, rely on `just`), no stack detection.
- Base commit: 031c837978832802ff5842623f2316edeb925299

## Summary of deliverable 1
done: the seed allows `just` and read-only `git` only; the hygiene test guards it; `docs/usage.md` says what a
project seeded before removes by hand; `.claude/update-docs-map.md` gains the row of a changed seeded file.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass, no spec touched (option 1) |
| docs | `docs/usage.md`, `.claude/update-docs-map.md` |
| code review | low, see below |

red: none traced (no spec): `FAIL - seeded allowlist has a rule of one stack (only `just` and `git`): Bash(cargo build *)` and the four other `cargo` rules

Code review: no findings (low; the test change was out of its scope).

not verified: none. decisions needed: none.

## Summary of deliverable 2
done: the six findings of the `high` review fixed (one shared rule check for the shared and the seeded
allowlists, strict stack-neutral match, invalid seed reported; docs paragraph corrected and moved).

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass, no spec touched |
| docs | `docs/usage.md` |
| code review | low, see below |

red: none traced (no spec): with `Bash(justify *)`, `Bash(git push *)`, `Bash(git reset *)` and `Read(./target/**)`
added to the seed, the test failed on each; with an invalid seed, `FAIL - seed/.claude/settings.json is not valid JSON`

Code review: no findings (low).

not verified: none. decisions needed: none.
