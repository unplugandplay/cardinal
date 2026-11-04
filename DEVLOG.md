# DevLog

Decisions, evidence and dead ends, newest first. One entry per working session.

## 2026-11-04 — first push

| Item | Detail |
| --- | --- |
| License | MIT added: root `LICENSE` (project) and `cardinal/LICENSE` (vendored source) |
| Branch | `main`, matching the CI trigger and the remote default |
| First commit | project files + vendored `cardinal/` — 461 files, ~4 MB |
| Remote | `https://github.com/unplugandplay/cardinal` |
| Push | `main` → `origin/main` |
| History | remote `main` held upstream v3.1.0 (`ab0402a`); replaced with this project's first commit via `--force-with-lease` — upstream stays at `Electron-libre/cardinal` |

## 2026-11-04 — model choice moved to `.env` (DeepSeek 4.1)

| Decision | Detail |
| --- | --- |
| Single source of truth | `.env` → `OPENCODE_MODEL=<provider>/<model>` |
| Current value | `alibaba-token-plan/deepseek-v4.1-flash-1017` |
| Mechanism | the adapter reads `.env` at setup and pins the default via `ctx.model.transform` / `editor.default.set` |
| Provider-agnostic | verified by switching to `mistral/deepseek-v4-1-flash-1017` and back; both replied |
| Explicit override | `opencode run --model …` still wins for that run |
| Credentials | stay in the OpenCode auth store, never in `.env` |

Dead ends:

| Attempt | Why it failed |
| --- | --- |
| `"model": "{env:OPENCODE_MODEL}"` in `opencode.jsonc` | the binary does not load `.env`; the variable is empty and OpenCode falls back to `crouton-alpha-free` |
| `await ctx.model.list()` then set | the promise API returns a wrapper, not an array (`list().some is not a function`); use `editor.list()` inside the transform |

Caveat: editing `.env` needs an adapter reload — touch `.opencode/plugins/cardinal/index.ts`
(hot reload) or run `opencode service restart`.

Cardinal itself never calls a model: the harness does, and `cardinal` never issues an LLM request.

### Push hygiene (no push performed)

| Item | State |
| --- | --- |
| Remote | `origin` → `https://github.com/unplugandplay/cardinal`, configured, nothing pushed |
| Source clone | `cardinal/` vendored in the repo (its nested `.git` removed); `target/` stays ignored |
| `.gitignore` | `.DS_Store` only |
| `cardinal.toml` source | relative `cardinal` (resolved against the project root) |
| Absolute home paths | none left in the tree (`/Users/…`, `~/Boxes/…` removed) |
| `.env` | contains only `OPENCODE_MODEL`, no credentials |
| Credentials | OpenCode auth store, outside the repo |
| Secret scan | no keys, tokens, private keys, or auth files found |
| `cardinal check` after the move | exit 0; `sync --dry-run` exits 0 |

### Vendored the Cardinal source

| Decision | Detail |
| --- | --- |
| Why | the project must not depend on a clone of `Electron-libre/cardinal` |
| What | `cardinal/` committed as files: 421 files, ~2.5 MB, no `target/` |
| Nested `.git` | removed (the clone was pristine: clean status, no local commits, no stashes) so git stores files, not a submodule pointer |
| Updates | manual: diff/merge against upstream; `cardinal sync` keeps reading the relative `cardinal` source |
| License | `Cargo.toml` declares MIT; upstream ships no `LICENSE` file — add one before a public push |

## 2026-11-03 — Cardinal installed, OpenCode + DeepSeek adapter built

### Installed

| Step | Result |
| --- | --- |
| `cargo install --force --path crates/cardinal --locked` | `cardinal` v3.1.0 in `~/.cargo/bin` |
| `brew install just` | `just` 1.59.0 |
| source clone | `cardinal/` (vendored, rev `main`) |
| `cardinal init` + `cardinal sync` in this repo | skills, rules, recipes, settings, native review gate |
| `cleanupPeriodDays: 365` in `~/.claude/settings.json` | evaluation transcript retention |

`cardinal check` exits 0.

### Discovery

Wrote `docs/discovery/2026-11-03-beyond-claude-rust.md`: the Rust tool is harness-agnostic except
for transcript reading; compared OpenCode 2 + DeepSeek (A), a hand-rolled DeepSeek harness (B) and
a full Nim port (C). Chose A. Corrections made after review:

| Claim | Correction |
| --- | --- |
| DeepSeek `deepseek-chat` / `deepseek-reasoner` | current IDs are `deepseek-v4-pro`, `deepseek-v4-flash`, `deepseek-flash` |
| 45 test files | 43 |
| `release` reads transcripts | imports a commit-config helper only |
| `cardinal-report` reads transcripts | aggregates the other readers' output |
| skills move as-is | 24 `.claude/…` references; reference the directory in place instead |
| evaluation = one reader | replace the shared `transcripts.rs` (~470 lines) + field mapping in 7 tools |

### Adapter built

`.opencode/plugins/cardinal/` — TypeScript, no dependencies, execs `cardinal run` with a synthesized
Claude Code hook event on stdin and applies the verdict. Config in `opencode.jsonc`.

| Mapping | Cardinal tool |
| --- | --- |
| `permission.evaluate` · `shell` | `review-gate check` |
| `permission.evaluate` · `read` | `read-hint` |
| `tool.execute.after` · `edit`/`write` | `format-code` |
| `tool.execute.after` · `skill` | `review-gate record` |
| `session.experimental.idle` | `lint-gate` (inject once, reset on prompt) |

### Dead ends and fixes

| Problem | Fix |
| --- | --- |
| Gates silently no-op'd | the server's PATH lacks `~/.cargo/bin`; resolve `~/.cargo/bin/cardinal`, override with `CARDINAL_BIN` |
| `permission.evaluate` never fired at first | the background service had not loaded the new plugin; `opencode service restart` once after adding it |
| Throwing from `execute.before` aborted non-interactive runs | moved pre-gates to `permission.evaluate` deny + message |
| `deepseek/deepseek-v4-flash` | listed by `opencode models` but unavailable for this account; `deepseek-v4-pro` and `deepseek-flash` work |
| Test sessions showed stale context | `opencode run` continued earlier sessions; use `--session <new-id>` for isolation |
| Plugin edits | hot-reload works; the service was restarted only once after the plugin was first added |

### Verified

| Test | Result |
| --- | --- |
| skills + rules load | 8 skills, 3 rules present in session |
| 32 KB whole read | refused with reason; model stopped |
| `git commit --no-verify` | refused |
| write `.rs` file | `format-code` fired |
| forced lint failure | output injected once, then released |

### Open

| Item | Owner |
| --- | --- |
| `workspace-lock` needs a harness-neutral session identity (no `CLAUDE_PID`) | Cardinal change |
| `language-hint` + evaluation need an OpenCode transcript reader | Cardinal change |
| `code-review` skill is absent, so nothing records a review marker | decide: write a skill or waive |
| first commit still pending; needs `just waive-review` from the user | user |
