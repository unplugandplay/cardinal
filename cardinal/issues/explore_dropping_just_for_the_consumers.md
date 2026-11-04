---
priority: low
why: 'an exploration: no consumer is blocked, `just` installs in one command, and the outcome may be to keep it'
---
# Explore dropping just for the consumers

# Goal
A project that adopts Cardinal must install `just` next to `cardinal` (`docs/tutorial.md` step 1): the agent reaches
the project's checks and Cardinal's recipes through it. Find out what it would take for a consumer to need only
`cardinal`, what it would cost, and decide whether to do it: the outcome is a measured proposal (keep `just`, make it
optional, or drop it), and the issues that carry it out if any.

# Findings
* 2026-11-02, `grep` of the managed content: the skills of `files/.claude/skills/` name `just <recipe>` about 26 times
  (`just check`, `check-issue`, `next-issue`, `close-issue`, `check-specs`, `spec-coverage`, `check-cardinal`,
  `open-worktree`, `release`, `check-commit`), plus `files/specs/README.md`.
* Every recipe of `cardinal*.just` calls `cardinal` (`cardinal run <tool>`, `cardinal run worktree open`,
  `cardinal check`), except `waive-review`, a shell script that creates the waiver file.
* `check` is the project's own recipe of the seeded `justfile` (format, lint, tests, then `check-specs`): the one
  entry point the agent runs before each review, and the only recipe with no `cardinal` equivalent.
* The review waiver is kept from the agent by the deny rules `Bash(just waive-review)` and
  `Bash(just waive-review *)` of `entries/.claude/settings.json`; `rule-breaches` detects `just` + `waive-review`
  (`tools/rule_breaches.rs`).
* `delivered-quality` counts a failed check as a `just check*` or `just test*` command (`tools/delivered_quality.rs`).
* `files/.github/workflows/cardinal.yml` installs `just` to run `just check-specs` and `just check-commits`.
* The seeded `.claude/settings.json` allows `Bash(just)` and `Bash(just *)`, and `docs/tutorial.md` tells the project
  to run the commands of its stack through `just`.

# Approach
Measure first, then propose; nothing is decided here.
1. List every place that needs `just` (skills, specs README, settings entries, CI, the readers of the transcripts,
   docs) and what each would become without it.
2. Candidates, which can be combined:
   1. Keep `just` and only document it (done in `docs/tutorial.md`): no cost.
   2. The skills name `cardinal run <tool>` instead of the recipes, and a `[project]` key (`check_command`, as
      `lint_command`) names the project's check; `just` stays an option of the project.
   3. A `cardinal waive-review` (or a tool) with its own deny rule, so the waiver no longer needs a recipe.
3. For each, weigh: the fixed context of the skills (`cardinal run` is longer than `just`), the deny rules and the
   readers of the transcripts that match `just` (a `rule-breaches` or `delivered-quality` series cut in two), the
   migration of the consumers (ADR 0017, `migrations/<n>.toml` for a renamed command), and what a project loses
   (`just --list`, its own recipes).
4. Write the proposal (an ADR draft when it changes a decision) and, on the user's agreement, the issues.

# Resources
* `cardinal.just`, `cardinal-*.just`, `seed/justfile`, `seed/.claude/settings.json`, `entries/.claude/settings.json`.
* `files/.claude/skills/*/SKILL.md`, `files/specs/README.md`, `files/.github/workflows/cardinal.yml`.
* `crates/cardinal/src/tools/rule_breaches.rs`, `crates/cardinal/src/tools/delivered_quality.rs`.
* `docs/tutorial.md` step 1, `docs/usage.md` ("Features a project turns off"), `docs/gates.md` (the review gate).
* ADR 0001 (one harness synced by a Rust tool), ADR 0005 (the gates catch mistakes), ADR 0017 (migrations).
* [add_a_just_recipe_to_waive_the_review](done/add_a_just_recipe_to_waive_the_review.md): why the waiver is a recipe.

# Hints
* A deny rule matches the command text: a new way to waive needs its own rule, or the agent can waive its review.
* The readers of the transcripts compare revisions: changing the command they match cuts a series unless both forms
  are read.
