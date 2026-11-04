# Align the end of the Tracking section in QUA-6 with the code

# Goal
QUA-6 of `specs/delivered-quality.md` says the `# Tracking` section ends "as in QUA-3", that is at the next line
starting with `#`, which would stop at `## Plan` and count no `Code review` line under `## Deliverable n`. The code
(`reviews` in `crates/harness/src/tools/delivered_quality.rs`) ends it at the next line starting with `# `. A test
written from the spec would fail against the code: make the wording say what the code does.

# Findings
* Found by the `high` review of `issues/done/check_the_red_run_of_traced_tests.md`; QUA-11 already says "ending at the next
  line starting with `# `".

# Approach
1. Amend QUA-6 to end the section at the next line starting with `# ` outside a fenced code block, as QUA-11.
2. Check that a test of QUA-6 has a `Code review` line under a `##` sub-heading (the fixture `ALPHA` has one).

# Specs
* `specs/delivered-quality.md`: amends QUA-6.

# Resources
* `specs/delivered-quality.md` (QUA-3, QUA-6, QUA-11), `crates/harness/src/tools/delivered_quality.rs` (`section`,
  `reviews`), `crates/harness/tests/delivered_quality.rs`

# Tracking

Plan: validated

## Understanding
Amend the wording of QUA-6 so the `# Tracking` section ends at the next line starting with `# ` (as the code and
QUA-11 do), not at any `#` line. Out of scope: any change of the code. The existing QUA-6 test already covers it.

## Plan
1. [x] Amend QUA-6: end the section at the next `# ` line outside a fence; verified by `just check` and the
   existing QUA-6 test (fixture `ALPHA` has its `Code review` lines under `## Deliverable 1`); makes true QUA-6.
2. [x] Fixes of the `high` review: the fixture `ALPHA` gets a `Code review` line under a `# ` heading after the
   Tracking section; verified by the QUA-6 test; makes true QUA-6.

## Decisions
- Mode: autonomous
- Base commit: `2abdb79c41abfc003244ca62f4284b8c9661ea24`

## Deliverable 1
- done: QUA-6 ends the `# Tracking` section at the next line starting with `# ` outside a fence, as the code and QUA-11.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (`just check-specs`) |
| docs | no other text cites the old ending |
| code review | low, see below |

- red: none (wording only; the existing QUA-6 test already passes against the code)
- Code review: 0 findings
- not verified: nothing
- decisions needed: none

## High review
`high` review of `2abdb79..7807790`: 6 findings.
1. No test of the `# ` end of the section: fixed in deliverable 2.
2. `~~~` and indented fences are not read as fences; 3. QUA-11 has no fence clause; 4. the heading search ignores
   fences: outside the requirements of this issue, opened as
   `issues/done/read_the_fences_of_the_issue_sections_as_the_specs_say.md`.
5. QUA-3 and QUA-6 end their sections differently: dismissed, on purpose (`## Plan` ends at `## Decisions`, the
   Tracking reviews span its `##` sub-headings).
6. The issue is not closed yet: dismissed, closing is the user's call.

## Deliverable 2
- done: `ALPHA` ends with `# Notes` holding `Code review: 4 findings`, which QUA-6 must not count.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | no change needed |
| code review | low, see below |

- red: QUA-6: FAIL - each Code review line of the Tracking counts a review; FAIL - the findings of the reviews are
  summed (against `reviews` with the `# ` end removed; the code already ends there)
- Code review: 0 findings
- not verified: nothing
- decisions needed: none
