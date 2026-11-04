# Bootstrap the spec of an existing capability

# Goal
A capability gets a spec only when an issue changes it, so a project that adopts the harness, or an old capability
of this repository, has behaviour no spec states. Give a way to write the spec of an existing capability from its code
and tests, traced to the tests that already prove it.

# Findings
* `specs/` covers the capabilities changed since the spec format existed; `issue-workflow` writes a spec only when
  an issue changes what a capability does.
* `write-spec` starts from an intended change, not from existing behaviour.

# Approach
1. A procedure in `write-spec` (or its own skill) for an existing capability: read the code and its tests, state
   only the behaviour a test already proves, add the `// spec:` citations, list the untested behaviour as a follow-up
   issue rather than a requirement.
2. A report of the code that no `accepted` spec covers (by module or by test file without a citation), to rank the
   capabilities worth a spec.

# Resources
* `files/.claude/skills/write-spec/SKILL.md`, `files/specs/README.md`, `files/.claude/specs/check_specs.rs`

# Hints
* A spec read back from the code states what the code does, bugs included: the user validates it like any spec.

# Tracking

Plan: validated

## Understanding
Give `write-spec` a procedure that reads the spec of an existing capability back from its code and tests, and a
report (`harness run spec-coverage`) of the test files that cite no `accepted` spec, to pick which capability first.
Out of scope: writing the specs of the uncovered capabilities themselves (each is its own issue).

## Plan
1. [x] Report: `harness run spec-coverage` and `just spec-coverage` list the test files with no citation of an
   `accepted` spec, largest first; verified by `crates/harness/tests/spec_coverage.rs` through the binary; makes true
   COV-1 to COV-8 (~350 lines).
2. [x] Procedure: a "Bootstrap from existing code" section in `write-spec` (pick with `just spec-coverage`, state only
   what a test proves, cite the existing tests, untested behaviour becomes a follow-up issue via `skill:create-issue`,
   the user validates it like any spec, bugs included) and its pointer in `specs/README.md`; verified by a
   `REQUIRED_PASSAGES` entry of `crates/harness/tests/skills.rs` (~60 lines).
3. [x] Fixes of the high review: test attributes such as `#[tokio::test]`, withdrawn requirements, a distinct
   message when no test file is found, the git failure stated, the citation reader of `check-specs` reused; verified
   by `crates/harness/tests/spec_coverage.rs`; makes true COV-1, COV-3, COV-7, COV-9, COV-10.

## Decisions
- Mode: autonomous
- Base commit: b8b64ce20b96efc0295a57f8ded19701dba2c991
- A test file counts as covered when it cites at least one requirement of an `accepted` spec (cases grouped in one `#[test]` would otherwise be reported).
- The report is its own tool, `spec-coverage`, reusing the citation reader of `check-specs`: a check fails, a report exits 0.

## Deliverable 1: report

- done: `harness run spec-coverage` / `just spec-coverage` list the uncited test files, largest first; `check-specs`
  now collects its spec warnings instead of printing them from the parser, so the report reuses it silently.
- checks: tests ok, lint ok, specs ok (COV `accepted`), docs updated (README, architecture, specs/README), code review low ran.
- red: COV-1, COV-3, COV-4, COV-5, COV-6, COV-7, COV-8: `harness: unknown tool `spec-coverage``, exit 3 (COV-2 passed
  before the code: an empty report lists no file).
- not verified: none.
- decisions needed: none.
- Code review: no findings
- Today's report: `skills.rs`, `settings_hygiene.rs`, `audit_sources.rs`, `check_issue.rs`, `src/tests.rs`.

## Deliverable 2: procedure

- done: section "Bootstrap the spec of an existing capability" in `write-spec` (pick with `just spec-coverage`, state
  only what a test proves, bugs included, untested behaviour as a follow-up issue, cite the existing tests), its
  description, and the pointers in `specs/README.md` and `docs/workflow.md`.
- checks: tests ok, lint ok, specs ok, docs updated, code review low ran.
- red: none (no spec: the skill text is proven by two `REQUIRED_PASSAGES`, which failed as missing before the text).
- not verified: how an agent follows the procedure on a real capability (a model's behaviour).
- decisions needed: none.
- Code review: no findings

## Deliverable 3: fixes of the high review

- done: COV-1, COV-3, COV-7 amended, COV-9 and COV-10 added; the report reuses `check_specs::citations` (now fed the
  listed files) instead of its own scan.
- checks: tests ok, lint ok, specs ok (COV back to `accepted`), docs updated (specs/README), code review low ran.
- red: COV-1, COV-3, COV-9: `#[tokio::test]` file and withdrawn citation not listed, `no test file without an accepted
  spec` printed for a root with no test (COV-10 passed before the code: the behaviour existed, now stated).
- High review, 7 findings: 1, 2, 4, 5, 6 fixed (above); 3 (a consumer whose crate sits at its root is not read, a
  limit shared with `check-specs`) filed as `issues/done/read_the_tests_of_any_project_layout.md`; 7 (spec problems
  ignored) not fixed: `check-specs` reports them, the report does not validate (non-goal added).
- not verified: none.
- decisions needed: none.
- Code review: no findings
