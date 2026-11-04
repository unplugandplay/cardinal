# DevLog

Decisions, evidence and dead ends, newest first. One entry per working session.

## 2026-11-04 — Nim sync complete (entries + review hook)

| Piece | Result |
| --- | --- |
| Settings entries | `sync` merges `entries/.claude/settings.json` into `.claude/settings.json`, records the 15 `entry` lines |
| Review hook | installs the native `pre-commit` gate; a foreign hook moves to `pre-commit.user`; identical bytes to the Rust one |
| Fresh project | Rust `cardinal check` exits 0 on a project synced only by `cardinal-nim` |
| Parity | entry lines identical, settings JSON equal, hook identical, executable bit set |
| `check` | now fails on a missing managed entry or a missing gate |
| Remaining in Nim `sync` | migrations, feature switching, `--rev`/`update` |

Rust is now needed only for the evaluation readers, `update`, and the hook binary; phase 5 installs
the Nim binary as `cardinal` and removes it.

## 2026-11-04 — Nim phase 3: plain-JS adapter

| Change | Detail |
| --- | --- |
| Adapter | `index.ts` removed; `index.js` plain ESM, no build, no dependencies |
| Protocol | neutral event `{action, input, session, cwd, stop_active, last_message, success}`; the Nim CLI normalizes it, legacy Claude-shaped events still pass |
| Binary | `CARDINAL_BIN`, then `<project>/nim/bin/cardinal-nim`, then `~/.local/bin`, then `PATH` |
| `language-hint` | ported to Nim (French/English stop words) — parity 2/0 and identical messages |
| Verified live | read-hint deny + retry (Nim), review-gate `--no-verify` refusal (Nim), workspace-lock, model pinning |
| Adapter README | rewritten: JS, Nim CLI, neutral protocol, gaps |

Known gaps unchanged: `code-review` skill, settings entries generation, evaluation readers
(Rust), non-interactive shell denials.

## 2026-11-04 — Nim phase 2: gates

| Gate | Parity with Rust |
| --- | --- |
| `review-gate check` | 6/6 commands: identical exit code and messages |
| `review-gate native` | no marker 2, covered 0, unseen file 2, waiver 0 — both |
| `review-gate record` | Rust marker accepted by Nim, and the reverse |
| `lint-gate` | failing command 2, passing 0 — both; same diagnostics header |
| `read-hint` | identical deny JSON and 38,700-byte estimate; record dir cross-compatible |
| `workspace-lock` | Rust lock + Nim check and Nim lock + Rust check both refuse; release works |
| `format-code` | same formatter invocation; not separately exercised |

Bugs found and fixed while porting: a nil `JsonNode` on a missing `offset`/`limit` (segfault), and
the trailing empty line inflating the read estimate by 8 bytes.

Deferred: `language-hint` (word lists), settings entries generation, review-hook install,
migrations, features, `--rev`. Details: `nim/README.md`.

## 2026-11-04 — Nim phase 1: sync and check

| Item | Result |
| --- | --- |
| Binary | `nim/bin/cardinal-nim`, ~300 lines + a vendored SHA-256; no dependencies |
| `check` parity | exit 0 on the synced project; all 41 Rust-written hashes match |
| Drift / missing | both binaries exit 1 and name the file |
| Fresh project `sync` | file tree identical to the Rust one; lock file lines identical |
| SHA-256 | empty, `abc`, quick-brown-fox vectors pass |
| Gap | settings entries preserved but not generated (0 vs 15 on a fresh project); gates are phase 2 |

Details: `nim/README.md`.

## 2026-11-04 — git 2.31 requirement removed

| Change | Detail |
| --- | --- |
| Portable paths | five source files and one test use `rev-parse --git-path` / `--git-common-dir`, resolving relative results themselves |
| Waiver recipe | `cardinal-review.just` uses `--git-path` and truncates with `: >` instead of `touch`, so repeated waivers work |
| Tests | `workspace_lock` 14, `lint_gate` 8, `close_issue` 12, `deliverable_size` 1 — all pass |
| Old git | verified with git 2.23: lock start/check refuse, waiver path resolves and truncates |
| Remaining caveat | Rust test fixtures still use `git init -b` (2.28+); the product does not |
| Next | `docs/discovery/2026-11-04-nim-rewrite.md`: prompts + one Nim CLI + plain-JS shim, Claude-shaped contract removed |

## 2026-11-04 — Claude-free bridges

| Bridge | Implementation | Verified |
| --- | --- | --- |
| `workspace-lock` | one sleeper `sh` process per OpenCode session supplies `CLAUDE_PID`; taken on session create, released on delete | two live sessions in one worktree: second refused with exit 2 |
| `language-hint` | the adapter passes `last_assistant_message` from OpenCode's context; no transcript read | French reply → exit 2 with the hint; short text and retry → 0 |
| Evaluation | sessions mirrored to `~/.local/share/cardinal/projects/<slug>/<session>.jsonl` with ids, models and real tokens | `session-metrics` shows turns=1, input=232, output=4, cache=7168 |
| git | Homebrew git 2.56 installed; Cardinal needs `--path-format` (git ≥ 2.31), the adapter prepends `/opt/homebrew/bin` | `workspace-lock` and the waiver recipe no longer dead on git 2.23 |

Remaining: the `code-review` skill is absent, so no review marker is recorded; the native
pre-commit gate still refuses un-reviewed commits. `opencode run` ends on a shell denial instead of
handing the reason back; the TUI continues.

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
