---
priority: low
why: 'only a sync killed mid-install reaches it, but every later sync then fails until the user merges two identical files'
---
# Finish an interrupted gate install

# Goal
A `sync` killed during the gate install can leave `pre-commit` and `pre-commit.user` as two hard links of the same
hook of the user. Every later `sync` then refuses with `both exist`, and `check` asks for a manual merge of two
identical files. Expected: `sync` sees that both names are the same file and finishes the install.

# Findings
* `crates/cardinal/src/gate.rs`, `swap`: for `State::Foreign` it first runs `link_new(hook, chained)` (a hard link),
  then `rename(staged, hook)`. A kill between the two leaves both names on the same inode. `conflicts` and `plan` take
  that for a conflict (found by the `high` review of `name_the_chained_hook_conflict_in_check`).

# Approach
1. Write a failing test with `pre-commit` and `pre-commit.user` hard-linked, then let `conflicts` treat the same
   inode (`std::os::unix::fs::MetadataExt::ino` and `dev`) as an interrupted move: install the gate over `pre-commit`
   and keep `pre-commit.user`.
2. Optionally, extend the same treatment to identical content.

# Specs
* `specs/sync.md` SYN-19 (amend: a `pre-commit.user` that is the same file as `pre-commit` is not a conflict), SYN-23.

# Resources
* `crates/cardinal/src/gate.rs`, `crates/cardinal/tests/gate_cli.rs`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
A `pre-commit.user` that is a hard link of a foreign `pre-commit` (same `dev` and `ino`, `symlink_metadata`) is an
interrupted move: `conflicts` says no, `swap` only renames the staged gate over `pre-commit`. Out of scope: two
separate files of identical content (approach step 2), left a conflict.

## Plan
1. [x] Finish an interrupted install: `conflicts` and `swap` treat a hard-linked `pre-commit.user` as the move done;
   verified by a `gate_cli.rs` test (hard link, `check` says `missing`, `sync` installs, `pre-commit.user` runs after
   the gate); makes true SYN-19, SYN-23 (amended). (autonomous)
2. [x] Fixes of the `high` review: `State::Interrupted`, report, `nlink` guard, staged leftover in the test. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 6133674607ea0e0498a91c9595c1a5d2277e0c85
- Identical content of two separate files: not treated (answer 1 a), still a conflict.

## Deliverable 1
done: `conflicts` and `swap` take a `pre-commit.user` hard-linked to the foreign `pre-commit` for the move a killed
`sync` left half done; `sync` renames the gate over `pre-commit`, `check` reports `missing`, not `conflict`.

| check | result |
|---|---|
| tests | `just check` green (new test in `gate_cli.rs`) |
| lint | fmt, clippy -D warnings: clean |
| specs | SYN-19, SYN-23 amended, traced, spec `accepted` |
| docs | `docs/gates.md` |
| code review | `low`: no findings |

red: SYN-19, SYN-23: `check` printed `conflict: native review gate (… both exist …)`, expected `missing: native review gate`.

Code review: no findings

not verified: the `cfg(not(unix))` fallback (no hard-link check off Unix) is not built here.

## Deliverable 2 (fixes of the `high` review)
done: `State::Interrupted`, decided once by `state` and checked again by `swap`; the report no longer says a move this
`sync` did not do; `same_file` asks for at least two links (inode numbers colliding on some file systems); the test
leaves the staged file a killed `sync` leaves too.

| check | result |
|---|---|
| tests | `just check` green |
| lint | fmt, clippy -D warnings: clean |
| specs | unchanged (SYN-19, SYN-23) |
| docs | unchanged |
| code review | `low`: no findings |

red: SYN-19: `sync` reported `pre-commit -> pre-commit.user moved` on the interrupted install.

`high` review, not fixed: the Windows fallback of `same_file` (releases build only Linux and macOS); an atomic
no-replace rename (`renameat2`, `renamex_np`) that would remove the window, left to the user.

Code review: no findings

not verified: an `nlink` of 1 on a file system that counts no links (the guard then refuses the finish, a conflict as before).
