# Read the user language from a personal file

# Goal
`user_language` is a preference of the person who talks to the agent, yet it sits in the `[project]` table of
`.claude/cardinal.toml`, a file the project commits and shares. In a team every member gets the same conversation
language. Read it (and any later personal preference) from a file each person owns and git ignores, the shared value
staying the default.

# Findings
* Readers of `user_language`: the `language-hint` Stop hook (`crates/cardinal/src/tools/language_hint.rs`, LNG-3),
  `Language::from_config` (`crates/cardinal/src/tools/language.rs`), rule `user-language` of `rule-breaches`
  (BRE-10), and the skills `issue-workflow`, `prioritize-issues`, `create-issue`, `audit-instructions` and
  `evaluate-harness`, which tell the agent to read the key in `.claude/cardinal.toml`.
* BRE-10 reads the `.claude/cardinal.toml` committed at the session's `start`: a git-ignored file has no history, so
  the rule would judge a session against the shared value, not the language that session used.
* `seed/.claude/cardinal.toml` seeds `user_language = "English"`; no other key of `[project]` is personal today.
* The user agreed on 2026-10-29 that this is worth doing for a team; the harness itself stays imposed on every
  member (the review gate is intended).

# Approach
1. Choose the personal file: a git-ignored `.claude/cardinal.local.toml` whose `[project]` values override the shared
   ones, or the `env` of Claude Code's personal `.claude/settings.local.json`, or a user-level file outside the
   repository (one language for every project of a person).
2. One reader for the merged configuration, used by the hook and `rule-breaches`, instead of each tool reading the
   key; `sync` seeds or ignores the personal file (an entry in `.gitignore`).
3. For BRE-10: read the personal value as it is when the measure runs, or record the language of the session in
   the transcript (the hint's own output), or keep the committed value as the only measured one and say so.
4. Point the skills at the merged value instead of `.claude/cardinal.toml`.

# Specs
* `specs/language-hint.md` (LNG-3), `specs/rule-breaches.md` (BRE-10), `specs/sync.md` if `sync` seeds or ignores
  the personal file.

# Resources
* `docs/architecture.md` ("What is shared, and how": project values go through `.claude/cardinal.toml`).
* `crates/cardinal/src/tools/language.rs`, `language_hint.rs`, `rule_breaches.rs`.
* `issues/done/hint_when_the_reply_is_not_in_the_user_language.md`.

# Hints
* A personal file must never be managed: `sync` and `check` hash only what the source writes.
* Keep the shared value as the fallback, so a solo project changes nothing.

# Tracking

Plan: validated

## Understanding
Read `user_language` from a git-ignored `.claude/cardinal.local.toml` (its `[project]` table overrides the shared one
key by key), the shared value staying the default; the hook, rule `user-language` and the skills read it. Out of
scope: any other key (the shared ones stay shared), a user-level file outside the repository.

## Plan
1. [x] Personal file and hook: managed `files/.claude/.gitignore` ignoring `cardinal.local.toml`, one reader of the
   personal value in `language.rs`, used by `language-hint`; ADR 0018 and the spec amendments; verified by tests
   through the binary (hook with both files, `sync` then `git check-ignore`); makes true LNG-12, SYN-59.
2. [x] `rule-breaches`: the personal value of the repository root, as it is at measure time, wins over the committed
   one; verified by a test through the binary; makes true BRE-10, BRE-14.
3. [x] Skills, seed comment and docs point at `.claude/cardinal.local.toml`, else `.claude/cardinal.toml`
   (`issue-workflow`, `prioritize-issues`, `create-issue`, `audit-instructions`, `evaluate-harness`,
   `docs/architecture.md`); verified by `just check-cardinal` and a reread.

## Decisions
- Mode: autonomous
- Base commit: 703b389ce5a67d5e34200e84ac3a8ec9dfb9be18
- Personal file: `.claude/cardinal.local.toml` per clone (1 a).
- Ignored by a managed `.claude/.gitignore`, never by an edit of the project's `.gitignore` (2 a).
- BRE-10 reads the personal value at measure time, else the committed one (3 a).
- The hint keeps `the user_language of .claude/cardinal.toml` (the mark BRE-13 finds in older transcripts).
- A personal `user_language` wins even when it names a language with no stop words (no judgement then).

## Deliverable 1

done: `.claude/cardinal.local.toml` read first by `language-hint`, ignored by the managed `.claude/.gitignore`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | LNG and SYN `accepted`, BRE `draft` until deliverable 2 |
| docs | `docs/architecture.md`, ADR 0018 |
| code review | see below |

red: LNG-12, SYN-59: the hook exited 2 on an English reply with `user_language = "English"` in the personal file;
`files/.claude/.gitignore` did not exist.

Code review: no findings

not verified: a consumer that already has its own `.claude/.gitignore` gets a sync conflict on it (SYN-3), as for
any managed path.

decisions needed: none

## Deliverable 2

done: rule `user-language` judges a session against the personal `user_language` of its repository when there is
one, else the committed one.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | BRE `accepted` |
| docs | n/a (no page states where the rule reads the language) |
| code review | see below |

red: BRE-14, BRE-10: the English blocks of a session whose personal `user_language` is English were breaches
(`s.jsonl:2 intermediate`), and the German one was judged.

Code review: no findings

not verified: none

decisions needed: none

## Deliverable 3

done: the five skills, the seeded comment and `docs/usage.md` name `.claude/cardinal.local.toml` first.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | n/a |
| docs | `docs/usage.md`, seed comment |
| code review | see below |

red: none (no test: wording of skills and docs)

Code review: no findings

not verified: none

decisions needed: none

## Deliverable 4 (high review fixes)

done: `start-issue` copies the personal file into each issue worktree (PAR-17); the hook reads the personal value past
a broken shared file; the hint names the personal file next to its mark; doc comments rewrapped.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | PAR-17 added, `accepted` |
| docs | ADR 0018, `docs/usage.md`, `docs/architecture.md` |
| code review | see below |

red: PAR-17: `start copies the personal configuration of the main worktree into the new one` failed (no file in the
worktree).

High review (`703b389..63b18b8`): 7 findings.
- worktree lacks the personal file, hook and measure disagree: fixed (PAR-17 copy).
- managed `.claude/.gitignore` conflicts with a project's own: kept, decision 2 a; such a project resolves it once.
- hint points at the shared file only: fixed (suffix naming `.claude/cardinal.local.toml`, mark kept).
- broken shared file hides the personal value: fixed.
- personal file read once per session: not fixed, one small read per transcript is negligible.
- "once per clone" ignores worktrees: fixed with the copy, docs say so.
- doc comment wrapping: fixed in the Rust files; the SKILL.md lines already exceed 120 columns elsewhere.

Code review: no findings

not verified: none

decisions needed: none

Status: done, awaiting the agreement to close.
