# Tell the agent to ask the user when the review waiver cannot be stamped

# Goal
When the native commit gate cannot stamp an empty waiver, it refuses the commit with the same instruction as every
other refusal: run `/code-review low`. No review clears that refusal, so the agent loops on reviews that cost tokens
while the commit stays refused. That refusal should tell the agent to ask the user to fix the waiver instead.

# Findings
* `files/.claude/hooks/require_code_review.rs`, `refusal()`: an empty waiver whose write fails returns
  "the waiver could not be stamped" before the marker is read, so a fresh review changes nothing.
* `refuse()` prints one instruction for every reason: "Run /code-review low on the current diff with the Skill
  tool (issue-workflow, step 4.5), fix what it finds, then commit."
* Found by the end-of-issue `high` review of `name_the_review_level_in_the_gate_message`.

# Approach
1. Give `refuse()` the instruction with the reason (or a second refusal function, as `refuse_skipped_hook` does),
   so the waiver failure says to ask the user to fix the waiver file and its directory permissions.
2. Amend REV-10 of `specs/review-gate.md` ("a message telling the agent to run the code review"): the message
   names the level of a deliverable (`/code-review low`, which the tests already pin since
   `name_the_review_level_in_the_gate_message`), and the waiver failure tells the agent to ask the user instead.

# Specs
* `specs/review-gate.md`: REV-10 amended (review level in the message, waiver failure).

# Resources
* `files/.claude/hooks/require_code_review.rs` (`refusal()`, `refuse()`)
* `files/.claude/hooks/test_require_code_review.rs`
* `specs/review-gate.md`, `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md`

# Tracking

1. [x] REV-10 amended; the refusal of a waiver that cannot be stamped tells the agent to ask the user (test with a
   read-only empty waiver).
2. [x] Fixes of the `high` review: a waiver that cannot be stamped is ignored (the review alone judges the commit) and
   a refusal names it by its path; the message no longer blames the directory; the test copes with root; `refuse()`
   doc comment; named fields instead of a pair.

## Decisions

* Mode: autonomous.
* Base commit: 5b53329960740759631c0700c9cff8eb8261691b.
* One deliverable; `refusal()` returns the instruction with the reason, `refuse_skipped_hook` unchanged.
* After the `high` review (user's call): a waiver that cannot be stamped no longer refuses on its own; `native` judges
  by the review and the refusal adds the ask-the-user line with the path (REV-10 amended again). The permanent bypass
  of an unreadable waiver becomes its own issue.

## Summary of deliverable 1

* done: `refusal()` returns the instruction with the reason; a waiver that cannot be stamped tells the agent to ask the
  user to fix the waiver file and its directory permissions; REV-10 amended; `docs/architecture.md` says so.
* checks: `just check` ok, `just test-harness` ok, `just check-harness` ok, code review `low`: no findings.
* red: `FAIL - that refusal tells the agent to ask the user, not to run a review`.
* not verified: none.
* decisions needed: none.

## Summary of deliverable 2

* done: a waiver that cannot be stamped waives nothing (the review judges the commit) and a refusal names it by its
  path; `Refusal` struct instead of a pair; `refuse()` doc comment fixed; the test skips the case under root; REV-10
  and `docs/architecture.md` updated; follow-up `issues/done/refuse_a_review_waiver_that_cannot_be_read.md` (high review
  finding 4).
* checks: `just check` ok, `just test-harness` ok, `just check-harness` ok, code review `low`: no findings.
* red: `FAIL - native refuses an unreviewed diff whose waiver it cannot stamp` (and the two other new checks).
* not verified: the root branch of the test (no root run).
* decisions needed: none.
