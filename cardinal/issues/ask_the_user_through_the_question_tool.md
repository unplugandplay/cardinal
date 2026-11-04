---
priority: medium
why: 'a question asked in text is easy to miss and its answer is free text; a dialog shows it and offers the choices'
---
# Ask the user through the question tool

# Goal
The skills have the agent stop and ask in its reply ("ask the user", "ask whether to close the issue", the
recommendations of the stop of step 2). A question buried in a long reply is easy to miss, and the answer comes
back as free text. Claude Code has a tool for it (`AskUserQuestion`): a dialog of up to 4 questions, each with 2 to
4 options, the recommended one first, "Other" always offered. The outcome: the agent asks its questions and
confirmations through the tool where they fit, and the share of turns that end on a question in text drops.

# Findings
* No file of `files/` names `AskUserQuestion` (`grep -rn AskUserQuestion files/` on 2026-11-03).
* Places that ask in text: `files/.claude/skills/issue-workflow/SKILL.md` step 2 (the recommendations, `ok` takes
  them all), line 89 (a missing date), line 174 (ask to close the issue), line 184 (the release proposal);
  `files/.claude/skills/create-issue/SKILL.md:16` and `:73`.
* The tool's own guidance says the user may see only the dialog, not the text before it: each question must stand on
  its own. A stop that shows a plan or a summary keeps it in the reply and puts only the decision in the dialog.
* `language-hint` (`crates/cardinal/src/tools/language_hint.rs`, `specs/language-hint.md`) is a `Stop` hook that
  reads the last reply and sends the agent back once; the same shape could hint when a reply ends on a question.
* Not measured yet: how many turns end on a question in text, and how the tool behaves under `claude -p` (the replay
  runs of `fixtures/replay/`).

# Approach
1. Measure first: count in the transcripts the last replies of a turn that end on a question with no
   `AskUserQuestion` call (a rule of `rule-breaches`, or a one-off script), per revision.
2. Write the rule: a line in `files/AGENTS.md` or a rule of `files/.claude/rules/` (closed choices and confirmations
   go through the tool, the question self-contained; open questions and content to review stay in text), and name
   the tool at the places of the skills listed above.
3. Optionally a `Stop` hook hint, like `language-hint`, when the last reply ends on a closed question asked in text;
   declared as a feature of `features.toml`.
4. Judge it with `skill:evaluate-harness` (`Expected-effect:` on the measure of step 1).

# Specs
* `specs/rule-breaches.md` if step 1 adds a rule; a spec of the hook if step 3 adds one.

# Milestone
User control

# Resources
* `.claude/rules/reports-to-the-user.md` (the format of the stops, which this rule must not contradict)
* `crates/cardinal/src/tools/language_hint.rs`, `specs/language-hint.md`, `features.toml`
* `crates/cardinal/src/tools/rule_breaches.rs`, `specs/rule-breaches.md`
* `docs/evaluation.md` (the replay runs, headless)

# Hints
* Headless runs cannot answer a dialog: check what the tool does under `claude -p` before the rule or the hook
  makes the replay stall or fail.
* "Don't ask what has a conventional default" (step 2) still holds: the tool must not turn into more questions.
* A rule always loaded costs fixed context (`fixed_context_budget`): keep it to a few lines.
