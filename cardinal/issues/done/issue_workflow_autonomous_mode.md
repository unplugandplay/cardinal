# Make `issue-workflow` autonomous by default after the plan

# Goal
Once the user has validated the plan (step 3), `skill:issue-workflow` chains the deliverables autonomously by
default, without the human review and commit validation stops of step 4.6, and keeps only the stops that
protect the user: anything outward-facing, a blocking question, and the final summary. At the plan validation
the user can instead ask for a validation at each deliverable.

# Findings
* Found by the instruction audit (counts only): about ten messages over ten sessions and four projects ask the
  agent to proceed autonomously "to the end" after validating a plan.
* Today the skill says "never skip a validation stop" and the agent has to be told again each time which
  stops are lifted, and whether the review and the tests still apply.

# Approach
Decided by the user: the autonomous mode is the default after the plan is validated, not something to request.
1. Steps 1 to 3 keep their stops (understanding, spec, plan). The plan validation also asks one question: autonomous
   (default) or validation at each deliverable. The answer is recorded in the issue's `## Decisions`.
2. In autonomous mode, step 4.6 becomes a summary written in the issue (with the `Code review:` line) instead of a
   stop, and step 4.8 commits without waiting; it never lifts the review, `just check`, `just test-harness`, or the
   rules on push, deletion and history rewriting. A blocking question still stops the chain, and the final
   summary (and the question to close the issue) ends it.
3. The mode survives a `/clear` or a resumed thread: it is written in `# Tracking` with the plan.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`

# Hints
* The skill tests (`test_skills.rs`) check the references of a skill: keep them valid.

# Tracking

1. [x] `issue-workflow`: autonomous mode by default after the plan, asked at step 3 and recorded in `# Tracking`; `test_skills.rs` checks the skill documents it

## Decisions

* No spec (a skill, no change in what `harness` does); one deliverable since everything lives in one file.
* Mode: autonomous (the user asked for it when validating the plan of this issue, before the step 3 question existed). Local commits only, no push; a decision needed in a summary stops the chain like a blocking question.
