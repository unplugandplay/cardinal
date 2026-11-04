---
priority: medium
why: 'blocks the adoption by users whose flow differs, but no such user is waiting yet'
---
# Let a project turn off a feature of the harness

# Goal
Every consumer gets the whole harness: each hook, rule, skill, check and managed file. A user whose flow differs
(an AI co-author trailer wanted, another commit convention, no parallel agents, issues tracked elsewhere) cannot
drop the part that does not fit: removing a managed hook is a conflict at the next `sync` and a `missing` line in
`check`, editing a managed file is a drift. The outcome: a project names the features it turns off in
`.claude/cardinal.toml`, and the harness of that project costs no more tokens nor time than before, less when a
feature is off. The ban on AI attribution in commits is the pilot feature; the others follow in their own issues.

# Findings
* A managed entry removed from the settings is a conflict at `sync` (SYN-33) and `missing:` in `check` (SYN-36).
* The attribution ban lives in four places: the `ATTRIBUTION` list of `crates/cardinal/src/tools/commit_check.rs`,
  the line that forbids co-author trailers of `files/AGENTS.md`, the `attribution` warning of `check` (SYN-26,
  `crates/cardinal/src/settings.rs`), and the breaches counted by `rule-breaches` (`specs/rule-breaches.md`).
  `check_issue.rs` mentions it too (to be read).
* `commit-check` already reads `.claude/cardinal.toml` (`commit_scopes`): a key read there costs nothing more.
* Candidate features found in the source, each a group of files, settings entries and checks:

  | Feature | What it is made of |
  |---|---|
  | AI attribution ban | `commit-check` list, `AGENTS.md` line, SYN-26 warning, `rule-breaches` count |
  | Conventional Commits | `check-commit`, `check-commits`, CI job, `AGENTS.md` section, `commit-message` skill |
  | Review gate | `review-gate` hooks, native git hook, `waive-review` deny rules |
  | Workspace lock and issue claim | 3 `workspace-lock` hooks, `issue-claim` |
  | Lint at Stop | `lint-gate` hook |
  | Format on edit | `format-code` hook |
  | Language hint | `language-hint` hook |
  | Issues in `issues/` | `create-issue`, `prioritize-issues`, `issue-workflow`, `next-issue`, `close-issue`, `check-issue` |
  | Specs and tracing | `write-spec`, `check-specs`, `spec-coverage`, `specs/` |
  | Harness measure | `evaluate-harness`, `audit-instructions`, SYN-28 warning |
  | GitHub CI | `.github/workflows/cardinal.yml`, `install_cardinal.sh` |
  | Report style | `.claude/rules/reports-to-the-user.md` |

# Approach
Decide at `sync` time, not at run time, so that a feature turned off costs nothing:
1. The source declares its features (a file at its root, e.g. `features.toml`): for each, its managed files, its
   settings entries, the checks of the binary it gates, and the features it needs (`issue-workflow` needs
   `commit-message` and the review gate).
2. The project lists what it turns off (e.g. `disabled = ["ai-attribution-ban"]` in `.claude/cardinal.toml`).
   `sync` does not copy those files nor add those entries, removes the ones it installed before, and records the
   exclusion in the lock so that `check` reports neither a missing file nor a missing entry. A feature needed by one
   left on is refused with the reason.
3. A check of the binary that a feature gates reads the key from the configuration it already reads; a hook is
   never left running to test a flag (a process per tool call): its entry is removed instead.
4. Text of a managed file that belongs to a feature: either move it to a file of its own (copied or not, like the
   skills), or strip marked blocks at `sync` and hash the rendered text in the lock. The first is simpler; the
   `AGENTS.md` attribution line is the test case.
5. An older `cardinal` would ignore the key and reinstall everything: raise `cardinal-format` and `FORMAT` (ADR
   0014), declare a migration if a name changes (ADR 0017), and write an ADR for the decision.
6. Pilot with the attribution ban only (its four places), then one issue per other feature, each small.

# Specs
* `specs/features.md` (new, FEA-1 to FEA-13); SYN-26 and BRE-9 are narrowed by FEA-10 and FEA-11, not reworded.

# Milestone
Optional features

# Resources
* `docs/architecture.md`, ADR 0007, 0013, 0014, 0017, 0018.
* `crates/cardinal/src/entries.rs`, `crates/cardinal/src/settings.rs`, `crates/cardinal/src/tools/commit_check.rs`,
  `crates/cardinal/src/tools/rule_breaches.rs`.
* `entries/.claude/settings.json`, `files/AGENTS.md`, `seed/.claude/cardinal.toml`.

# Hints
* The user's own projects keep every feature on: the default is all on, and turning one off is the project's choice.
* `cardinal.local.toml` holds personal preferences (ADR 0018); whether a person may turn off a feature for
  themselves only (the language hint) or only the project may (the review gate, a team rule) is a decision of step 1.
* The fixed context budget (SYN-38) should drop when a skill is turned off: a quick way to show the gain.

# Tracking

Plan: validated

## Understanding
A source declares features in `features.toml`; a project turns some off with `disabled` in `[cardinal]`; `sync` treats
their files and entries as unmanaged, the binary's checks of a feature stay quiet. Pilot: `no-ai-attribution`. Out of
scope: the other features, dependencies between features, a personal opt-out.

## Plan
1. [x] Turn off a feature at sync: `features.toml`, `disabled`, files and entries left out, errors, format 7, ADR
   0025; verified by sync and check tests through the binary; makes true FEA-1 to FEA-7. ~600 lines.
2. [x] Feature `no-ai-attribution`: the rule moves from `files/AGENTS.md` to `files/.claude/rules/no-ai-attribution.md`,
   declared in `features.toml`, and `commit-check`, `check-issue`, the SYN-26 warning and rule `commit-attribution`
   read `disabled`; verified by the CLI tests of each tool and a test of the source; makes true FEA-8 to FEA-13.
   ~400 lines.
3. [x] Docs of the whole issue and fixes of its `high` review; makes true FEA-14, FEA-15.

## Decisions
- Mode: autonomous
- Base commit: 8e94d11bac17a61fb4fce098245715bd55c5edc3
- An unknown name in `disabled` fails the sync (FEA-5): a typo must not leave a feature on silently.
- Only the project turns a feature off (`.claude/cardinal.toml`), not the personal file: `sync` writes committed files.
- A text of a shared file that belongs to a feature moves to a file of its own, not marked blocks stripped at sync.
- The milestone `Optional features` comes first among the open ones.

## Deliverable 1
- done: `features.toml` read by `sync`, `disabled` of `[cardinal]`, the files and entries of a feature turned off left
  out (removed, kept when edited, out of the lock), errors for an unknown name and a bad declaration, format 7.
- red: FEA-1, FEA-2, FEA-3, FEA-5, FEA-6, FEA-7: 7 of the 8 tests of `tests/features_cli.rs` failed with the call to
  `features::left_out` replaced by an empty `Left` (the file still added, `sync` exit 0 on an unknown name); FEA-4
  holds by construction (nothing out of the lock is reported) and its test passed red too.
- not verified: a real consumer with a git source synced to format 7.

## Deliverable 2
- done: the rule moved to `files/.claude/rules/no-ai-attribution.md`, declared in `features.toml`; `commit-check`,
  `check-commits` (FEA-8 widened to it: the CI would refuse the trailer otherwise), `check-issue`, the SYN-26 warning
  and rule `commit-attribution` read `disabled`; spec accepted.
- red: FEA-8, FEA-9, FEA-10, FEA-11, FEA-12, FEA-13: each new test failed before the code (attribution reported by
  `commit-check` and `check-issue`, the SYN-26 warning printed, the `commit-attribution` row present, no
  `features.toml`).
- not verified: a consumer that turns the feature off and commits with a co-author trailer through Claude Code.

## Deliverable 3
- done: `features.toml` in the architecture diagram and the docs map; `update` no longer refuses a hook only a feature
  turned off runs (FEA-14); a managed path of a feature turned off is never seeded (FEA-15); `rule-breaches` reads the
  configuration of a commit once; `[cardinal] disabled` in the architecture text.
- red: FEA-14, FEA-15: `update` exited 3 naming `cardinal newcmd`; `sub/b.txt` was seeded back after its removal.
- Code review (high): 8 findings. Fixed: 2 (update), 3 (seed), 5 (two `git show`), 8 (architecture). Not fixed:
  1 (the rule left `AGENTS.md`, read by agents other than Claude Code: decided at step 2, Cardinal targets Claude Code,
  and `commit-check` and CI still refuse the trailer), 4 (`check-issue` takes the root from the issue path: it has no
  git repository in its tests, and the path is `issues/<name>.md` by its own rules), 6 (a malformed `disabled` read as
  absent outside `sync`: the ban stays on, and `sync` fails loudly on it), 7 (one more `git rev-parse` in
  `commit-check`: a few milliseconds per commit).
