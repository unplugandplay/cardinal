---
rank: 165
why: 'after the size tool: the second mechanical rule the audit found; measure first, it may not be broken'
---
# Refuse staging every file before a commit

# Goal
`skill:commit-message` tells the agent to add only the files of the deliverable, never `git add -A`, so personal
or unrelated files (`settings.local.json`, another deliverable's work) stay out of the commit. The rule is prose;
a deny rule or a hook could refuse the command, if the agent breaks it often enough to pay for one.

# Findings
* The audit of the shared instructions (`issues/done/audit_the_shared_instructions_for_conflicts_and_stale_guidance.md`,
  row M2) found the ban in four skills, now kept in `commit-message` alone.
* `rule-breaches` has no rule for it: how often the agent runs `git add -A` or `git add .` is not measured.

# Approach
1. Measure first: a rule `stage-everything` in `rule-breaches` (a `Bash` call of `git add` with `-A`, `--all` or
   `.`), read over the kept transcripts.
2. If it is broken: a deny rule in `entries/.claude/settings.json` (`Bash(git add -A)`, `Bash(git add -A *)`, …),
   or a `PreToolUse` refusal when a command chains it (`git add -A && git commit`), which a prefix rule misses.
3. Breached rarely: keep the prose line and close the issue with the measure.

# Specs
* `specs/rule-breaches.md`: a new requirement for the rule of step 1.

# Milestone
Lean agent loop

# Resources
* `files/.claude/skills/commit-message/SKILL.md`, `entries/.claude/settings.json`, `specs/rule-breaches.md`.
* ADR 0005 (the gates catch mistakes, not bypasses), ADR 0013 (sync manages the allow rules too).

# Hints
* `issues/done/hook_code_review_before_commit.md`: the review gate runs before the `git add` of
  `git add -A && git commit`, so a check on the staged files sees nothing yet.

# Tracking

Plan: validated

## Understanding
Measure how often the agent stages everything (`git add -A`, `--all`, `.`), then refuse it in the existing
`PreToolUse` hook `review-gate check` if it is broken: one mechanism that also sees chained lines, so no deny rules.
Out of scope: `git commit -a` (tracked files only, no personal untracked file), a glob (`git add *`).

## Plan
1. [x] Rule `stage-everything` in `rule-breaches`, then the measure over the kept transcripts recorded here; verified by
   a test of the rule on a built transcript; makes true BRE-21.
2. [x] If the measure shows breaches in the sessions of a Cardinal revision: `review-gate check` refuses a `git add` of
   everything, the commit-message prose kept as the reason; verified by tests of `check` on chained, quoted and
   `-C` lines; makes true REV-21. Else: REV-21 withdrawn and the issue closed with the measure.

## Decisions
- Mode: autonomous
- Broken: at least one breach in a session of a Cardinal revision (answer 1 a).
- Mechanism: extend `review-gate check`, no deny rules (answer 2 a).
- Base commit: ae5e7c99ea1918807bff76f67117a706b8e29294
- Pre-measure (grep of the raw transcripts, before any revision filter): 105 `git add -A` and 2 `git add .` in 29 of
  the 151 transcripts holding a commit, 24 of them dated 2026-10-24 or later.
- BRE-21 applies to the sessions with a `git add` (not those with a commit): a finding in a session with no commit
  would else count in no applied session.
- Measure (2026-10-31, `cardinal run rule-breaches --summary`, every revision): `stage-everything` applied in 200
  sessions, breached in 33 (16.5 %), 55 breaches and 12 blocked (by a permission prompt), in `cardinal` (21),
  `Assistant` (19) and `RetroDoc` (15). Broken by decision 1 a: deliverable 2 goes.

## Summary of deliverable 1
done: rule `stage-everything` in `rule-breaches`, the shared `git_invocations` reads the subcommand for BRE-16 and BRE-21.
red: BRE-21: `FAIL - a git add of -A, --all or . is a finding, not one of named files nor a quoted one`
Code review: 1 finding
- A `-A` or `.` of a later chained command (`git add a && ls .`) counts, the words reaching the next `git` (BRE-21
  as written, like `--no-edit` of BRE-16): not fixed, a rare over-count of a measure; the hook of deliverable 2 reads
  each command of the line apart.
- `rule-breaches` was left `draft` by deliverable 1 though BRE-21 is traced: set `accepted` with deliverable 2.

## Summary of deliverable 2
done: `review-gate check` refuses a `git add` of `-A`, `--all` or `.` in any command of the line, quoted strings and
here-document bodies dropped first; `git_commands` shares the reading of the global options with REV-12; the
commit-message line names the hook.
red: REV-21: `FAIL - git add -A` (and the 7 other refused lines)
- The first build refused a command of this session whose here-document quoted `git add -A` between backticks: the
  line is now unquoted before it is cut into commands, and the allowed cases cover it.
Code review: no findings
3. [x] Docs of the whole issue and fixes of its `high` review: README (Gates), `docs/usage.md`; one shell reader
   `tools/shell.rs` shared by the hook and the rule; makes true REV-21, BRE-21 as amended.

## Summary of deliverable 3
done: `shell.rs` reads the commands of a line once (quotes, escapes, comments, here-documents with a terminator
starting with a letter) and judges a staging of everything for `review-gate check` and `stage-everything`;
`-A` in a bundle, `./` and `:/` count, a dry run does not; README and `docs/usage.md` name the rule.
red: REV-21: `FAIL - echo $((1 << 2))` and the other cases of the high review, before the shared reader.
Code review (high, `base..HEAD`): 8 findings
- 1, 2, 3, 5, 7 (here-document terminator, escapes, a `.` of a later command, comments, two readers): fixed by
  `shell.rs`.
- 4 (`-Av`, `./`, `:/`): fixed; a quoted flag (`git add '-A'`, `git "add"`) is a bypass, not a mistake (ADR 0005).
- 6 (dry run refused): fixed.
- 8 (REV-12 keeps its own reading of the line): not fixed, out of this issue; REV-12 works and its tests pass.
- Measure again with the shared reader: 201 sessions applied, 33 breached, 55 breaches, 12 blocked.
Code review (low): 1 finding: the rule parsed the command twice; fixed, `stages_everything` takes the parsed commands.
