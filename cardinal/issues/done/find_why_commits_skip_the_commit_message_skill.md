---
priority: medium
why: 'the rule breached most often on the last groups, with false positives that blur it'
---
# Find why commits skip the commit message skill

# Goal
`skill-routing-commit-message` (BRE-16) counts a `git commit` made before `skill:commit-message` was loaded in the
session. Its breached share swings from 4% to 43% between groups and stayed at 40% and 42% on the last two complete
groups, so the skill is often skipped, or the rule counts commits it should not. The outcome: the commits the rule
should not count are left out, and the remaining skips have a known cause and a fix.

# Findings
* Measured 2026-11-03 over 525 sessions: 145 breaches and 40 blocked commits in 44 sessions, spread over this
  repository and three consumer projects.
* Breached share by group (last five complete): 10%, 43%, 4%, 40%, 42%; a rule audit was never run on it.
* Commands of the breaches, by kind:
  * harness update commits (`chore(harness): update the shared harness to …`, `chore(cardinal): update Cardinal to …`);
  * the close of an issue or a review line appended to the issue then committed in the same command;
  * commits in a throwaway repository (`mktemp -d`, `git init`, `chore: init`, `chore: adopt cardinal`): a smoke
    test, not a commit of the project;
  * ordinary deliverable commits with a message written inline (`msg='…'`, `-F -`).

* Classified 2026-11-03 (195 findings, 48 sessions; cardinal 99, Assistant 51, RetroDoc 43, CatchLead 2), by the
  subject and the command of each commit:

  | kind | findings | sessions | message checked in the same command |
  |---|---|---|---|
  | deliverable commit | 91 | 28 | 81 |
  | issue only (open, close, tracking, roadmap) | 66 | 30 | 52 |
  | harness update (`update`, `sync`, `adopt`) | 28 | 13 | 19 |
  | throwaway repository (`git init`, `mktemp`, `cd /tmp/…`) | 10 | 5 | 0 |

* 152 of the 195 commands pipe the message through the checker (`cardinal run commit-check`, `just check-commit`)
  themselves: step 4 of the skill done without loading it. `commit-format` counts 1 breach over 273 sessions: the
  convention is followed; what is skipped is the loading of the skill, not its outcome.
* The skips cluster by session: 4 findings per breaching session on average, 25 of the 48 sessions hold several kinds.

# Approach
1. Classify all the breaches of the last groups by the kinds above, with counts.
2. Leave out of the rule the commits outside the project's repository (a `git init` or a `cd` to another directory in
   the same command), and amend BRE-16.
3. For the kinds that remain, run `skill:audit-instructions` with the counts: candidates are a line in
   `skill:issue-workflow` at the close of an issue, a line in the update recipe or `cardinal update` output, or the
   commit gate hook refusing a commit with no checked message.

# Specs
* `specs/rule-breaches.md` BRE-15, BRE-16.

# Milestone
Lean agent loop

# Resources
* `specs/rule-breaches.md`, `files/.claude/skills/commit-message/SKILL.md`, `files/.claude/skills/issue-workflow/SKILL.md`.
* `files/.claude/skills/audit-instructions/SKILL.md`.

# Hints
* A `blocked` commit (its result an error) was refused by a hook: it is a breach the gate already caught.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
Most skips are not wrong commits: 78% of them validate the message with the checker inline, and the convention
holds (`commit-format`: 1 breach). Leave out what the rule should not count, then fix the routing of what remains
with an evidence-based instruction change. Out of scope: the other routing rules (BRE-17, BRE-18).

## Plan
1. [x] Rule scope: a commit in a throwaway repository (`git init`, `mktemp`, a `cd` or `git -C` outside the work
   tree, earlier in the same call) is no trigger, nor is a commit whose call runs the checker;
   verified by tests of `rule_breaches.rs` from the scenarios; makes true BRE-16. (autonomous)
2. [x] Remaining skips: run `skill:audit-instructions` with the counts of the kinds left after deliverable 1 and
   apply the instruction change it proposes and the user validates (candidates: a line at the close of
   `skill:issue-workflow`, the output of `cardinal update`, a commit gate refusing an unchecked message); verified by
   `just check` and the re-measured counts written in the issue. (autonomous)
3. [x] Fixes of the `high` review: the trigger reads the commands in order (a `mktemp`, `git init` or `git clone` before
   the commit, a `cd` back into the work tree ends the detour, `..` resolved), the checker only as a command, `-C` read
   by `shell::git_invocations`; verified by the table of commits of the routing test; makes true BRE-16. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: f8c93e18b4d4d9f3f2633c8da9b00ad53dd4f69c
- A commit whose call runs the message checker is no trigger: the skill's outcome is met (question 1).
- Deliverable 2 runs `skill:audit-instructions` on the remaining kinds and applies the change the user validates (question 2).
- Audit validated: the close commit of `skill:issue-workflow` names `skill:commit-message` (7 close commits in 7
  sessions); the update output becomes [route_the_update_commit_to_the_message_skill](route_the_update_commit_to_the_message_skill.md);
  a gate refusing an unchecked commit waits for the re-measure.

## Deliverable 1
done: a commit in a throwaway repository or whose call runs the message checker is no trigger of BRE-16; re-measured
on 2026-11-03: 33 findings in 22 sessions (from 195 in 48), 21 breached of 151 applied sessions.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | `docs/evaluation.md` still exact, no change |
| code review | low: 1 finding, fixed |

red: BRE-16: the throwaway and checked commits of the test were findings (`s.jsonl:12` to `s.jsonl:22`).

Remaining 33 by kind: issue only (open, close, tracking) 16, harness update 9, deliverable 8.

Code review: 1 finding
- `mktemp` was read anywhere in the text, so a commit message naming it hid a trigger: fixed, only a command
  `mktemp` or a `$(mktemp` substitution counts, with a test.

not verified: none

decisions needed: none

## Deliverable 2
done: the close commit of `skill:issue-workflow` (Rules and "Merge back") takes its message from
`skill:commit-message`; the follow-up issue for the update output is opened.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | the skill is the doc; `just sync` copied it |
| code review | low: no findings |

red: none

Code review: no findings

not verified: the effect on the close commits, until new sessions are measured

decisions needed: none

## Deliverable 3
done: the fixes of the `high` review of `f8c93e1..81f0732`; re-measured: still 33 findings in 22 sessions.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a: BRE-16 is the doc, amended |
| code review | high: 10 findings, 7 fixed; low of the fixes: no findings |

red: BRE-16: on the code of deliverable 1, `git clone … && git commit` was a finding and the checker staged, a later
`mktemp`, a `$(mktemp` in a message, a `cd` back and `<repo>/sub/..` were none.

High review findings:
- the checker read in any word (a staged `commit_check.rs`): fixed, a command running it only;
- a `$(mktemp` in a message: fixed, the text before the first `commit` only;
- a `mktemp` after the commit: fixed, the commands read in order;
- a `cd` back into the project: fixed;
- `git clone` into a scratch directory: fixed, read like `git init`;
- `-C` read by a second parser: fixed, `shell::git_invocations` returns it;
- `..` in a path: fixed, resolved on the text;
- symlinked paths: not fixed, the paths of a transcript may be gone and the measure runs on Linux;
- a removed worktree whose line had a subdirectory `cwd`: not fixed, the fallback of `worktree()` is shared by
  every rule of BRE-7;
- a `cd` to another worktree of the same repository: not fixed, the agent never redirects to the main worktree
  (`skill:issue-workflow` step 0) and the gone path cannot be asked for its common dir;
- quoted substitutions dropped by `shell::commands`: not fixed here, a change of the shared tokenizer for every rule.

Code review: 10 findings

not verified: none

decisions needed: none
