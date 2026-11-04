---
id: AUD
status: accepted
adrs: [0006]
---

# Sources of the instruction audit

## Intent

`cardinal run audit-sources` lists, per project of the transcripts directory, what the user told the agent over time
(sessions, typed messages, memory files, instruction files), so that `audit-instructions` proposes only the skills,
hooks and rules that do not exist yet.

## Requirements

- **AUD-1** (deterministic) Without `--messages`, `audit-sources` must print, for each working directory of a project's sessions, one line `  instructions: <file>` per instruction file of that directory and of each of its ancestor directories up to the root of its repository (the nearest one holding `.git`; none without one), `<file>` its absolute path, sorted, each file once however many paths name it (a symbolic link printed as the file it points to): in each of these directories, the memory files a session loads at launch as REP-2 defines them (imports included, resolved in the tree of the repository), plus `CLAUDE.local.md` and the files it imports as REP-2 defines the imports, `AGENTS.md`, `.claude/AGENTS.md` and the `.claude/rules/**/*.md` files whose front matter has a `paths` key, when they exist, the user-level files of AUD-2 excepted when the configuration directory is the `.claude/` of one of these directories.
- **AUD-2** (deterministic) Without `--messages`, `audit-sources` must print, once and before the projects, one line `instructions: <file>` per user-level instruction file, `<file>` its absolute path, sorted, each file once (a symbolic link printed as the file it points to): in the configuration directory (`CLAUDE_CONFIG_DIR` when set and not empty, else the `.claude/` directory of the home directory), `CLAUDE.md` and the `rules/**/*.md` files, scoped by `paths` or not, when they exist, plus the files that `CLAUDE.md` and the rules without `paths` import as REP-2 defines the imports, resolved in that directory (an import that is absolute, starts with `~` or leaves it is not listed); no line when there is none.

## Scenarios

- (AUD-1) Given a directory with `CLAUDE.md`, `AGENTS.md`, a rule `.claude/rules/a.md` and a rule `.claude/rules/b.md` scoped by `paths`, when the summary is printed, then it lists the four files.
- (AUD-1) Given a `CLAUDE.md` that imports `@docs/x.md` and `@~/notes.md`, when the summary is printed, then it lists `docs/x.md` and not `~/notes.md`.
- (AUD-1) Given a `CLAUDE.local.md` that imports `@notes.md`, when the summary is printed, then it lists both: a session loads them at launch, though git does not track them.
- (AUD-1) Given a `CLAUDE.md` that is a symbolic link to `AGENTS.md`, when the summary is printed, then it lists `AGENTS.md` once.
- (AUD-1) Given a rule that is a symbolic link to `/shared/rule.md`, when the summary is printed, then it lists `/shared/rule.md`.
- (AUD-2) Given `CLAUDE_CONFIG_DIR` naming a directory with `CLAUDE.md` importing `@notes.md`, a rule `rules/a.md` and a rule `rules/b.md` scoped by `paths`, when the summary is printed, then its first lines list the four files, before the first project.
- (AUD-2) Given `CLAUDE_CONFIG_DIR` unset, when the summary is printed, then it lists the user-level files of `~/.claude`.
- (AUD-1) Given sessions started in `/x/r/a/b`, `/x/r` a repository, with a `CLAUDE.md` in `/x`, `/x/r` and `/x/r/a`, when the summary is printed, then it lists `/x/r/CLAUDE.md` and `/x/r/a/CLAUDE.md`, not `/x/CLAUDE.md`: a session loads the memory files above the repository too, but the audit reads nothing outside it.

## Non-goals

- The user-level files among a project's (AUD-1): AUD-2 lists them once.
- The managed policy files of the system (`/etc/claude-code/`): the user did not write them.
- The content of the files: the skill reads them.
