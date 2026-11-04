# Name the review level in the commit gate's refusal

# Goal
When the commit gate refuses a commit, its message tells the agent to run `/code-review` with no level, so the
review reuses whatever level was typed last. `issue-workflow` now names the level of each review. The refusal should
give the same instruction as the skill.

# Findings
* `files/.claude/hooks/require_code_review.rs`, `refuse()`: "Run /code-review on the current diff with the Skill tool
  (issue-workflow, step 4.5), fix what it finds, then commit."
* `files/.claude/skills/issue-workflow/SKILL.md` step 4.5 now says `/code-review low` per deliverable and "always
  name the level"; a `high` review runs once per issue on `<base>..HEAD`.
* Found by the end-of-issue `high` review of `tighten_the_issue_workflow` (finding outside its deliverables).

# Approach
1. Make the refusal say `/code-review low` (the level of a deliverable), keeping the pointer to step 4.5. No test
   nor spec pins the text today.

# Resources
* `files/.claude/hooks/require_code_review.rs`
* `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)

# Tracking

## Decisions
* Mode: autonomous. Base commit: `aa9a58bfdae82ff34b4998081e91b60ae284dd09`.
* No spec change: REV-10 already asks for a message telling the agent to run the review; the level is its wording.
* The test pins `/code-review low` in the refusal (REV-10 check and the plain refusal check).

1. [x] The refusal names the level `low`.
   * done: `refuse()` says `/code-review low`; the two refusal checks of `test_require_code_review.rs` look for it.
   * check | result: `just check` ok, `just test-harness` ok, specs ok, docs: nothing cites the message, code review: see below.
   * red: `FAIL - native refuses an unreviewed repository with a message naming the review and its level`,
     `FAIL - refuses a commit without any review`.
   * Code review: no findings.
   * not verified: none. decisions needed: none.

## End-of-issue review (`high`, `aa9a58b..HEAD`)
* Waiver-stamp refusal also says to run a review, which cannot clear it: outside this issue, filed as
  `tell_the_agent_to_ask_the_user_when_the_waiver_cannot_be_stamped`.
* `low` hard-coded in the hook, duplicating step 4.5: the decision of this issue; the test pins it so a drift fails.
* REV-10 does not name the level the test now pins: amendment folded into the waiver issue above (user agreed).
* "/code-review low with the Skill tool" may be split badly by an agent: same wording as step 4.5, not changed.
* Approach says no test pins the text: true when written; the Tracking records the change.
