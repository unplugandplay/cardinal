# Tighten the issue workflow

# Goal
`issue-workflow` drives every change, so its gaps and its weight are paid on every issue. The review lets findings
outside the deliverable grow its scope, its cost depends on whatever level was typed last, the TDD loop has no
refactor step, and the skill loads rare details on each run. Make each issue cheaper and its deliverables smaller
and cleaner.

# Findings
* `files/.claude/skills/issue-workflow/SKILL.md` is about 10 KB, loaded for every issue; the "Parallel agents"
  section and the waiver details of step 4.5 serve rarely.
* Step 4.5 invokes `/code-review` without a level; the skill then reuses the level the user typed last.
* `issues/done/make_the_review_waiver_usable.md`: 10 review findings fixed inside one deliverable, none of them in
  the requirements it was planned for.
* Step 4 goes test, implement, docs, review: no step cleans the code once the test passes.
* Step 4.2 asks for a test that "fails for the right reason", but the summary of step 4.6 never shows the failure.
* Steps 1 and 3 are two separate stops even for an issue whose plan is one deliverable.
* Seven skills say "Show the user your results as `AGENTS.md` section ... says", which makes the agent read one more
  file to learn four rules.

# Approach
1. Review scope: a finding outside the requirements of the deliverable becomes an issue (`skill:create-issue`), not a
   fix in the deliverable; only findings on the deliverable's own change are fixed.
2. Review level: name it in step 4.5 (for example `low` per deliverable, `high` once at the end of the issue).
3. Add a refactor step after "Implement": clean under green tests, no new behaviour.
4. Show the red: one `red:` line with the failure message in the step 4.6 summary.
5. Let steps 1 and 3 be one stop when the issue changes no capability and fits one deliverable.
6. Move "Parallel agents" and the waiver details to `docs/architecture.md`, with one line pointing there; consider
   inlining the four report rules instead of citing `AGENTS.md`.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`, `files/AGENTS.md` (section "Reports to the user")
* `files/.claude/skills/test_skills.rs` (checks the passages of `REQUIRED_PASSAGES` and the sections of `AGENTS.md`
  the skills cite)
* `docs/architecture.md`

# Hints
* `specs/agent-reports.md` and the wording checks of `test_skills.rs` are gone
  (`drop_redundant_guards_and_prose_tests`); `REQUIRED_PASSAGES` still pins the autonomous mode.
* Keep the validation stops that the autonomous mode never lifts.

# Tracking

1. [x] Step 4 tightened: review scoped to the deliverable and its level named (4.5), refactor step, `red:` line in
   the 4.6 summary.
2. [x] Steps 1 and 3 may be one stop for an issue that changes no capability and fits one deliverable.
3. [x] Slimmer skill: "Parallel agents" and the waiver details move to `issue-workflow/details.md`; the four report
   rules are inlined in `issue-workflow` only.
4. [x] End-of-issue `high` review on `4b50d41..HEAD`, fixes of the findings on deliverables 1 to 3.

## Decisions

- Mode: autonomous.
- No spec: the issue changes the text of a shared skill, no capability.
- Base commit: `4b50d41` (the work is on `main`, so `main..HEAD` would be empty).
- Review level: `low` per deliverable, `high` once at the end of the issue on `4b50d41..HEAD`.
- Report rules: inlined in `issue-workflow` only; the six other skills keep citing `AGENTS.md`.
- Refactor step: a cleanup by hand under green tests, no `/simplify`.
- The moved passages go to a shared file of the skill (`details.md`), not to `docs/architecture.md`, which consumers
  do not get.

## Summaries

### 1. Step 4 tightened

- done: 4.3 implements then refactors under green tests (kept in step 3, so the step numbers that other text cites
  stay); 4.5 names the level (`low`) and turns findings outside the deliverable into issues; 4.6 and the autonomous
  summary show a `red:` line; a `high` review of the whole issue closes it, never lifted by the autonomous mode.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| lint, specs | pass |
| docs | none to change (the skill is the doc) |
| code review | `low`, see below |

- red: n/a, text change, no test.
- Code review: no findings.
- not verified: an agent following the new steps on a real issue.
- decisions needed: none.

### 2. Steps 1 and 3 in one stop

- done: step 1 shows the plan of step 3 in the same message when the issue changes no capability and fits one
  deliverable; one stop validates both. Also commits the `harness.lock` left by the sync of deliverable 1.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness`, `just check-harness` | pass |
| docs | none to change |
| code review | `low`, see below |

- red: n/a, text change, no test.
- Code review: no findings.
- not verified: an agent following the merged stop on a real issue.
- decisions needed: none.

### 3. Slimmer skill

- done: the waiver details and the parallel-agents procedure move to `issue-workflow/details.md`, linked from
  `SKILL.md`; the four report rules are inlined in `issue-workflow`; `test_skills.rs` checks that the relative
  markdown links of a skill (outside code spans) name an existing file of its directory.
- size: `SKILL.md` 10 778 B before this deliverable (10 046 B before the issue) → 9 898 B, and no more read of
  `AGENTS.md` for the report rules; `details.md` 1 671 B, read only for a waiver or a worktree.

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness`, `just check-harness` | pass |
| docs | none to change (no doc lists the skill's files or the checks of `test_skills.rs`) |
| code review | `low`, see below |

- red: `FAIL - issue-workflow: links the missing file 'details.md'`
- Code review: no findings.
- not verified: that an agent opens `details.md` when a worktree or a waiver comes up.
- decisions needed: none.
- note: the `low` review left `test_skills.rs` out; the `high` review of deliverable 4 covered it.

### 4. End-of-issue high review

- done: `/code-review high` on `4b50d41..HEAD`, 9 findings:

| finding | outcome |
|---|---|
| the fix commit after a clean-tree review is refused by the gate | fixed: the fixes are one more deliverable with their own `low` review |
| no human stop for those fixes in per-deliverable mode | fixed: same, steps 4.2 to 4.8 apply |
| no way to find the base of the issue (`main..HEAD` empty on `main`) | fixed: step 3 records the base commit; this issue's decision corrected to `4b50d41` |
| the link check fails on `#anchor`, a title or `mailto:` | fixed: anchor and title dropped, schemes skipped |
| the checks read only `SKILL.md`, not `details.md` | fixed: every markdown file of a skill is checked |
| `red:` says "step 2" and nothing for a deliverable without test | fixed: step 4.2, `none` without test |
| the inlined report rules can drift from `AGENTS.md` | not fixed: decision (b) of step 1 |
| the gate's refusal names no level | outside the deliverables: `issues/done/name_the_review_level_in_the_gate_message.md` |
| the linked-worktree closing written three times | fixed: the Rules line only points to "Parallel agents" |

| check | result |
|---|---|
| tests, lint, specs (`just check`) | pass |
| `just test-harness`, `just check-harness` | pass |
| docs | none to change |
| code review | `low` on the fixes, see below |

- red: none (the link check was verified by hand: a missing file fails; an anchor, a title and `mailto:` pass).
- not verified: an end-of-issue review run by an agent on another issue.
- decisions needed: none.
- Code review (`low`, on the fixes): no findings.
