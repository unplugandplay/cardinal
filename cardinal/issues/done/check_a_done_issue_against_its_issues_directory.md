---
priority: low
why: '`check-issue` validates new issues; a done issue is checked only by hand'
---
# Check an issue of issues/done against its issues directory

# Goal
For `issues/done/foo.md`, `check-issue` takes `issues/done` as the issues directory: it always reports
`file must be issues/<snake_case>.md`, and looks for each `depends_on` stem in `issues/done/` and `issues/done/done/`,
so a dependency still open in `issues/` is reported missing. The outcome: either `check-issue` checks a done issue
against `issues/`, or it refuses a file outside `issues/` with one clear message.

# Findings
* `crates/cardinal/src/tools/check_issue.rs`: `front_matter_problems` (the `dir` of the dependencies) and the name check of `problems`.
* Found by the `high` review of `find_the_root_of_a_done_issue`, which fixed only the project root (FEA-9).

# Approach
1. Decide whether `check-issue` accepts a done issue at all (CKI spec, non-goals).
2. If so, take the issues directory as the parent of `done/` for a file in it.

# Specs
* `specs/check-issue.md` CKI-2.

# Resources
* `crates/cardinal/src/tools/check_issue.rs`, `crates/cardinal/tests/check_issue.rs`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
`check-issue` validates a new issue; a done issue always fails it (its `# Tracking`), so it refuses a file of
`issues/done/` with one message (CKI-4) instead of a misleading list. Out of scope: checking a done issue against `issues/`.

## Plan
1. [x] Refuse a done issue: `problems` returns the single CKI-4 problem for a file whose parent is `done` under
   `issues` (path canonicalized, so `a.md` run from `issues/done` is caught); the done-issue part of the FEA-9 test
   becomes a CKI-4 test; verified by `crates/cardinal/tests/check_issue.rs`; makes true CKI-4. (autonomous)

   done: `check-issue` refuses a file of `issues/done/` with the single CKI-4 problem; spec `accepted`.
   red: CKI-4: `issues/done/a.md: file must be `issues/<snake_case>.md`` and `` `depends_on` names `b`: no `b.md` in `issues/done` nor in `issues/done/done` ``
   Code review: no findings
2. [x] Fixes of the `high` review: the directory of the issue is resolved once (`issue_dir`) and serves every
   check, so a new issue named from `issues/` passes; FEA-9 again covers that case and the ban left on; the CKI-4
   message loses its nested backticks; verified by `crates/cardinal/tests/check_issue.rs`. (autonomous)

   done: `high` review of `df52695..HEAD`, 8 findings: 7 fixed; the helper shared with `issue_claim.rs` is not
   (its rule differs: an open issue), left as is.
   red: FEA-9: `a.md: file must be `issues/<snake_case>.md`` for a new issue named from `issues/`
   Code review: no findings

## Decisions
- Mode: autonomous
- Base commit: df52695528b05cac48ea6863bd53ef645feb6ce5
- Question 1: a, `check-issue` refuses a done issue with one message (CKI-4).
