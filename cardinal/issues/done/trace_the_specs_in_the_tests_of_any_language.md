# Trace the specs in the tests of any language

# Goal
`check-specs` reads the `// spec: ID` citations only in `.rs` files, so in a project whose tests are in Python,
TypeScript, Go or shell, every requirement reads as untraced and the traceability gate of `just check` cannot be
met. Read the citations in the test files of the languages a project declares, with the comment syntax of each, so
that specs, traceability and the `traced` columns of `delivered-quality` work on any stack.

# Findings
* TRC-9 (`specs/spec-tracing.md`): the checker reads "the citations of every `.rs` file under the root";
  `crates/cardinal/src/tools/check_specs.rs` lists the files with `repo_files(root, &["*.rs"])` and filters on the
  `rs` extension, then keeps the lines whose trimmed text starts with `// spec:`.
* TRC-9 also skips a `target` directory beside a `Cargo.toml`: the build output of Rust only; other stacks have
  theirs (`node_modules` is skipped, `dist`, `build`, `.venv`, `__pycache__` are not).
* QUA-11 (`specs/delivered-quality.md`) counts `traced` from the added lines starting with `// spec:` in the files
  of an issue, whatever their extension: a `#` or `--` comment is never read.
* Every consumer measured on 2026-10-29 is a Rust project; no other stack has been tried.

# Approach
1. Declare the test languages in `.claude/cardinal.toml` (e.g. `spec_files = ["*.py", "*.ts"]`), the default staying
   `*.rs` so that a Rust project changes nothing.
2. Accept the citation after any common line-comment marker (`//`, `#`, `--`, `;`), or let the project give the
   marker per file pattern; the id list and the `(internal: <reason>)` of TRC-6 to TRC-8 stay the same.
3. Generalize the boundary rule of TRC-1 to TRC-5 (a test under `src` is internal) or make the internal directories
   a value, since other stacks keep their tests elsewhere (`tests/`, `__tests__/`, `*_test.go` beside the code).
4. Read the citations of QUA-11 with the same function, and check whether `spec-coverage` lists the test files the
   same way.

# Specs
* `specs/spec-tracing.md` (TRC-1, TRC-5, TRC-9, TRC-10), `specs/delivered-quality.md` (QUA-11),
  `specs/spec-coverage.md` if its file list changes; `files/specs/README.md` documents the citation form.

# Resources
* `crates/cardinal/src/tools/check_specs.rs`, `delivered_quality.rs`, `spec_coverage.rs`, `repo_files.rs`.
* `docs/architecture.md` ("What is shared, and how": project values go through `.claude/cardinal.toml`).

# Hints
* A new key read by an older `cardinal` is ignored, not misread; check whether it still needs `cardinal-format`
  (ADR 0014).

# Tracking

Plan: validated

## Understanding
Let a project declare its test files (`spec_files`, default `*.rs`) and read their citations with the comment marker
of each extension, in `check-specs`, `spec-coverage` and `delivered-quality`; make the internal directories of the
boundary rule a value (`internal_dirs`, default `src`). Out of scope: block comments, the build output of other stacks.

## Plan
1. [x] Test files and markers of `check-specs`: `spec_files` patterns, marker by extension, the placement rule
   outside `.rs`, a bad pattern reported; `spec-coverage` counts a non-`.rs` file as a test; `files/specs/README.md`
   and `seed/.claude/cardinal.toml` document them; verified by tests through the binary; makes true TRC-11, TRC-12,
   TRC-13, TRC-15, COV-12.
2. [x] `traced` of `delivered-quality` reads the citations with the marker of each file's extension; verified by a
   test on a fixture repository; makes true QUA-13.
3. [x] `internal_dirs` replaces `src` in the boundary rule when set; documented in the README and the seed; verified
   by a test through the binary; makes true TRC-14, and sets TRC, COV and QUA back to `accepted`.
4. [x] Fixes of the `high` review of the issue; verified by tests; makes true TRC-11, TRC-12, TRC-16, QUA-13, COV-8.

## Decisions
- Mode: autonomous
- Base commit: 0bea4593b300242fe33eea66165affbed59f72f7
- No `cardinal-format` raise: the new keys are ignored by an older `cardinal`, and the managed content holds no
  citation outside `//` (ADR 0014).
- Markers: the one of the file's extension, the four for an unknown extension (question 1 a).
- After a citation outside `.rs`: any code line past the comments and the `@` decorators (question 2 a).
- `internal_dirs` in this issue, deliverable 3 (question 3 a).

## Deliverable 1
- done: `check-specs` reads the `spec_files` patterns with the marker of each extension, checks the placement outside
  `.rs` and reports a pattern holding a path separator; `spec-coverage` counts a declared non-`.rs` file as a test.
- red: TRC-11, TRC-12, TRC-13, TRC-15: `FAIL - the spec_files are read instead of the .rs files` (8 cases); COV-12:
  `left: (Some(0), "no test file found\n")`.
- Code review: no findings
- not verified: a real consumer in another language.
- decisions needed: none.

## Deliverable 2
- done: `traced` of `delivered-quality` reads each diff line with the citation markers of its file (old path for a
  removed line, new path for an added one); QUA-11 points to QUA-13 for the citation lines.
- red: QUA-13: `FAIL - a citation is read with the marker of its file's extension`.
- Code review: no findings
- not verified: none.
- decisions needed: none.

## Deliverable 3
- done: `internal_dirs` replaces `src` in the boundary rule and its warning names them; README, seed and
  architecture document the key; TRC, COV and QUA are `accepted` again.
- red: TRC-14: `FAIL - a test under an internal_dirs entry is internal, one under src is not`.
- Code review: 1 finding: an `internal_dirs` entry holding a `/` never matched; fixed, reported as a problem
  (TRC-15 extended).
- not verified: the red run of the TRC-15 case added by the fix.
- decisions needed: none.

## Deliverable 4
- done: fixes of the `high` review (10 findings): `traced` reads only the test files of `spec_files`, with the
  marker of each file, a header path ending in a tab or quoted read right; `spec_files` and `internal_dirs` that are
  not lists of strings are problems (TRC-16), checked by `check-specs` only, so `spec-coverage` no longer fails on
  them (COV-8); every pattern character but `*` is literal (all files listed, then matched); `;;` accepted (TRC-12);
  markers computed once per file, `.RS` read as Rust; the hints no longer name `//`; one `config_names` helper for
  both keys; the `//`-in-`.py` case now fails when `//` is read.
- red: TRC-12, TRC-16, QUA-13, COV-8: `FAIL - a ;; line of a Lisp file is a citation`, `FAIL - a key that is not a
  list of strings is a problem`, `FAIL - a citation is read with the marker of its file's extension`,
  `left: (Some(1), "")`.
- Code review: no findings
- not verified: the wildcard case passed before the fix outside git (the fixture is not a git repository).
- decisions needed: none.
