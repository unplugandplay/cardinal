---
priority: low
why: 'each case needs a dangling link or a race, rare, but the install then loses or half-moves a hook of the user'
---
# Keep the gate install unchanged on failure

# Goal
SYN-19 promises the install over a user's `pre-commit` never overwrites it and fails changing nothing. Three paths
of `gate::install` break it: a failure there leaves the staged hook behind, or loses the user's hook. Expected: any
failure of the install leaves the hooks directory as it found it, and the move never replaces a `pre-commit.user`.

# Findings
* Found by the `high` review of `guard_a_dangling_chained_hook_at_the_gate_install`, in
  `crates/cardinal/src/gate.rs`, `install`:
* A dangling user `pre-commit` link is `Foreign`: `install` writes `pre-commit.cardinal-new`, then
  `fs::canonicalize(&hook)` fails with `No such file or directory`, the staged file left in the hooks directory.
* In the symlink branch, `fs::remove_file(&hook)` runs before `symlink(&target, &chained)`: when the link fails, the
  user's link is gone and no gate is installed.
* `fs::rename(&hook, &chained)` replaces a `pre-commit.user` created between `plan` and `apply` without an error.

# Approach
1. Write the failing test of SYN-19 with a dangling user `pre-commit` (refuse it in `plan`, or move the link as a
   link).
2. Create the new link before removing the old one.
3. Refuse at the move rather than only in `plan`: `hard_link` then `remove_file`, or `renameat2(RENAME_NOREPLACE)`;
   remove the staged file on any error.

# Specs
* `specs/sync.md` SYN-19 (scenarios).

# Resources
* `crates/cardinal/src/gate.rs`, `crates/cardinal/tests/gate_cli.rs`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
Make `gate::install` leave the hooks directory as it found it on any failure, and never replace a `pre-commit.user`.
Means: the user's `pre-commit` moves in the same directory, so a link moves as a link with its relative target intact:
`hard_link` (which links the link itself) then `remove_file` replaces both `canonicalize` + `symlink` and `rename`;
a dangling link then moves like any link (the gate skips a `pre-commit.user` that is not executable, as git did).

## Plan
1. [x] Move the user's hook without replacing, and clean up on failure: `hard_link` + `remove_file` for the move
   (fails on an existing `pre-commit.user`), the staged file removed on any error; verified by a `gate_cli.rs` test
   with a dangling user `pre-commit` link (moved as a link, no staged file) and an internal test of `install` with a
   `pre-commit.user` created after `plan` (error, hooks directory unchanged); makes true SYN-19. (autonomous)

2. [x] Fixes of the `high` review: no `remove_file` between the link and the rename, a rename where the file system
   makes no hard links, a `pre-commit` written since `plan` never replaced, the refusal names both files; verified by
   the internal tests of `gate.rs`; makes true SYN-19. (autonomous)

## Decisions
- Mode: autonomous
- 1 a: a dangling user `pre-commit` link moves as a link.
- Base commit: 08392896ab189f58ba9d93d75124da6cc62705e5

## Summary of deliverable 1
done: the user's `pre-commit` moves by `hard_link` + `remove_file` (a link as a link, its relative target kept), the
staged hook and a half-made move are undone on any error (the `Modified` gate is replaced by one rename); the SYN-60 test reads the target the moved link reaches.
red: SYN-19: `sync` exited 3 with `No such file or directory (os error 2)` on a dangling user link; the internal test
saw `install` succeed over a `pre-commit.user`.
Code review: 1 findings
- the final rename did not undo the move: fixed, the user's hook goes back.
not verified: a failure of `remove_file` or of the final rename (no cheap way to stage it).

## Summary of deliverable 2
done: the move links then renames over the user's hook (one undo: the link), falls back to a rename where hard
links fail, refuses a `pre-commit` written since `plan` (Missing), and both refusals say "merge them by hand".
red: SYN-19: `install` with state `Missing` replaced a `pre-commit` written since the plan.
Code review: 7 findings (`high`)
- no hard links on the file system: fixed, rename fallback with `to` checked absent.
- undo of `remove_file` failing: fixed, the step is gone.
- `pre-commit` written since `plan` replaced: fixed for `Missing`; `Modified` replaces our own gate, left.
- extra `remove_file` window: fixed.
- bare io errors: fixed for the refusal; other io errors keep std's message.
- rollback error dropped: not fixed, the error returned is the cause; the next `sync` repairs a missing gate.
- `inspect_err`: applied.
- low review: an error after the gate was linked in place: fixed, the leftover staged name is ignored.
not verified: the fallback without hard links (no such file system in the tests).
