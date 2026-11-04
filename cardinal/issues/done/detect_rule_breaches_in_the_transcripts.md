# Detect the breaches of the harness rules in the transcripts

# Goal
Know whether the agent follows the rules the harness states (AGENTS.md, CLAUDE.md, skills, ADRs), and which
rules are broken often enough to deserve a hook or a rewording. Only the breaches a program can see are in scope.

# Findings
* 2026-10-26: the review gate refusals are visible in the transcripts (for instance "the last code review predates
  the last commit", "only the user waives the code review"); a refusal followed by another way to the same commit
  is what ADR 0005 calls a bypass.
* Skill calls named `files:commit-message`, `files:create-issue`, `files:write-spec`, `files:update-docs` appear
  in this repository's transcripts: the agent picked the source copy under `files/` instead of the synced one.
* Rules a program can check from a transcript: no AI attribution trailer in commit messages, Conventional Commits
  format, conversation in the `user_language` of `.claude/harness.toml`, no edit of a managed copy in a consumer,
  one worktree per issue (no issue work on the default branch), the single validation stop of `issue-workflow`
  before any test or code, no `--no-verify`.

# Approach
1. A list of detectors, one per rule, each reading the tool calls and results of a session and reporting the
   breaches with the session and the line; start with the cheapest (commit commands, edited paths, branch).
2. Count, per rule and per harness revision, the breaches over the sessions where the rule applied, so a rule that
   rarely applies is not read as respected.
3. For the rules that need judgment (the shape of the reports to the user), sample sessions and leave the verdict
   to a later LLM judge: out of scope here.
4. Feed the result to `skill:audit-instructions`: a rule broken often is a hook candidate.

# Specs
* `specs/evaluation.md` (from `extract_session_metrics_from_the_transcripts`), new requirements for the detectors.

# Resources
* `files/AGENTS.md`, `CLAUDE.md`, `.claude/harness.toml`
* `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md`, `docs/adr/0003-one-worktree-per-issue.md`
* `files/.claude/hooks/require_code_review.rs` (its refusal messages)
* `issues/done/extract_session_metrics_from_the_transcripts.md`

# Hints
* A breach already blocked by a hook is not a breach of the agent's behaviour but a gate doing its job; report
  both apart.
* The `files:` skill calls may be a harness defect (skills under `files/` being discovered) rather than an agent
  mistake: check before counting them.

# Tracking

Plan: validated

Status: done (2026-10-26)

## Understanding
List each breach of a rule a program can see in the transcripts, `breach` apart from `blocked` by a gate, and count
them per rule and harness revision over the sessions where the rule applied (BRE-1 to BRE-12). Out of scope: the
rules that need judgment, one worktree per issue (parallel work only), the `files:` skill calls (a Claude Code
offer, not an agent mistake). Means: a managed script next to `session_metrics.rs`; a new `specs/rule-breaches.md`
rather than new requirements in the accepted `specs/evaluation.md`.

## Plan
1. [x] Shared resolution and transcript detectors: project, revision and "file committed at start" move to
   `transcripts.rs`; `rule_breaches.rs` with the rows, `--summary` and the rules `no-verify`, `review-before-commit`,
   `review-waiver`, `managed-copy`, `validation-stop`; verified by `test_rule_breaches.rs` on fixture transcripts and
   repositories, `test_session_metrics.rs` unchanged; makes true BRE-1 to BRE-8, BRE-11, BRE-12.
2. [x] Commit rules: commits of the project joined to the sessions by date; verified by fixture repositories;
   makes true BRE-9.
3. [x] Language rule: stop-word count per assistant text block; verified by fixture transcripts; makes true BRE-10
   (spec `accepted`).
4. [x] Feed `audit-instructions`: its skill runs `rule_breaches.rs --summary` and proposes a hook for a rule often
   broken; verified by `test_skills.rs`.
5. [x] Fixes of the `high` review: stashes, notes and rebased commits, slash commands as user turns, a worktree
   entered mid-session, `<<` inside quotes, scopes of the commit itself, shared `push_distinct`; verified by
   `test_rule_breaches.rs`; amends BRE-4, BRE-5, BRE-7, BRE-8, BRE-9.

## Decisions
- Mode: autonomous
- Base commit: fa37c2219324ad93f51b61fad92b76bf424092c8
- Commit messages: read from the git history of the project, joined to the session by date (a hand commit during a session counts as the agent's).
- Language: stop-word count, French and English only, blocks of 20 words or more.

## Deliverable 1
- done: `transcripts.rs` now holds the session place (cwd, branches, start, end), the repository, project, revision
  and the file committed at a date; `rule_breaches.rs` with the rows, `--summary` and five rules; architecture, README.
- red: the script did not exist, every check failed.
- Spec changes found on the real transcripts: only a refusal of a `git commit` call counts (the old text hook
  refused a python edit script, and the commit after it was covered); only `Plan: proposed` written under `issues/`
  arms the validation stop (a test fixture holding the text did); Bash writes are a non-goal.
- Smoke run on the real transcripts: 3 s, no `breach` left; 21 review refusals and 14 waiver refusals `blocked`.
- Code review: a review refusal suggesting to "ask the user to waive the review" was filed as a waiver refusal and
  disarmed the bypass check; fixed (the waiver refusal is matched on `Refused: only the user waives`, BRE-5, BRE-6,
  fixture extended).

## Deliverable 2
- done: `commit-attribution` and `commit-format` from `git log --all --no-merges` of each project, joined to the
  latest-starting session whose span holds the committer date; scopes from the `harness.toml` committed then;
  architecture, README.
- red: no commit finding, the two commit checks failed.
- Smoke run: 72 sessions with a checked commit, no attribution, two subjects over 72 characters (`4d9117f`,
  `009c901`, which `just check-commit` rejects too).
- Code review: no findings.

## Deliverable 3
- done: `user-language`, stop words of French and English per agent text block of 20 words or more, code removed,
  sub-agents skipped; spec `accepted`; architecture, README.
- red: no language finding, the two language checks failed.
- Smoke run: 74 sessions where it applied, 3 with English answers to a French user (8 blocks, read: true breaches).
- Code review: no findings.

## Deliverable 4
- done: `audit-instructions` runs `rule_breaches.rs --summary` at collection and proposes a hook (or a rewording)
  for a rule broken in a noticeable share of the sessions where it applied; architecture.
- red: `audit-instructions: missing 'rule_breaches.rs --summary'` in `test_skills.rs`.
- Code review: no findings.

## Deliverable 5 (fixes of the `high` review)
- Fixed: commits read from the local branches by author date (a stash was a format breach, a rebased user commit
  the agent's); a slash command or `!` command typed by the user is a user turn (a typed `/code-review` gave a false
  bypass); paths relative to the work tree of each call's `cwd`; `<<` opens a here-document only outside quotes;
  scopes from the `harness.toml` of the commit; `push_distinct` shared; no double tokenizing nor prefix stripping.
- Not fixed: the git calls are not cached (4 s over every transcript); the commit rules are not shared with
  `commit_check.rs`, which is a script with its own `main` and checks the work tree config: BRE-9 keeps the subject
  and attribution rules; `chrono` and `toml` for `audit_sources.rs`, built once, rather than a second module and
  stamp.
- red: the g, h, stash, rebased and branch scope checks failed (extra findings, missed findings, wrong counts).
- Smoke run: same findings as before (2 `commit-format`, 8 `user-language`).
- Code review: no findings.
