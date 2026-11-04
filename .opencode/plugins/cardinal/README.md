# Cardinal ↔ OpenCode 2 adapter

Runs Cardinal's gates around an OpenCode + DeepSeek session. Cardinal's gate commands are
harness-agnostic CLI tools that expect a Claude Code hook event on stdin; this plugin builds that
event from OpenCode's hooks and applies the verdict.

## Requirements

| | |
| --- | --- |
| `cardinal` | on the server's `PATH`, or set `CARDINAL_BIN=/abs/path/to/cardinal` |
| `cardinal sync` | run once in the project (the git `pre-commit` review gate is installed by it) |
| Config | `skills: [".claude/skills"]`, `instructions: [".claude/rules/*.md"]` in `opencode.jsonc` |

## Wiring

| OpenCode hook | Cardinal tool | Effect |
| --- | --- | --- |
| plugin setup · `.env` | — | pins the default model from `OPENCODE_MODEL=<provider>/<model>` |
| `permission.evaluate` · `shell` | `review-gate check` | denies `--no-verify`, `core.hooksPath`, `git add` of every file |
| `permission.evaluate` · `read` | `read-hint` | denies an oversized whole read once per file and session |
| `tool.execute.after` · `edit`/`write` | `format-code` | formats the edited file with the project's formatter |
| `tool.execute.after` · `skill` | `review-gate record` | records a finished `code-review` skill run |
| `session.experimental.idle` | `lint-gate` | injects the lint output once so the agent fixes it |

After editing `.env`, reload the adapter: touch `index.ts` (hot reload) or `opencode service restart`.

The lint nudge is reset by each new user prompt, mirroring Cardinal's `stop_hook_active` retry
limit.

## Known gaps

| Gap | Why |
| --- | --- |
| `workspace-lock` | needs `CLAUDE_PID`; OpenCode sessions share the server process, so liveness cannot tell them apart |
| `language-hint` | reads Claude Code transcripts (`~/.claude/projects/**/*.jsonl`), which OpenCode does not write |
| Evaluation tools | `session-metrics`, `rule-breaches`, `delivered-quality`, `cardinal-report` — same transcript dependency |
| `code-review` skill | the review gate records it when run; that skill is not part of Cardinal's 8 skills, so nothing records a review yet (the native pre-commit gate still refuses un-reviewed commits) |
| Non-interactive deny | `opencode run` ends the run on a shell denial (`permission.rejected`) instead of handing the reason back; the TUI continues |

Fixing `workspace-lock` and `language-hint` means a small Cardinal change (harness-neutral session
identity and a transcript source), not a larger plugin: the plugin already translates every event.
