# Cardinal ↔ OpenCode 2 adapter

Plain JavaScript (`index.js`), no build, no dependencies. It sends a neutral event to the Nim CLI:

```json
{ "action": "read", "input": { "file_path": "…" }, "session": "ses_…", "cwd": "…" }
```

and applies the answer: `0` pass, `2` refusal (the message is shown to the model), `3` tool error.

## Requirements

| | |
| --- | --- |
| Nim CLI | `nim/bin/cardinal-nim` (built with `nim c -d:release -o:nim/bin/cardinal-nim nim/cardinal.nim`), or `~/.local/bin/cardinal`, or set `CARDINAL_BIN` |
| git | 2.23+ (any recent git); the adapter prefers `/opt/homebrew/bin/git` when present |
| Config | `skills: [".claude/skills"]`, `instructions: [".claude/rules/*.md"]` in `opencode.jsonc` |
| Sync | `cardinal sync` once, for the managed files |

Binary lookup order: `CARDINAL_BIN`, `<project>/nim/bin/cardinal-nim`, `~/.local/bin/cardinal-nim`,
`~/.local/bin/cardinal`, then `cardinal-nim` and `cardinal` on the server's `PATH`.

## Wiring

| OpenCode hook | Event `action` | Nim tool | Effect |
| --- | --- | --- | --- |
| plugin setup · `.env` | — | — | pins `OPENCODE_MODEL=<provider>/<model>` |
| `permission.evaluate` · `shell`/`edit` | `lock` | `workspace-lock check` | denies a second live session in the worktree |
| `permission.evaluate` · `shell` | `shell` | `review-gate check` | denies `--no-verify`, `core.hooksPath`, `git add` of every file |
| `permission.evaluate` · `read` | `read` | `read-hint` | denies an oversized whole read once per file and session |
| `tool.execute.after` · `edit`/`write` | `format` | `format-code` | formats the edited file |
| `tool.execute.after` · `skill` | `skill` | `review-gate record` | records a finished `code-review` skill run |
| session created / deleted | `start` / `stop` | `workspace-lock start`/`release` | takes and frees the worktree lock |
| `session.experimental.idle` | `idle` | `lint-gate`, `language-hint` | injects their output once per turn |
| `session.experimental.idle` | — | transcript mirror | writes `~/.local/share/cardinal/projects/<slug>/<session>.jsonl` |

The per-session liveness PID is a small `sh` loop owned by the plugin; it exits within a minute of
the server. The mirror carries ids, models and real token usage, so
`cardinal-nim run session-metrics …` reads it directly.

After editing `.env`, reload the adapter: touch `index.js` (hot reload) or `opencode service restart`.

## Known gaps

| Gap | Why |
| --- | --- |
| `code-review` skill | the review gate records it when run; that skill is not part of Cardinal's 8 skills, so nothing records a review yet (the native pre-commit gate still refuses un-reviewed commits) |
| Non-interactive deny | `opencode run` ends the run on a shell denial (`permission.rejected`) instead of handing the reason back; the TUI continues |
| Settings entries | `cardinal-nim sync` preserves them but does not generate them yet |
| Legacy evaluation readers | `rule-breaches`, `delivered-quality`, `cardinal-report`, `cardinal-changes`, `upgrade` are retired; `session-metrics` covers the measurement need |