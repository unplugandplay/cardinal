# Tooling

A project harness for agent-driven work: [Cardinal](https://github.com/Electron-libre/cardinal) supplies the
skills, rules, gates and recipes; an OpenCode 2 adapter runs them outside Claude Code, with DeepSeek as the model.

## Quick start

```sh
opencode            # in this directory: DeepSeek + the Cardinal skills are already configured
```

| Choice | Where |
| --- | --- |
| Model and provider | `.env` → `OPENCODE_MODEL=<provider>/<model>` (read by the adapter) |
| Credentials | OpenCode auth store (`opencode auth`), never in `.env` |

| What you type | What happens |
| --- | --- |
| `issues/<name>.md` or "the next issue" | `issue-workflow` drives it to done in validated deliverables |
| "Create an issue: …" | `create-issue` writes `issues/<name>.md` |
| "Prioritize the issues" | `prioritize-issues` reviews order and dependencies |
| `just next-issue` | shows the first ready issue |

## Layout

| Path | Role |
| --- | --- |
| `.claude/` | Cardinal's managed files: 8 skills, 3 rules, `cardinal.toml`, `cardinal.lock`, `settings.json` |
| `cardinal/` | vendored Cardinal source (MIT), so the project needs no separate clone |
| `.opencode/plugins/cardinal/` | the OpenCode adapter: maps OpenCode hooks onto `cardinal run` gates ([README](.opencode/plugins/cardinal/README.md)) |
| `opencode.jsonc` | skills and instructions sources, waiver denies (no model — see `.env`) |
| `.env` | `OPENCODE_MODEL=<provider>/<model>` — the single place the model and provider are chosen |
| `AGENTS.md`, `cardinal*.just`, `justfile`, `specs/` | Cardinal's managed harness, synced |
| `ROADMAP.md`, `CHANGELOG.md` | seeded by Cardinal, owned by the project |
| `docs/discovery/` | design notes and comparisons |

## Cardinal commands

| Command | Does |
| --- | --- |
| `cardinal check` | fails when a managed copy drifted |
| `cardinal sync` | copies, seeds, merges settings, installs the review gate |
| `cardinal update` | moves to a new revision of the source |
| `just waive-review` | skips the review gate for one commit — the user's, never the agent's |

## Gates in force

| Gate | Hook | Refuses |
| --- | --- | --- |
| review gate | native git `pre-commit` + `permission.evaluate` | a commit without a code review; `--no-verify`; `git add` of every file |
| workspace lock | `permission.evaluate` + session events | a second live session in the same worktree |
| read hint | `permission.evaluate` | a whole read of a large file once per file and session |
| format | `tool.execute.after` | leaves the edited file unformatted |
| lint gate | `session.idle` | ending a turn while the project's lint fails (one nudge per turn) |
| language hint | `session.idle` | a reply not in `user_language` (one nudge per turn) |

## No Claude Code needed

| Piece | How it works without Claude |
| --- | --- |
| Hook protocol | the adapter builds the Claude-shaped event itself; `cardinal` never invokes `claude` |
| `workspace-lock` | the adapter owns one sleeper process per session and passes its PID, so liveness works |
| `language-hint` | the adapter passes `last_assistant_message` from OpenCode's session context |
| Evaluation | the adapter mirrors each session to `~/.local/share/cardinal/projects/<slug>/<session>.jsonl` |

```sh
cardinal run session-metrics ~/.local/share/cardinal/projects
```

Remaining gap: the `code-review` skill is not part of Cardinal's 8 skills, so no review marker is
recorded; the native pre-commit gate still refuses un-reviewed commits.

## Requirements

| Tool | Install |
| --- | --- |
| `cardinal` | `cargo install --force --path cardinal/crates/cardinal --locked` |
| `just` | `brew install just` |
| git | 2.31+ (Cardinal uses `--path-format`); the adapter prefers `/opt/homebrew/bin/git` |
| OpenCode | 2.2.4+ with a signed-in provider of your choice |

Remote: `https://github.com/unplugandplay/cardinal` — `main` is the default branch.

## License

MIT — see [LICENSE](LICENSE). The vendored source under `cardinal/` is MIT, Copyright
(c) Electron-libre contributors; see [cardinal/LICENSE](cardinal/LICENSE).

Note: Cardinal itself never calls a model — the harness does. `cardinal` only synchronises files and
runs gates; the model is chosen in `.env` and pinned by the adapter.
