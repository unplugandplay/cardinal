# Read the tests of a project whatever its layout

# Goal
`check-specs` and `spec-coverage` read the tests of `crates/` and `.claude/` only, the layout of this repository. A
consumer whose crate sits at its root (`src/`, `tests/`) has its `// spec:` citations ignored, so an `accepted` spec of
it looks untraced, and `spec-coverage` reports no test file, which hides every capability worth a spec. Read the
tests of any layout, or say which directories hold them.

# Findings (high review of `bootstrap_specs_for_an_existing_capability`, 2026-10-28)
* `check_specs::test_files` lists the `.rs` files under `<root>/crates` and `<root>/.claude` only, through
  `repo_files` (ADR 0009).
* `spec-coverage` reuses it: in a root holding `src/lib.rs` and `tests/cli.rs` with `#[test]`s, it prints
  `no test file under crates/ or .claude/` (COV-9 states it, a non-goal of `specs/spec-coverage.md` keeps the limit).
* The managed tests under `.claude/` must stay read whatever the layout.

# Approach
1. Reproduce with a test through the binary: a root with `src/` and `tests/` at its top, citations in both.
2. Candidates (can be combined):
   1. Read every `.rs` file git lists, the nested worktrees already left out by `repo_files`.
   2. A `test_dirs` key of `.claude/harness.toml`, defaulting to `crates` and `.claude`.
3. Keep the split between the tests under a `src` directory and the others (TRC) right for a root `src/`.

# Specs
* `specs/spec-tracing.md` (the files read, TRC-5), `specs/spec-coverage.md` (COV-1, COV-9 and the non-goal).

# Resources
* `crates/harness/src/tools/check_specs.rs` (`test_files`), `crates/harness/src/tools/spec_coverage.rs`,
  `crates/harness/src/tools/repo_files.rs`, `docs/adr/0009-scanners-list-the-repository-files-through-git.md`,
  `seed/.claude/harness.toml`

# Hints
* Reading every `.rs` file also reads the build scripts and the examples: a `// spec:` there must still sit above a
  test `fn`.

# Tracking

Plan: validated

## Understanding
`check-specs` and `spec-coverage` read every `.rs` file of the root (git's list, or the disk outside git), except
under `.git`, `node_modules`, a `target` beside a `Cargo.toml` and the `skipped_dirs` of `.claude/harness.toml`, so a crate at the root is
traced and reported. Means: candidate 2.1 of the issue (no `test_dirs` key); `skipped_dirs` is the existing escape.

## Plan
1. [x] Read the tests of any layout: `check_specs::test_files` lists every `.rs` file of the root minus the skipped
   directories, `spec-coverage` prints `no test file found`; verified by binary tests on a root crate (`src/`,
   `tests/`, `build.rs`, `target/`, `vendor/` skipped) in `tests/check_specs.rs` and `tests/spec_coverage.rs`; makes
   true TRC-9, TRC-10, COV-1, COV-9, COV-11, PAR-16 (reworded).
2. [x] Fixes of the `high` review: no symbolic link followed outside git, a `harness.toml` that does not parse is a
   problem, git filters the `.rs` files, the seeded comment and a non-goal of TRC say which `target` is skipped;
   verified by `tests/check_specs.rs`; makes true TRC-9, TRC-10.

## Decisions
- Mode: autonomous
- Base commit: 083f6e21e633cbba5483d953425f866be5d04ad7
- COV-9 prints `no test file found`.
- `skipped_dirs` applies in a git repository too, not only on the disk walk.
- A `target` directory is skipped only beside a `Cargo.toml`: a test module named `target` (`crates/a/tests/target/`) stays read, as before.

## Deliverable 1
- done: `check-specs` and `spec-coverage` read every `.rs` file of the root, but `.git`, `node_modules`, a `target`
  beside a `Cargo.toml` and the `skipped_dirs`; `spec-coverage` prints `no test file found`; TRC, COV, PAR accepted again.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | `docs/architecture.md`, `files/specs/README.md` (synced) |
| code review | low: no findings |

- red: TRC-9, TRC-10, COV-9, COV-11: `FAIL - a crate at the root is traced`, `FAIL - a citation in a build script
  must sit above a test fn`; spec-coverage printed `no test file under crates/ or .claude/` for a root crate.
- Code review: no findings (low)
- not verified: a real consumer with a root crate (only temp roots).
- decisions needed: none.

## Deliverable 2
- done: fixes of the `high` review of `083f6e2..8d7766f`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | `seed/.claude/harness.toml` comment, TRC non-goal |
| code review | low: no findings |

- red: TRC-9, TRC-10: `crates/up/ext/x/bad.rs: cites RER-9, which no spec defines` (a link cycle followed outside git);
  the unparsable `harness.toml` passed silently.
- High review, 10 findings: fixed the link walk, the silent config error, the git pathspec, the seed comment, the
  doubled skip, the stale test doc and the Understanding; not fixed: the `close-issue` skip rule differs on purpose
  (a text rewrite, not a test scan); the module `crates/a/tests/target/` is already traced by the case `a test under
  tests/target is traced`; a custom `target-dir` and fixtures became a non-goal of TRC (`skipped_dirs`).
- Code review: no findings (low)
- not verified: none.
- decisions needed: none.
