# Write the issue preparation in the files, not in the console

# Goal
Starting an issue with `issue-workflow` costs three validation stops (reformulation, spec, plan), each shown as
prose in the console, then written again in the issue file. In most cases the user only answers "ok". Make the
preparation land in the files (issue, spec, ADR) where it keeps its history, and replace the three stops by one
short stop: a link to the files, a summary of the issue in a few lines, and the open questions only.

# Findings
* Steps 1, 2 and 3 of `files/.claude/skills/issue-workflow/SKILL.md` each end with a STOP; step 1 shows `goal`,
  `out of scope`, `questions`, step 3 shows the plan, then writes it under `# Tracking`. The content is shown in the
  console first and copied in the issue afterwards.
* `skill:write-spec` has its own validation stop (its step 4), so a spec adds a stop of its own.
* User feedback (2026-10-26): the reformulation is too long, it should be a summary in a few lines; most of the
  time the user just validates.

# Approach
1. Merge steps 1 to 3 into a preparation step with no stop: read, write the spec draft (`skill:write-spec` up to its
   format check), write `# Tracking` in the issue with a short `## Understanding`, the plan checklist and
   `## Decisions` (mode, base commit), plus an ADR draft when a decision of architecture is taken.
2. One validation stop: the links to the files written, the issue summed up in two or three lines, and the
   questions, if any, as a table `question | why | impact` with the recommended answer; the details stay in the files.
3. Mark the state of the plan in `# Tracking` (for instance `Plan: proposed` / `Plan: validated`) so a resumed
   thread knows whether the stop was passed.
4. Renumber the deliverable steps and update every reference to the old step numbers (`create-issue`,
   `check_issue.rs` message, `details.md`, `docs/architecture.md` on the model split).

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`, `files/.claude/skills/issue-workflow/details.md`
* `files/.claude/skills/write-spec/SKILL.md` (its validation stop)
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/create-issue/check_issue.rs`
* `files/AGENTS.md`, section "Reports to the user"
* `docs/architecture.md` (review gate step number, model split by step)
* `issues/done/tighten_the_issue_workflow.md`, `issues/done/make_the_agent_reports_concise_and_visual.md`

# Hints
* The stops are what make the autonomy safe: the merged stop must still come before any test or code.
* `check_issue.rs` rejects a `# Tracking` section in a new issue; its message says the plan is written "after the
  plan is validated", which is no longer true.
* After changing `files/`, run `just sync` and `just test-harness`; `just check-harness` fails on a drifted copy.

# Tracking

Plan: validated

## Plan
1. [x] Rewrite steps 0 to 3 of `issue-workflow` (preparation in the files, one stop with link, summary and numbered
   questions), renumber the steps and update the references (`create-issue`, `check_issue.rs`, `details.md`,
   `write-spec`, `docs/architecture.md`); verified by `just check`, `just test-harness`, `just check-harness`.
2. [x] Fixes of the `high` review of the whole issue.

## Decisions
- Mode: autonomous.
- Base commit: 4c21fa1.
- Questions are numbered, each with lettered options, the recommended one marked, so the user answers `1 a, 2 c`.
- Called from `issue-workflow`, `write-spec` skips its own stop: the single stop covers the spec.
- One stop always, whatever the number of deliverables or an architecture decision.

## Summary of deliverable 1
- done: steps 1 to 3 merged into a preparation in the files and one stop (links, summary, numbered questions
  `# | question | options | why | impact`); deliverable steps renumbered 4.x to 3.x everywhere (skills, hook message,
  `docs/architecture.md`); `write-spec` skips its stop when called from `issue-workflow`.
- checks: `just check`, `just test-harness`, `just check-harness` pass.
- red: `FAIL - issue-workflow: missing 'autonomous (default)'` (the skill was rewritten before `test_skills.rs` got
  the `Plan: proposed` and `1 a, 2 c` passages, so these two were never seen red).
- not verified: the new flow on a real issue (it is prose read by the agent).
- Code review (low): resume branch sent an issue with no `# Tracking` to step 3; fixed (no section: start at step 1).

## Summary of deliverable 2 (fixes of the `high` review)
- done: resume with a missing spec keeps `# Tracking`; base commit set at validation; spec and ADR drafts committed
  with the first deliverable; the stop names a means that differs and the requirement ids; an answer that changes the
  plan shows it again; `Plan: validated` is a required passage; `write-spec` caller exception dropped (redundant);
  `docs/architecture.md` no longer advises `opusplan` (plan mode cannot write the issue).
- not fixed: the queue still lists the issue; closing it is the user's call.
- red: none (prose; the new passage was present before the test).
- Code review (low): dropping the `write-spec` exception could bring back its stop; `issue-workflow` now says to skip it.
