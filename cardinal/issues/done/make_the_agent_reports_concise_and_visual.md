# Make the agent reports concise and visual

# Goal
The skills make the agent end its work with long prose reports. Replace them with short, scannable output (tables,
checklists, one line per fact) and remove the other noise the skills ask the agent to produce, so the user reads
the state of the work at a glance.

# Findings
* Several skills already ask for a table: `prioritize-issues` step 5, `audit-instructions` step 4,
  `update-docs` step 6.
* Prose summaries are still requested elsewhere, with no format:
  * `issue-workflow` step 6 (human review summary: done, verified, not verified, decisions, `Code review:` line),
    step 0 (summary of where a resumed issue stands) and the final summary after the last deliverable.
  * `create-issue` step 6 ("a two-line summary").
  * `commit-message` and `write-spec` give no output format.
* Other noise to measure: restating the issue in step 1, repeating the plan, narrating each step.
* The size of real reports has not been measured yet.

# Approach
Measure first: read recent session transcripts for the final reports (length, repeated content) to see which
ones are the longest and what the user actually acts on.
Then, candidates that can be combined:
1. A shared output rule in `AGENTS.md` (or one small shared document the skills cite): facts in a table or
   checklist, no narration of steps, no repetition of what the diff or the issue already shows, prose only for
   a decision the user must take.
2. A fixed template for the step 6 review stop: a table `check | result` (tests, lint, specs, docs, review
   findings) plus a short list `not verified` and `decisions needed`.
3. The same treatment for the step 0 resume summary, the final summary, and `create-issue` step 6.
4. Drop instructions that only produce narration (restating the issue when it is already clear, announcing
   each step).

# Specs
* `specs/agent-reports.md`: new spec `RPT` (RPT-1 to RPT-7), validated.

# Resources
* `files/.claude/skills/*/SKILL.md`, `files/AGENTS.md`
* `skill:prioritize-issues` and `skill:update-docs`: existing table formats to reuse
* Session transcripts of the project (for the measure)

# Hints
* Keep every validation stop and what it must show (`what was not verified`, the `Code review:` line): the
  hook `require_code_review.rs` and the process rely on them. Shorten the form, not the content.
* The conversation language stays the `user_language` of `.claude/harness.toml`; table headers follow it.
* Related issue: `drop_the_thread_rename_instructions` touches the same `issue-workflow` step 0; do it first.

# Tracking

1. [x] Shared output rule in `AGENTS.md`, cited by the 7 skills, narration and plan repetition dropped (RPT-1, RPT-2, RPT-6). Verified: `just check`, `just test-harness`, `just check-harness`; not verified: that the agent's real reports got shorter (measured after deliverable 3); RPT-2 "does not restate" only catches the four exact rule phrases. Code review: loose substring matches of the RPT-1, RPT-2 and RPT-6 checks fixed (exact rule phrases, case-folded, narrower narration phrases); not fixed: the spec stays `draft` until RPT-3, 4, 5 and 7 are traced (deliverables 2 and 3), and the heading coupling to `AGENTS.md` is kept since `AGENTS.md` is a managed file.
2. [x] Fixed template of the step 4.6 review stop (RPT-3, RPT-7). Verified: `just check`, `just test-harness`, `just check-harness`; not verified: that agents follow the table in practice. Code review: the layout dropped "what was done", now a `done` line (also named in the Lifted bullet, which no longer says "same layout"); a missing step 6 now reports itself; not fixed: the test only checks that the markers are named (the layout itself is prose).
3. [x] Templates of the resume summary, the final summary, `create-issue` step 6 and the step 1 reformulation (RPT-4, RPT-5). Verified: `just check`, `just test-harness`, `just check-harness`; not verified: real reports got shorter (no new transcript to measure). Code review: the "nothing else" of `create-issue` contradicted the offer to start the workflow, now "no other recap"; the resume lines now name the recorded mode and the deliverables left; not fixed: the step 1 layout shows the means only when they differ from the issue's (the goal still is restated for validation), and the test matches literal phrases (as `REQUIRED_PASSAGES` does).

## Decisions

* Mode: autonomous.
* The rule lives in `AGENTS.md`; no numeric length limit.
* Tests: `REQUIRED_PASSAGES` and new checks in `files/.claude/skills/test_skills.rs`, tagged `// spec: RPT-n`.
* Measure: 347 final agent messages, 130 words on average; the longest (530 to 740 words) are step 1 reformulations, step 4.6 summaries and audit reports.
