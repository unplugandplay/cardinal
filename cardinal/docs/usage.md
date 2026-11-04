# Usage

The reference the [README](../README.md) leaves out. `cardinal help <command>` and `cardinal run` (the list of the
tools) say the same from the binary.

## Commands

```sh
cardinal init <source> [--rev <rev>]  # writes .claude/cardinal.toml; <source>: a directory (relative to the project) or a git URL
cardinal sync [--dry-run] [--force]
cardinal update [--rev <tag|branch|commit>] [--dry-run] [--force]
cardinal check
cardinal run <tool> [args]
cardinal upgrade <source> <commit>
```

| Command | What it does |
|---|---|
| `sync` | copies the managed files (`files/`), seeds the missing project-owned ones (`seed/`), adds the hooks and the allow, ask and deny rules of Cardinal to `.claude/settings.json` (or removes those the source dropped), installs the git `pre-commit` review gate (removes it, a chained hook of yours put back, once the project turns off `review-gate`). A managed file edited locally is a conflict: fix it in the source, or `--force` to overwrite it. |
| `update` | git source only: resolves the revision (default: the head of the default branch), syncs to it, then pins the commit in `rev`. It refuses a revision whose hooks run a `cardinal` subcommand the installed one does not know, and names the install to run first (see below). Once it wrote a change, it ends on `next: commit the update with skill:commit-message` when the project has that skill. |
| `check` | fails when a managed file was modified or deleted, the git review gate is missing (or cannot be installed over a `pre-commit.user` of yours: merge it by hand), or a hook runs a `cardinal` subcommand the installed one refuses. It warns when `.claude/settings.json` lacks a deny rule of the review waiver, runs a missing hook or leaves the attribution on (unless the project turns off `no-ai-attribution`), and when Claude Code keeps the transcripts less than a year (`sync` and `update` warn too; unless the project turns off `harness-measure`). |
| `upgrade` | replaces the first `cardinal` of the `PATH` (a symlink, not its target) with the one of a full commit hash of a git source, fetched as `run` fetches it (the release asset checked against its SHA-256, else a `cargo` build), with no configuration needed; it refuses a short hash or a local source and then leaves the installed one as it is. |
| `run` | runs a tool of Cardinal with the binary of the pinned revision, fetched (release asset) or built once into the cache; `cardinal run` alone lists the tools. |

The seeded `.claude/settings.json` allows no command of a stack: `just` and the read-only `git` commands, next to
the rules `sync` manages (`cardinal run`, the deny rules of the review waiver). The project adds the rules of its own
stack (`cargo test`, `npm test`…), or runs them through `just`. A project seeded before allows `cargo build`, `cargo
test`, `cargo clippy`, `cargo fmt` and `cargo check`: it owns the file, so a project of another stack removes them by
hand.

The seeded `roadmap` key names `ROADMAP.md`, the milestones `skill:create-issue` and `skill:prioritize-issues` keep. A
project seeded before names `docs/TODO.md`, which nothing seeds: `sync` copies `ROADMAP.md` when it is missing, and
the project points its key at it by hand.

The seeded `user_language` is the default language of the conversation. Each person may set their own in a
`[project]` table of `.claude/cardinal.local.toml`, which the managed `.claude/.gitignore` keeps out of git: the
`language-hint` hook, rule `user-language` of `rule-breaches` and the skills read it first, and `open-worktree`
copies it into each worktree it opens.

With a local directory source (this repository), `cardinal run` builds the source's crate with `cargo`, so a hook runs
the tools of the working tree, or of the last build that compiled.

### Features a project turns off

The source groups some managed files and settings entries into named features in `features.toml`
([specs/features.md](../specs/features.md), ADR 0025):

```toml
[no-ai-attribution]                      # a feature, by name
files = [".claude/rules/no-ai-attribution.md"]   # paths under files/
entries = []                             # entries as `sync` prints them, e.g. "hook Stop cardinal run language-hint"
needs = []                               # features it needs, left on while it is
```

A project turns features off in the `[cardinal]` table of `.claude/cardinal.toml`, never in the personal file:

```toml
[cardinal]
source = "..."
disabled = ["no-ai-attribution"]
```

`sync` then handles their files and entries as ones the source does not manage: it removes what it installed (and
keeps a file the project edited, reported `kept`), installs nothing of them and leaves them out of the lock, so
`check` reports nothing. Removing a name from `disabled` adds them back at the next `sync`. A name the source does not
declare fails `sync`, writing nothing, as does a `disabled` that turns off a feature one left on `needs`: the error
names both, and turning both off is accepted.

| Feature | Turns off |
|---|---|
| `no-ai-attribution` | the rule against AI attribution and the checks of the binary that enforce it |
| `conventional-commits` | the commit convention (`commit-message/conventional-commits.md`), its checks in `commit-check` and `check-commits`, and rule `commit-format` of `rule-breaches`; the attribution and `Expected-effect:` checks stay, and `release` reads no commit type (see Releases) |
| `lint-gate` | the Stop hook that runs the project's lint, for a lint that runs in CI |
| `format-code` | the hook that formats each edited file, for an editor that formats |
| `language-hint` | the Stop hook that sends a reply back when it is not in the `user_language`; rule `user-language` of `rule-breaches` still applies |
| `read-hint` | the PreToolUse hook that sends the agent back once before a whole `Read` whose result would hold more than 25,000 bytes; rule `oversized-result` of `rule-breaches` still applies |
| `review-gate` | the review gate, for a project that reviews in pull requests or a solo user: its two hooks (the refusals of `--no-verify` and `git add -A` with them), the deny rules of the waiver, `waive-review`, the native git `pre-commit`, which `sync` removes, a chained hook of yours put back; rules `no-verify`, `stage-everything`, `review-waiver` and `review-before-commit` of `rule-breaches` skip the sessions that start once the change is committed on the default branch |
| `harness-measure` | the skills `evaluate-harness` and `audit-instructions`, for a project that does not measure Cardinal, and the warning on the retention of the transcripts the user settings keep |
| `report-style` | the format of the reports to the user (`.claude/rules/reports-to-the-user.md`); the language of the conversation stays (`.claude/rules/user-language.md`) |
| `github-ci` | the GitHub CI job (`.github/workflows/cardinal.yml`) and its installer, for a project not on GitHub |
| `issue-files` | the issue files, for a project tracking its issues in another tool: the skills `issue-workflow`, `create-issue` and `prioritize-issues` and the recipes of `cardinal-issues.just` (`next-issue`, `close-issue`, `check-issue`); the gates, the specs and the commit checks stay, the seeded `ROADMAP.md` too, and the project writes its own workflow skill; `issues/` is then a directory like any other: the review gate reviews it, `deliverable-size` counts it, `review-target` keeps it, and `rule-breaches` applies rule `validation-stop` to no session |
| `specs` | the specs and their tracing, for a project that writes no spec: `specs/README.md`, `specs/TEMPLATE.md`, the skill `write-spec` and the recipes of `cardinal-specs.just` (`check-specs`, `spec-coverage`); `issue-workflow` then skips its spec steps, the CI and the seeded `check` skip `check-specs` without `cardinal-specs.just`. A `justfile` seeded with `check: check-specs` is the project's: drop the dependency before turning the feature off, or `just` refuses the whole file |
| `workspace-lock` | the three hooks of the worktree lock, for a user who never runs two sessions in one worktree: the hook call of each tool call goes; `issue-claim` stays, so `next-issue` still skips an issue another worktree works |

### The installed launcher

The installed `cardinal` is the launcher of every project of the machine, not a binary of the pinned revision. A
revision whose hooks call a subcommand it does not know needs its `cardinal` installed first, or every hook refuses and
the agent can run nothing, the install included. `cardinal update` refuses such a revision and prints the install to
run: `cardinal upgrade <source> <commit>` for a git source at a full commit, which needs no Rust when the source
publishes the binary of that commit (a GitHub source, a commit of its main branch), else a `cargo install` (BIN-20).
`just check-cardinal` says so before an issue branch is merged (`issue-workflow`, "Merge back").

### Formats and migrations

A source declares the format of its managed content in `cardinal-format`, a whole number (ADR 0014). An older
`cardinal` refuses what it would misread, writes nothing and prints the install to run:

| Refused by | When | Spec |
|---|---|---|
| `sync`, `update` | the source is of a format above the one the installed `cardinal` reads | SYN-40 |
| `check`, `sync`, `update` | the lock was synced from a source of such a format | SYN-42 |
| `init`, `sync`, `update` | the source is below the format 5, whose tools order the issues by `issues/queue.md` | SYN-51, ADR 0020 |

The files of a project are `.claude/cardinal.toml` (table `[cardinal]`), `.claude/cardinal.lock` and `cardinal.just`
(which imports the `cardinal-<part>.just` files).

A source that raises its format declares what changed in `migrations/<n>.toml`, `<n>` the new format (ADR 0017):

```toml
[[names]]            # a managed name the format replaced
old = "check-old"
new = "check-new"

[[rewrite]]          # a seeded text of a project file, matched as whole lines
path = "justfile"
old = "check: check-old"
new = "check: check-new"
```

`sync` and `update` apply each migration above the format of the lock, up to the one of the source, in increasing
format (SYN-52); one that cannot be read fails them, writing nothing.

- Each `rewrite` replaces its text where the project left it as seeded, printed `rewritten: <path>`, and leaves a
  line the project changed as it is (SYN-53).
- Then each line of a file git tracks, outside the managed files, `issues/done/` and `docs/adr/`, that still cites
  an old name of `names` as a word is printed on stderr as `warning: <path>:<line> cites <old>, named <new> since the
  format <n>`, for the agent or the user to edit. Only the run that applies the migration prints it (SYN-54).
- Until a project with a git source moves, `check` and `sync` print `hint: .claude/cardinal.lock is of the format
  <a>, this cardinal reads the format <m>: run `cardinal update` …` when its lock is below the format the installed
  `cardinal` reads (SYN-55).

## Size of a deliverable

`cardinal run deliverable-size [<rev> | <a>..<b>]` ([specs/deliverable-size.md](../specs/deliverable-size.md)) counts
the added plus removed lines, untracked files included, outside `issues/` and the lock files: with no argument the
working tree against `HEAD`, against `deliverable_size_limit` (1000); with a revision or a range, against
`issue_size_limit` (3000), both in the `[project]` table of `.claude/cardinal.toml`. A rename counts 0 unless the file
also changed: then its whole old and new content count. It exits 1 over the limit; `issue-workflow` cuts the
deliverable then.

## Target of a code review

`cardinal run review-target [<base>]` ([specs/review-target.md](../specs/review-target.md)) prints the target
`issue-workflow` hands to `/code-review`: the working tree against `HEAD` or `<base>`, then `-- ':/'` and one
`':(top,exclude,literal)<path>'` for `issues/`, `.claude/cardinal.lock` and each managed file it lists, so the
reviewers diff the code, tests, specs and docs and not the issue tracking nor the managed copies. It refuses an
untracked file outside those paths, which `git diff` would leave out, printing the `git add -N` that marks it
intent-to-add. The commit gate still records the whole diff as reviewed.

## Releases

`just release` (`cardinal run release`, [specs/release.md](../specs/release.md)) numbers the versions of a project
from its Conventional Commits, in three steps; an agent runs the last two only with the user's permission (the managed
ask rules `Bash(cardinal run release *.*)` and `Bash(just release *.*)`, auto mode included):

| Step | What it does |
|---|---|
| `just release` | prints the proposed version: from the last `vX.Y.Z` tag, the major raised by a breaking commit (`!`, a `BREAKING CHANGE:` footer, or a file of `release_breaking_files`), else the minor by a `feat`, else the patch; `0.1.0` with no tag yet |
| `just release <version>` | refuses a version lower than the proposed one, or any when no commit follows the last tag; writes its section of `CHANGELOG.md` (Milestones, Breaking, Added, Fixed; `First numbered release.` with no tag yet), adds `- Released: v<version>` to the `done` milestones of the `roadmap` documents that had none, runs `release_version_command` with the version as last argument. Reread and edit the section, then commit it. |
| `just release <version> --tag` | once the release is committed, creates the annotated tag `v<version>` on `HEAD`, the section as its message, and prints `git push origin v<version>`: the push is the user's |

A consumer of Cardinal pins a version with `cardinal update --rev v<version>` and reads in `CHANGELOG.md` what
changed since its last pin.

A project that turns off `conventional-commits` gets no type read: a commit is breaking only when it changes a file of
`release_breaking_files`, the proposal is a major or a patch (a minor only from a `feat` written before the turn off), `release` warns so on stderr (and says when that list is
empty: every such commit is a patch), and the section lists the other subjects whole under `Changed`. Each commit is
read with the setting of its own revision, so a release spanning the turn off still reads the commits written before
it as Conventional Commits.

## Evaluating Cardinal

Cardinal is evaluated from the Claude Code session transcripts, over several of its revisions. Claude Code deletes
them after 30 days by default, for every project at once: keep them a year in your user settings
(`~/.claude/settings.json`, or `settings.json` in `CLAUDE_CONFIG_DIR`), which `cardinal` never writes:

```json
{ "cleanupPeriodDays": 365 }
```

`fixed_context_budget = <bytes>` in the `[project]` table of `.claude/cardinal.toml` bounds the fixed context every
session pays (`CLAUDE.md`, `.claude/CLAUDE.md`, the rules without `paths`, the files they import, `AGENTS.md` and
`.claude/AGENTS.md` only when imported or when neither `CLAUDE.md` file exists, and the skill descriptions): `check`,
`sync` and `update` warn when the work tree exceeds it, naming the largest part, and never fail because of it.

The skill `evaluate-harness` runs the readers below and reports the trend per axis; each can also run alone. With no
projects directory given, they read the `projects/` of the configuration directory and warn when it cannot be read.

| Tool | Prints |
|---|---|
| `session-metrics [--archive <file>]` | one row of numbers per session (cost, user interventions, project, issue, Cardinal revision, model, version), no message text; `--archive` also merges them into a file kept outside the repositories, so the history outlives the transcripts |
| `rule-breaches [--summary]` | the breaches of the rules a program can see (no `--no-verify`, a review before each commit, no `git add` of every file, no edit of a managed copy, the validation stop of an issue, the commit message format and no AI attribution, the conversation language, the skill a commit, a plan or a new spec was due to load) and the tool results over 25,000 bytes (the session's own, a reviewer's or another sub-agent's), apart from what a gate blocked; `--summary` counts them per rule and Cardinal revision, with the bytes of the waste rules |
| `delivered-quality [repo] [--summary]` | one row per done issue: its commits and deliverables, the fixes inside it, the fixes of the lines it wrote and the reverts of its files in the 30 days after its close, its reviews and their findings, the lines it changed in the specs and in the docs, the setup (main model, alone or advised; `mixed` when its sessions differ), failed checks, typed messages and tokens of the sessions that made it; `--summary` sums them per Cardinal revision and setup |
| `fixed-context` | per project and Cardinal revision, the bytes of the memory files (`AGENTS.md` apart, counted only when loaded) and the skill descriptions that every session pays, and the skills the revision ships |
| `cardinal-changes [repo] --sessions <file>` | one row per `Expected-effect: <metric> <up\|down>` line of a commit of the Cardinal source that changes `files/`, with the revisions of the sessions that hold the change |
| `cardinal-report --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>] [--changes <file>] [--min-sessions <n>] [--min-issues <n>] [--by revision\|setup] [--last <n>] [--anonymous]` | one markdown table per axis (efficiency, quality, rules with the waste bytes per session, skills, fixed context), and with `--changes` a verdict per expected effect; see below |

`cardinal-report` in detail:

- **Groups**: a row per group of consecutive revisions with at least `--min-sessions` (10) sessions, the model,
  version and project mix next to the numbers (`--anonymous` keeps only the count of projects); `--last <n>`
  prints only the last `n` groups, the verdicts still reading them all.
- **`--by setup`**: groups the sessions by setup (`<model> solo` or `<model> advised by <advisor>`) instead, with
  their revision mix and the quality of each setup, and leaves out the tables kept per revision (rules, fixed
  context, verdicts).
- **Verdicts** (`--changes`): `helped`, `hurt`, `no observed effect`, `no data`, or `confounded` when something else
  changed too, a group is too small, or, for a ratio of the quality table, a group closed fewer issues than
  `--min-issues` (5) (only those of known tokens count for the tokens) or, for the findings per review, also held
  fewer reviews than it.

## Parallel issues in worktrees

For an issue run in parallel, the user opens a worktree (with `git worktree add`, or the recipe below) and hands it
issues; the agent never creates, enters or leaves one (ADR 0011, ADR 0022,
[specs/parallel-issues.md](../specs/parallel-issues.md)). The agent claims each issue with `cardinal run issue-claim`,
and the `workspace-lock` hooks keep one session per worktree:

| Recipe | What it does |
|---|---|
| `just open-worktree <name> [--path <dir>] [--branch <branch>]` | creates the worktree `../<repo>-<name>` next to the main one (or `<dir>`, relative to the current directory), on a new branch `<name>` (or `<branch>`) cut from the branch of the main worktree, or on that branch as it is when it exists and no worktree has it; refuses a path inside the main worktree; copies `.claude/cardinal.local.toml` into it. It takes no issue: the worktree serves issues in turn and keeps its build. Launch a plain `claude` in it. |
| `just close-worktree <name\|path> [--force]` | from outside the worktree, removes it and deletes the branch it has checked out once that branch is merged (fast-forward, rebase or squash); `--force` also drops work not on the main branch, never uncommitted changes nor the main worktree. A name designates `../<repo>-<name>`, then a worktree left under `.claude/worktrees/<name>`, then the worktree of the branch `<name>`, then that branch alone (a `Stale:` branch of `next-issue`); a name that designates both a worktree on another branch and a branch is refused: pass the path |

The steps around them (merge, close) are in [workflow.md](workflow.md#parallel-issues).

A worktree the former `start-issue` left under `.claude/worktrees/` is still inside the main one: remove it with
`close-worktree <stem>` once its branch is merged.
