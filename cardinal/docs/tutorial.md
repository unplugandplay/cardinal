# Tutorial

A first walk through Cardinal for a project that adopts it, from an empty project to a first issue delivered by the
agent. Each step says what to type and what to expect; the reference of each command is in [usage.md](usage.md), the
process in [workflow.md](workflow.md).

## 1. Install the tools

Cardinal needs two programs on your `PATH`: [`just`](https://github.com/casey/just#installation), which runs the
recipes of the project (`just check`, the agent's entry point, and the recipes of Cardinal), and `cardinal`. The
recipes of Cardinal mostly call `cardinal`, which runs them without `just` too: `just next-issue` runs
`cardinal run next-issue`.

Download the binary of the latest release for your platform (`x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` or `aarch64-apple-darwin`) into a directory of your `PATH`:

```sh
target=x86_64-unknown-linux-musl
base=https://github.com/Electron-libre/cardinal/releases/latest/download
curl -fsSL -o ~/.local/bin/cardinal "$base/cardinal-$target"
curl -fsSL "$base/cardinal-$target.sha256"   # compare with: sha256sum ~/.local/bin/cardinal
chmod +x ~/.local/bin/cardinal
cardinal --version
```

On another platform, build it with Rust:
`cargo install --git https://github.com/Electron-libre/cardinal.git --locked`.

The installed `cardinal` serves every project of the machine; `cardinal run` fetches the binary of the revision each
project pins by itself. When a revision needs a newer `cardinal`, it refuses and prints the command that installs it,
`cardinal upgrade <source> <commit>`: run it, then retry.

Claude Code deletes its session transcripts after 30 days; the evaluation of Cardinal reads them, so keep them a year
in `~/.claude/settings.json`:

```json
{ "cleanupPeriodDays": 365 }
```

## 2. Adopt Cardinal in a project

In the root of a git repository:

```sh
cardinal init https://github.com/Electron-libre/cardinal.git   # writes .claude/cardinal.toml
cardinal update   # pins the latest commit in `rev`, then syncs
```

`cardinal update --rev v<version>` pins a release instead of the head of the default branch.

`sync` prints one line per change:

| Line | Means |
|---|---|
| `added: <path>` | a managed file, copied from `files/` of the source: never edit it in the project |
| `seeded: <path>` | a starting point copied once, then yours (`justfile`, `ROADMAP.md`, `CHANGELOG.md`, `.claude/settings.json`…) |
| `added: .claude/settings.json hook …` / `allow …` / `deny …` | a hook or a permission rule merged into your settings |
| `added: native review gate` | the git `pre-commit` hook of the review gate, installed in this clone |

`cardinal check` then prints nothing: the copies match the source.

## 3. Fit it to the project

Edit the `[project]` table of `.claude/cardinal.toml` (seeded, yours). The keys a first adoption usually sets:

| Key | Set it to |
|---|---|
| `user_language` | the language the agent talks to you in (`"French"`); docs, code and issues keep the project's |
| `lint_command`, `lint_files` | the project's lint, run before the agent ends a turn (default: `cargo clippy` over `.rs` files; `""` turns it off) |
| `format_command`, `format_files` | the formatter run after each edit (default: `cargo fmt` for a `.rs` file) |
| `commit_scopes` | the scopes allowed in commit messages, besides `docs` and `cardinal` |
| `confidential` | what must never be named in an issue or a committed file |

A personal override (your own `user_language`) goes in `.claude/cardinal.local.toml`, which git ignores.

Then fill the `check` recipe of the seeded `justfile` with the project's format, lint and tests, above the last line:
the agent runs `just check` before each review. Add the commands of your stack to the `allow` rules of
`.claude/settings.json` (`npm test`…), or run them through `just`, which is allowed already.

A feature you don't want (the GitHub CI, the specs, the issue files…) goes in `disabled` of the `[cardinal]` table;
the next `sync` removes its files ([usage.md](usage.md#features-a-project-turns-off)):

```toml
[cardinal]
source = "..."
disabled = ["github-ci"]
```

## 4. Commit the adoption

```sh
git status --short   # the files sync added or seeded
git add .claude AGENTS.md CHANGELOG.md ROADMAP.md justfile cardinal*.just
git add .github specs  # unless step 3 turned off `github-ci` and `specs`
git commit -m "chore: adopt cardinal"
```

The review gate refuses it: `Refused: no code review has run in this project.` Every commit needs a code review of
its diff. Either launch `claude` and ask it to commit (it runs `/code-review low` first), or, for this commit of
copied files, waive the review yourself, then commit again:

```sh
just waive-review
```

The agent can never run `just waive-review`: the waiver is yours alone ([gates.md](gates.md#the-review-gate)).

## 5. Plan the work

Describe your milestones in `ROADMAP.md` (the seeded file shows their shape: a goal, a why, a state). Then, in
`claude`:

> Create an issue: the login page must reject an empty password.

`skill:create-issue` writes `issues/<name>.md` (Goal, Findings, Approach, Resources), gives it a `priority` and its
`depends_on`, attaches it to a milestone, checks it with `just check-issue`, and commits it. Several issues later,
ask it to prioritize them (`skill:prioritize-issues`) to review their order.

```sh
just next-issue    # the first ready issue, then the others
```

## 6. Have the agent deliver an issue

> Work the next issue.

or name it: `issues/<name>.md`. `skill:issue-workflow` then:

1. **Prepares**: writes its understanding, a spec draft when a behaviour changes, and a plan of deliverables in the
   issue.
2. **Stops once**: shows the plan and its questions as a numbered table. Answer in one line: `ok` (the recommended
   answers), or `1 a, 2 c`, and the mode. Your answers go to `## Decisions` of the issue.
3. **Delivers** each deliverable: a failing test first, the code, the docs, `just check`, a code review, a commit. In
   autonomous mode (the default) it chains them and writes each summary in the issue; it stops only on a question, a
   finding or a check it cannot settle. Otherwise it stops after each deliverable for your validation.
4. **Closes**: after a `high` review of the whole issue, it asks whether to close it; on your agreement,
   `just close-issue` moves it to `issues/done/` and prints the next one.

A session cut off midway resumes where it stopped: hand the same issue again, the plan in its `# Tracking` section
says how far it went.

## 7. What the gates stop

Hooks guard the agent's mistakes, not your own actions ([gates.md](gates.md)):

| You see | Because | Do |
|---|---|---|
| a commit refused for a missing review | its diff holds a file the last review never saw | let the agent review, or `just waive-review` yourself |
| `git add -A`, `git add .` or `--no-verify` refused | each file is staged by name, no hook skipped | nothing: the agent stages its files by name |
| the agent goes on after it meant to stop | the lint warns, or the reply was not in `user_language` | nothing: it fixes the warning or replies again |
| edits refused in a worktree | another live Claude Code session holds it | use one session per worktree |
| a commit message refused | not Conventional Commits, or an AI attribution | the agent rewrites it (`skill:commit-message`) |

## 8. Keep the project in step

| Situation | Run |
|---|---|
| the source has a new revision | `cardinal update [--rev <tag\|branch\|commit>]`: syncs, then pins the commit in `rev` of `.claude/cardinal.toml` |
| you want to see what a sync would do | `cardinal sync --dry-run` |
| `check` reports a managed file modified | `cardinal sync --force` drops the local edit; a fix worth keeping goes to the Cardinal repository (an issue or a pull request), never into the copy |
| a fresh clone of the project | `cardinal sync`: it installs the review gate of this clone |

The [`CHANGELOG.md`](https://github.com/Electron-libre/cardinal/blob/main/CHANGELOG.md) of Cardinal says what
changed between two releases.

## 9. Two issues at once

```sh
just open-worktree login     # ../<repo>-login, on a new branch login
cd ../<repo>-login && claude # hand it an issue there
```

Each session claims its issue, so `just next-issue` skips it in the other worktrees. Once its branch is merged,
`just close-worktree login` removes the worktree and the branch ([workflow.md](workflow.md#parallel-issues)).

## 10. Release

When `close-issue` marks a milestone `done`:

```sh
just release              # proposes the next version from the commits
just release 1.2.0        # writes its CHANGELOG.md section: reread, edit, commit
just release 1.2.0 --tag  # tags v1.2.0 locally; the push is yours
```

## Troubleshooting

| Message | Fix |
|---|---|
| `needs a cardinal that reads the format <n>` | run the `cardinal upgrade …` it prints |
| a `cargo install …` to run, with no Rust | `rev` is not a commit: `cardinal update` pins one, then retry; otherwise that `cardinal` predates `cardinal upgrade`: download the latest release again (step 1) |
| `no .claude/cardinal.toml` | `cardinal init <source>` first |
| `no .claude/cardinal.lock` | `cardinal update` (or `cardinal sync`) first |
| `no issues directory` from `just next-issue` | no issue yet: create one (step 5) |
| a hook refuses every command after an update | the installed `cardinal` is too old to even skip what it does not know: download the latest release again (step 1) |
