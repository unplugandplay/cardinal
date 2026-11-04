# Rename the harness-named tools to cardinal

# Goal
Rename what still carries "harness" as the product name once the binary and files are renamed: the skill
`evaluate-harness`, the tools `harness-report` and `harness-changes`, the spec `harness-verdicts`, the commit scope
`harness`, the CI workflow `harness.yml` and the recipe `check-harness`.

# Findings
* `evaluate-harness` appears in 25 lines of 14 files, `harness-report` in 28 lines of 16 files, `harness-changes`
  in 21 lines of 12 files, `harness-verdicts` in 5 lines.
* `AGENTS.md` makes `harness` a scope that always exists (`.claude/`, `justfile`, CI); `check-commits` enforces it,
  and `harness-changes` may read past commits by it.
* `CLAUDE.md` defines the `Expected-effect` footer against the metrics of `harness-report`.

# Approach
1. Decide per name: rename to `cardinal-…`, or keep the generic noun where it means the concept (e.g.
   `evaluate-harness` evaluates an agent harness and may stay).
2. Keep reading the old commit scope in the history tools, so past commits still count.
3. `git mv` in a deliverable of its own, then the references, then `just sync`.

# Specs
* `specs/harness-report.md`, `specs/harness-verdicts.md` and the specs citing these tools; renamed or amended at
  `issue-workflow` step 1.

# Resources
* `files/.claude/skills/evaluate-harness/`, `crates/cardinal/src/tools/harness_report.rs`, `harness_changes.rs`
* `AGENTS.md`, `files/.github/workflows/harness.yml`, `justfile`

# Hints
* Also the product prose that `name_the_product_cardinal` left out of its scope: the specs (e.g. `specs/sync.md`
  "keeps the shared harness", `specs/harness-verdicts.md` "harness source") and the doc comments of the crate
  (`lib.rs`, `tools/harness_changes.rs`).
* After `rename_the_consumer_files_to_cardinal`.
* Renaming a skill changes the fixed context of every session and the skill names counted by `harness-report`:
  past sessions cite the old name.

# Tracking

Plan: validated

## Understanding
Rename to `cardinal-…` what names the product: the tools `harness-report`/`harness-changes` (and their modules,
tests, specs `harness-report.md`/`harness-verdicts.md`, ids REP/VRD kept), the commit scope, the CI workflow, the
recipe `check-harness`, and the product prose of the specs and doc comments. Out of scope: the `harness` binary
alias and the old-name fallbacks (`drop_the_harness_alias`), the gate mark `# harness: native review gate`.
No `FORMAT` raise: no fragment key, lock line or source layout changes meaning; `sync` deletes the old copies (SYN-5).

## Plan
1. [x] Moves: `git mv` of `specs/harness-{report,verdicts}.md` to `specs/cardinal-{report,verdicts}.md`,
   `tools/harness_{report,changes}.rs` and `tests/harness_{report,changes}.rs` to `cardinal_*`,
   `files/.github/workflows/harness.yml` to `cardinal.yml`, with only the `mod`/`use` lines and links that keep
   `just check` green, then `just sync`; verified by `just check`, `just check-harness`; ~60 lines + renames.
2. [x] Tool names: `cardinal run cardinal-report` and `cardinal-changes` (registry, usage strings, skills, docs,
   CLAUDE.md `Expected-effect`); verified by the tools CLI test through the new names and `tests/skills.rs`;
   makes true REP-3, VRD-3, VRD-5, VRD-7, VRD-8 amended; ~150 lines.
3. [x] Commit scope `cardinal`: `commit-check` built-in scopes `docs` and `cardinal`; `rule-breaches` reads
   `docs`, `cardinal` and `harness` (past commits); AGENTS.md, the `cardinal.toml` comments; verified by a
   commit-check test (`cardinal` accepted, `harness` refused) and a rule-breaches test (a past `harness` commit is no
   breach); makes true BRE-9 amended; ~80 lines.
4. [x] CI and recipe: workflow `Cardinal` (job, concurrency group), recipe `check-cardinal`, `ci.yml`,
   `install_cardinal.sh`, README, docs, skills; verified by `just check-cardinal`, `just check-harness` (alias);
   ~80 lines.
5. [x] Product prose: "the harness" meaning Cardinal becomes "Cardinal" in the specs (`sync.md`, REP, VRD, EVA,
   BIN titles and intents) and the crate doc comments and tool about strings; the generic "agent harness" stays;
   verified by `just check`; ~120 lines.
6. [x] Fixes of the `high` review: `check-harness` a recipe depending on `check-cardinal` (an alias broke a
   justfile overriding it); `rule-breaches` reuses the built-in scopes of `commit-check` plus the old one.

Specs amended in place (ids kept): REP-3, VRD-3, VRD-5, VRD-7, VRD-8 (tool names) in deliverable 2, BRE-9 (scope
`cardinal`, `harness` still read) in deliverable 3; prose only in deliverable 5.

## Decisions
- Mode: autonomous
- Base commit: e4cd7fcb4252a4e1989cdbe2768d983af7d15124
- `evaluate-harness` kept (the generic noun; its skill counts stay whole).
- `commit-check` refuses the scope `harness` (only `docs` and `cardinal`); `rule-breaches` still reads it.
- Workflow `cardinal.yml`, named `Cardinal`, job `cardinal`; a consumer's required check `harness` must be renamed (final report).
- Recipe `check-cardinal`, `alias check-harness` kept until `drop_the_harness_alias`.
- No alias for the old tool names (skills and binary are pinned together).

## Deliverable 1: moves
- done: the specs, tool modules, tool tests and the CI workflow moved to their `cardinal` names; links and `mod`
  lines follow; `sync` replaced `.github/workflows/harness.yml` by `cardinal.yml` (SYN-5).

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| copies | `just check-harness` passes |
| docs | spec paths updated (CLAUDE.md, architecture, update-docs map, doc comments) |
| code review | low, see below |

- red: none (moves only, no behaviour)
- Code review: no findings
- not verified: the GitHub run of the renamed workflow (runs on push)
- decisions needed: none

## Deliverable 2: tool names
- done: the tools answer to `cardinal-report` and `cardinal-changes` (registry kept in alphabetical order, usage
  strings, `evaluate-harness`, specs REP-3 and VRD-3/5/7/8, CLAUDE.md, docs).

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| copies | `just sync` then `just check-harness` pass |
| docs | usage, workflow, architecture, update-docs map, CLAUDE.md |
| code review | low, see below |

- red: REP-3, VRD-3, VRD-8: `cardinal: unknown tool \`cardinal-changes\``; `tests/skills.rs`: 2 failures (the skill
  named the old tools)
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3: commit scope
- done: `commit-check` (and `check-commits`) accept `docs` and `cardinal`, refuse `harness`; `rule-breaches` reads
  `docs`, `cardinal` and `harness` (BRE-9 amended); AGENTS.md (scope and example) and the `cardinal.toml` comments.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| copies | `just sync` then `just check-harness` pass |
| docs | AGENTS.md, seed and project `cardinal.toml` comments, BRE-9 |
| code review | low, see below |

- red: BRE-9: a `fix(cardinal): v` commit counted as a `commit-format` breach; commit-check: `unknown scope \`cardinal\``
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 4: CI and recipe
- done: workflow `Cardinal` (concurrency group and job id `cardinal`), recipe `check-cardinal` with
  `alias check-harness`, `ci.yml`, README, CLAUDE.md, docs, `issue-workflow`; the alias added to
  `drop_the_harness_alias`. The job's display name `agent harness checks` is kept: GitHub names a required check by
  it, so a consumer's branch protection keeps working (decision 3 needs no action there).

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| recipes | `just check-cardinal` and the alias `just check-harness` pass |
| docs | README, CLAUDE.md, usage, workflow, `issue-workflow` |
| code review | low, see below |

- red: none (recipes and CI, no test)
- Code review: no findings
- not verified: the GitHub run of the workflow (runs on push)
- decisions needed: none

## Deliverable 5: product prose
- done: "the harness" naming the product became "Cardinal" in the spec titles and intents (sync, report, verdicts,
  evaluation, rule breaches, tool binaries, delivered quality, clippy gate), the crate doc comments, the tool about
  strings, and the binary named in the test doc comments (`cardinal tool <name>`); "the shared agent harness" (the
  generic noun) and the old-name compatibility (`[harness]`, `harness run`, the gate mark) stay.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| copies | `just sync` changes nothing |
| docs | specs and doc comments are the change |
| code review | low, see below |

- red: none (prose only)
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 6: fixes of the high review
- done: `check-harness` is a recipe running `check-cardinal`, not an alias (just refuses an alias a consumer's
  justfile redefines as a recipe, even under `allow-duplicate-recipes`); `rule-breaches` reads
  `commit_check::BUILT_IN_SCOPES` plus `OLD_SCOPE` instead of a copy.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| recipes | `just check-cardinal`, `just check-harness` pass; a justfile overriding `check-harness` parses |
| docs | `drop_the_harness_alias` names the recipe |
| code review | high on `e4cd7fc..HEAD`, then low on the fixes |

- red: none (the collision reproduced by hand: `recipe check-harness ... is redefined as an alias`)
- Code review: 9 findings (high). Fixed: the alias collision, the duplicated scope list. Kept, decided at step 2:
  `check-commits` refuses `harness` in an open PR's older commits (decision 2), `rule-breaches` accepts both scopes
  in every revision (Approach 2). Outside this issue: 5 findings on `e5352ff` (clippy pedantic, not of this
  issue: `is_git_url`, `total_cmp`, `check_specs` closure, `review_gate` concat, `budget` let-else), reported to
  the user. Low review of the fixes: no findings.
- not verified: none
- decisions needed: none
