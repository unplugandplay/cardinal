---
priority: medium
why: 'a team needs to know who approved a plan or a deliverable and each person should own what they approve, but nothing is blocked without it'
---
# Record who validated each step of an issue

# Goal
The validation stops of `skill:issue-workflow` (the plan at step 2, each deliverable at step 3.6 when the mode is
not autonomous) leave no trace of who validated them: `Plan: validated` and the ticked deliverables don't name
anyone, and the commits only carry the git author, the same whoever approved. In a team, the next person reading the
issue or the history cannot tell who approved a plan or a deliverable. A visible signature also makes the person
who validates own what they approve.

Expected outcome: each validation names the person and the date, in the issue and in the commit, in a declarative
form the agent writes from the git identity of the working copy. It records a statement, not a proof.

# Findings
* The plan marker is a bare `Plan: validated` plus `Base commit` in `## Decisions` (issue-workflow step 2), and
  the deliverables are ticked `[x]` at step 3.7, with no name and no date.
* In autonomous mode, only the plan has a human validation; the deliverables are committed right after their
  summary is written in the issue.
* `cardinal run commit-check` already checks a footer line (`Expected-effect:`) and bans attribution trailers;
  a new `Validated-by:` trailer must not trip the attribution rule.

# Approach
Declarative signature only (level 1). A signature typed by the human (a `UserPromptSubmit` hook) or signed with
GPG/SSH is out of scope: too constraining for now.

1. At step 2, write `Plan: validated by <git user.name> <<git user.email>> on <YYYY-MM-DD>` in place of the bare
   marker (`git config user.name` / `user.email` of the working copy); `Plan: validated` alone stays readable for
   the issues planned before.
2. At step 3.6 (per-deliverable mode), record the validator on the deliverable line when ticking it at step 3.7;
   in autonomous mode, record that the deliverable was not validated by a person (e.g. `autonomous`), so the
   signature never claims a review that did not happen.
3. Add a `Validated-by: <name> <<email>>` trailer to the commit of a deliverable a person validated
   (`skill:commit-message`), and none to an autonomous one; maybe let `commit-check` check its form.
4. Decide whether the signature belongs to an existing feature (`issue-files`) or to a new one of
   `features.toml` a solo project can turn off.

# Specs
* The issue-workflow skill has no spec of its own; if `commit-check` checks the trailer, amend the spec that covers
  commit messages, else name the new requirements at step 1.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 2, 3.6 to 3.8, "Autonomous mode")
* `files/.claude/skills/commit-message/`
* `crates/cardinal/src/tools/commit_check.rs`
* `crates/cardinal/tests/skills.rs` (asserts texts of the skills, `Plan: validated`)
* `features.toml`, `specs/features.md`

# Hints
* The git identity is the one of the machine, which the agent can read and use: the signature states who was at
  the keyboard, it proves nothing; say so in the skill.
* The resume of a thread (issue-workflow) reads the plan marker and must still accept the bare `Plan: validated`;
  `rule_breaches.rs` matches `Plan: proposed` (BRE-8), which this change leaves alone.
* Edit `files/`, then `just sync`: the copies in `.claude/` are not the source.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
Each human validation of `issue-workflow` (plan at step 2, deliverable at step 3.6) names who validated and when,
from the git identity, in the issue and as a `Validated-by:` commit trailer; an autonomous deliverable says so.
Out of scope: a proof (a hook on the typed prompt, a GPG/SSH signature). No spec: a change of the harness text.

## Plan
1. [x] Signed validations: `files/.claude/skills/issue-workflow/SKILL.md` writes `Plan: validated by <name> <<email>> on
   <date>` at step 2, the validator or `autonomous` on the ticked deliverable at step 3.7, and the trailer at step
   3.8 for a validated deliverable only; a resume still reads a bare `Plan: validated`; `just sync`; verified by new
   assertions of `crates/cardinal/tests/skills.rs` and `commit-check` passing a message with the trailer. (autonomous)

2. [x] Fixes of the `high` review: the signature of step 3.7 waits for step 6, an unset git identity is asked, the
   footer of the commit convention names `Validated-by:`, `docs/workflow.md` shows the signature, a passage pins the
   resume of a bare `Plan: validated`; verified by `skills.rs`. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: d7d4dc31900f1c31bec563b5c7183b4504a06ab0
- Feature: the signature belongs to `issue-files`, no new feature (1 a).
- `commit-check` does not check the form of `Validated-by:`; a test only shows it passes (2 a).

## Summary of deliverable 1
- done: issue-workflow signs the plan marker, the ticked deliverable line and the commit trailer `Validated-by:`;
  an autonomous deliverable is marked `(autonomous)`, with no trailer.
- checks: `just check` green (fmt, clippy, tests, specs); docs: `docs/workflow.md` keeps the `Plan: validated` prefix.
- red: none (no spec): the three passages of `skills.rs` missing before the edit of the skill.
- Code review: no findings

## Summary of deliverable 2
- done: the fixes of the `high` review (plan line 2).
- checks: `just check` green; docs: `docs/workflow.md`, `conventional-commits.md`.
- red: none (no spec): the resume passage was the only new assertion.
- Not fixed from the `high` review: a stopped autonomous chain stays `(autonomous)` (a decision is not the
  validation of the deliverable); the `commit-check` test stays a pass-through (decision 2 a); the email in the
  issue files is the one git already publishes in every commit.
- Code review: 1 finding (plan line 2 pasted into the Findings): fixed
