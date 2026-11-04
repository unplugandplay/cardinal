# TLDR;

One agent harness for many projects. Prompts and their order are the product; six gates and a
small CLI keep them honest. No Claude Code and no Rust are needed.

## 30 seconds

```sh
# build and install the CLI
nim c -d:release --hints:off -o:nim/bin/cardinal-nim nim/cardinal.nim
install -m 755 nim/bin/cardinal-nim ~/.local/bin/cardinal

# start the agent
opencode
```

| You want | Do |
| --- | --- |
| Work an issue | hand `issues/<name>.md` to the agent, or say "the next issue" |
| Create an issue | "Create an issue: …" |
| Check the harness | `cardinal check` |
| Sync after editing `harness/` | `cardinal sync` |
| Metrics | `cardinal run session-metrics ~/.local/share/cardinal/projects` |
| See it work | `sh demo/run.sh` |

## The model

| Layer | Role |
| --- | --- |
| `harness/` | prompts (`files/`), seeds, managed settings entries |
| `nim/` | the CLI: sync/check, gates, workflow helpers, metrics |
| `.opencode/plugins/cardinal/` | plain-JS shim: OpenCode events → neutral JSON → CLI |
| `.env` | `OPENCODE_MODEL=<provider>/<model>` — any provider |

The order that matters: roadmap → issues → priority → plan validated → deliverables (test, code,
docs, review, commit).

## The gates

| Gate | Refuses |
| --- | --- |
| review | a commit without a review or your waiver (`just waive-review`) |
| lock | two live sessions in one worktree |
| read | a whole read of a large file once per session |
| format | an edited file left unformatted |
| lint | ending a turn while the lint fails (one nudge) |
| language | a reply not in the user's language (one nudge) |

## More

| Document | Content |
| --- | --- |
| [README](../README.md) | the full map |
| [demo](../demo/README.md) | hello world, 43 checks, logs |
| [nim/README](../nim/README.md) | CLI commands and parity |
| [DEVLOG](../DEVLOG.md) | decisions and evidence |
| [docs/discovery](../docs/discovery/2026-11-04-nim-rewrite.md) | why Nim, what was dropped |