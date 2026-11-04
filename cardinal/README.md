<p align="center"><img src="docs/logo/cardinal-wordmark.svg" alt="Cardinal, set the course for your agents" width="480"></p>

# Cardinal

Cardinal is one agent harness for many projects. This repository holds the Claude Code skills, hooks, checks, `just`
recipes and CI that a project needs to have an agent work its issues safely, plus `cardinal`, the tool that copies them
into each project and tells when a copy drifted.

## What a project gets

| | |
|---|---|
| Skills | `issue-workflow` (an issue driven to done in validated deliverables), `create-issue`, `prioritize-issues`, `write-spec`, `update-docs`, `commit-message`, `evaluate-harness`, `audit-instructions` |
| Rules | `.claude/rules/user-language.md`: the language of the conversation, `.claude/rules/reports-to-the-user.md`: the format of the reports to the user, and `.claude/rules/no-ai-attribution.md`, loaded in every session with no edit of the project's `CLAUDE.md` |
| Optional features | `disabled` in `.claude/cardinal.toml` turns off a feature of `features.toml` (`no-ai-attribution`, `conventional-commits`, `lint-gate`, `format-code`, `language-hint`, `read-hint`, `review-gate`, `harness-measure`, `report-style`, `github-ci`, `issue-files`, `specs`, `workspace-lock`): `sync` removes its files and entries |
| Gates | no commit without a code review of the diff, no `git add` of every file, no turn ending on a warning of the project's lint (clippy by default), a reply not in the `user_language` sent back once, a whole read of a large file sent back once, the project's formatter (`cargo fmt` by default) after each edit, one Claude Code session per worktree; commit messages checked (Conventional Commits, no AI attribution, each a feature) |
| Recipes | `cardinal.just` (`open-worktree`, `check-cardinal`, `release`…), which imports the recipes of each part a feature may leave out when present: `cardinal-issues.just` (`next-issue`, `close-issue`, `check-issue`), `cardinal-specs.just` (`check-specs`, `spec-coverage`), `cardinal-commits.just` (`check-commit`, `check-commits`), `cardinal-review.just` (`waive-review`) |
| Releases | `just release`: the next semver proposed from the commits, its `CHANGELOG.md` section written to reread, then a local `v<version>` tag; an agent writes the section and the tag only with the user's permission (managed ask rules) |
| CI | `.github/workflows/cardinal.yml`: the copies and the specs checked on main and on each pull request, plus its commit messages |
| Evaluation | `cardinal run session-metrics`, `rule-breaches`, `delivered-quality`…: did a Cardinal revision make the agent better? |

```mermaid
flowchart LR
  subgraph source["cardinal (source)"]
    files["files/<br/>managed"]
    seed["seed/<br/>project-owned"]
    entries["entries/<br/>hooks, allow, ask and deny rules"]
  end
  sync(["cardinal sync"])
  subgraph project["your project"]
    copies["same paths,<br/>checked by cardinal check"]
    owned["copied once,<br/>then yours"]
    settings[".claude/settings.json"]
    gate[".git/hooks/pre-commit<br/>review gate"]
  end
  source --> sync
  sync -->|managed| copies
  sync -->|if missing| owned
  sync -->|entries merged| settings
  sync -->|"installed, in each clone"| gate
```

## Install

```sh
cargo install --force --path crates/cardinal --locked   # from a clone
cd my-project
cardinal init <git-url-or-path> [--rev main]    # writes .claude/cardinal.toml
cardinal sync                                   # copies, seeds, merges the settings, installs the review gate
```

Then keep the Claude Code transcripts a year, the evaluation reads them:
`{ "cleanupPeriodDays": 365 }` in `~/.claude/settings.json`.

## Commands

| Command | Does |
|---|---|
| `cardinal init <source> [--rev <rev>]` | points the project at a source (a directory or a git URL) in `.claude/cardinal.toml` |
| `cardinal sync [--dry-run] [--force]` | brings the project in step with its source |
| `cardinal update [--rev <rev>] [--dry-run] [--force]` | moves a git source to a revision, syncs, pins the commit |
| `cardinal check` | fails when a copy drifted, the review gate is missing or a hook needs a newer `cardinal` |
| `cardinal upgrade <source> <commit>` | replaces the installed `cardinal` with the one of a commit: its release binary, checked against its SHA-256, else built with `cargo` |
| `cardinal run [<tool>]` | runs a tool of the pinned revision (hooks, checks, transcript readers); alone, lists them |

## Daily flow

```mermaid
flowchart LR
  create["create-issue"] --> queue["priority of each<br/>issues/NAME.md"]
  queue -->|just next-issue| work["issue-workflow"]
  work -->|plan validated| deliver["test, code, docs,<br/>review, commit"]
  deliver -->|each deliverable| deliver
  deliver --> close["just close-issue"]
```

Tell the agent `issues/<name>.md` (or "the next issue"); it prepares a plan, stops once for your validation, then
delivers. Details: [docs/workflow.md](docs/workflow.md).

## Read next

- [docs/tutorial.md](docs/tutorial.md): a first walk through, from an empty project to a first issue delivered.
- [docs/usage.md](docs/usage.md): every command in detail, the evaluation, parallel worktrees.
- [docs/workflow.md](docs/workflow.md): how an issue goes from idea to merge, alone or in parallel.
- [docs/architecture.md](docs/architecture.md): how Cardinal is built; [docs/gates.md](docs/gates.md) and
  [docs/evaluation.md](docs/evaluation.md) hold the detail of its hooks and of its evaluation.
- [specs/](specs/): what each capability must do; [docs/adr/](docs/adr/): why it is so.
