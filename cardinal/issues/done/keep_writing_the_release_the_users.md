---
priority: medium
why: 'one ask rule closes what the autonomous mode leaves open: a release written without the user validating it'
---
# Keep writing the release the user's

# Goal
The Rules of `issue-workflow` say a release is the user's to trigger, and the agent only proposes the version
(`cardinal run release` with no argument). Yet the managed allow rule `Bash(cardinal run *)` also lets
`cardinal run release <version>` (writes `CHANGELOG.md`, runs `release_version_command`) and
`cardinal run release <version> --tag` (creates the tag) run without a prompt. The outcome: the agent proposes the
release, the user validates it, the agent runs it. The bare proposal stays allowed; each form that writes runs only
once the user approved it (a permission prompt, never a refusal), so the agent still executes the release, never
unasked.

# Findings
* 2026-11-02, the `high` review of `issues/done/allow_the_release_proposal_in_auto_mode.md`: `entries/.claude/settings.json`
  allows `Bash(cardinal run *)`, which covers every form of `cardinal run release`; nothing guards the writing forms.
* 2026-11-02, the user: the agent proposes the release, the user validates, the agent executes. A deny rule would
  leave the user to type the release themselves: not the outcome. Approach 1 chosen over a `review-gate check`
  answering `ask` (no new entry kind, but a second job for the gate).
* `sync` manages only `permissions.allow` and `permissions.deny` (`crates/cardinal/src/entries.rs`, SYN-29, SYN-34).

# Approach
1. Manage the ask rule `Bash(cardinal run release *)` in `entries/.claude/settings.json`: Claude Code reads deny, then
   ask, then allow, so it wins over `Bash(cardinal run *)` while the bare form, matched by the allow rule only, runs
   without a prompt. `sync` must then manage `permissions.ask` (amend SYN-29, SYN-34 with an `ask <rule>` entry); the
   lock line `{"ask": ...}` is one an older `cardinal` misreads, so raise `cardinal-format` and `FORMAT` (ADR 0014).
   Check that the ask rule still prompts in auto mode.
2. The Rules of `issue-workflow`: at a milestone end, propose the version and stop; on the user's go, run
   `cardinal run release <version>`, let the user reread the section, commit, then `cardinal run release <version>
   --tag`; the user pushes (REL-8). The permission prompt is the proof of the validation, not a second question.

# Milestone
Longer autonomy

# Resources
* `entries/.claude/settings.json`, `crates/cardinal/src/entries.rs`, `features.toml` (a rule may belong to a feature),
  `specs/release.md`, `specs/sync.md` (SYN-29, SYN-34), `docs/adr/0013-sync-manages-the-allow-rules-too.md`,
  `docs/adr/0014-the-source-declares-the-format-cardinal-must-read.md`,
  `files/.claude/skills/issue-workflow/SKILL.md` (Rules).

# Hints
* A rule on `just release <version>` alone is not enough: a project may redefine the recipe (`set
  allow-duplicate-recipes`).
* A headless run (`claude -p`, `just replay`) cannot answer a prompt: the writing forms fail there, which is the
  intent.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
The agent proposes the release, the user validates, the agent runs it: the managed ask rules
`Bash(cardinal run release *.*)` and `Bash(just release *.*)` make each writing form prompt (in auto mode too: "explicit ask rules still force a
prompt", Claude Code permission modes), the bare proposal stays allowed. Out of scope: pushing.

## Plan
1. [x] Ask before writing the release: `sync` manages `permissions.ask` (entry `ask <rule>`, lock line
   `{"ask": ...}`), `cardinal-format` and `FORMAT` raised to 8 (no migration: no name replaced), the fragment
   asks for the writing forms, the Rules of `issue-workflow` propose then run on the user's go;
   verified by tests of `sync` adding, recording and removing an ask rule, and of the repository fragment asking for
   the writing forms only; makes true SYN-29, SYN-34. (autonomous)
2. [x] Review fixes of the `high` review: the next step `release` prints, the review before the release commit, the
   README, the traceability of the fragment test; verified by `just check`. (autonomous)

## Decisions
- Mode: autonomous
- Approach 1 (user, 2026-11-02).
- The rule is `Bash(cardinal run release *.*)`, not `Bash(cardinal run release *)`: a trailing ` *` as the only
  wildcard also matches the bare command (Claude Code permissions), which would prompt the proposal; a version holds
  a dot (REL-3). `Bash(just release *.*)` too: a rule matches the command as typed, and an unallowed `just release
  <version>` goes to the auto mode classifier, which may run it.
- Base commit: 671382267970c0936a3699c45434c9fcdaf0867e

## Deliverable 1

done: `sync` manages `permissions.ask` (format 8); the fragment asks before `cardinal run release <version>` and
`just release <version>`, with or without `--tag`; the Rules of `issue-workflow` propose, stop, then run on the
user's go.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | SYN-29, SYN-34 and a non-goal of `specs/sync.md` amended |
| docs | `docs/architecture.md`, `docs/usage.md`, `docs/workflow.md`, `README.md`, `CLAUDE.md`, `.claude/update-docs-map.md` |
| code review | low, see below |

red: SYN-29, SYN-34: `sync_appends_a_managed_ask_rule_records_it_and_removes_it_once_dropped`: `left: None right: Some("added")`

Code review: no findings

not verified: a live prompt of Claude Code on the rule (its matching is read from its docs, not run); the installed
`cardinal` reads up to the format 6 already, so `cardinal check` asks to reinstall it (`cargo install --force --path
./crates/cardinal --locked`).

## Deliverable 2

done: `release <version>` prints `cardinal run release <version> --tag` as its next step; the Rules review the
release commit before it; README Releases names the prompt; the fragment test no longer traces SYN-29.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | none amended (REL-6 names no recipe) |
| docs | `README.md`, `issue-workflow` Rules |
| code review | low, see below |

red: none (wording and traceability, no new behaviour)

Code review: no findings

`high` review, 9 findings: fixed the next step of `release`, the release commit's review, the README and the
fragment test's `// spec:`; not fixed:
- a version through a variable or `$(...)` misses `*.*`: the gates catch a cooperative agent's mistakes, not
  bypasses (ADR 0005), and the Rules name the literal form;
- the bare proposal redirected to a dotted file name would prompt: no skill writes it so;
- a project `permissions.ask` that is not an array now leaves the settings unmerged: the shape of SYN-29 includes
  it, as for `allow` and `deny` (SYN-35 warns);
- one `Entry::Rule` variant instead of three: a refactor outside the issue;
- the auto mode prompt not run live: quoted from Claude Code's permission modes page in the Understanding.

not verified: a live prompt of Claude Code on the rules.
