# Discovery: Cardinal beyond Claude Code and Rust

Date: 2026-11-03 · Source studied: `cardinal/` @ main (cardinal v3.1.0)

Question: can Cardinal run without Claude Code (DeepSeek as the model instead), and can the
Rust tool become Nim? Three candidate stacks are compared on overhead:

| Stack | Harness (the agent) | Harness tooling | Model |
| --- | --- | --- | --- |
| A | OpenCode 2 | `cardinal` binary (Rust, kept) | DeepSeek via provider config |
| B | "DSH": a purpose-built minimal harness over the DeepSeek API | `cardinal` binary (Rust, kept) | DeepSeek API direct |
| C | Plain Nim CLI: a homegrown harness in Nim | `cardinal` ported to Nim | any (DeepSeek included) |

*Baseline to beat: Claude Code + Rust `cardinal` — what the user has today.*

---

## 1. What the Rust tool actually is

| Fact | Value |
| --- | --- |
| Source | 14,669 lines of Rust in `crates/cardinal/src` (1 binary, 30 modules under `src/tools/` + 10 support modules; 25 exposed via `cardinal run`, the rest are subcommands) |
| Tests | 21,712 lines, 43 integration test files |
| Dependencies | `clap`, `serde_json`, `toml`, `sha2`, `chrono`, `anyhow` — no HTTP, no database |
| What it does | file synchroniser (`init`/`sync`/`check`/`update`), gate runner (`run <tool>`), release tooling, worktrees, transcript readers |
| Calls an LLM? | **Never.** It is pure file, git and text processing |

The tool splits in two by harness coupling:

| Group | Commands / tools | Coupled to Claude Code? |
| --- | --- | --- |
| Harness-agnostic | `sync`, `check`*, `init`, `update`, `review-gate`, `lint-gate`, `format-code`, `read-hint`, `workspace-lock`, `commit-check`, `check-issue`, `next-issue`, `close-issue`, `check-specs`, `spec-coverage`, `check-commits`, `deliverable-size`, `release`, `worktree`, `review-target`, `issue-claim`, `upgrade` | No — they read stdin JSON, git, and files |
| Transcript-coupled | `session-metrics`, `rule-breaches`, `delivered-quality`, `cardinal-changes`, `audit-sources`, `fixed-context`, `language-hint` | Yes — they parse `~/.claude/projects/**/*.jsonl` (Claude Code transcript format) |
| Derived | `cardinal-report` | No — aggregates the outputs of the readers above, never the transcripts |

\* `check` is 99% agnostic: it also reads `~/.claude/settings.json` to warn about transcript retention.

Claude-specific surface, in full:

| Surface | Where | Replacement difficulty |
| --- | --- | --- |
| Hook wiring | `entries/.claude/settings.json` — PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd, matchers `Bash`, `Edit\|Write`, `Read`, `Skill` | Low — any harness with pre/post tool hooks; the commands (`cardinal run …`) stay identical |
| Hook stdin format | Claude Code hook JSON (`tool_name`, `tool_input`, `cwd`) read by `lint-gate`, `read-hint`, `review-gate` | Low — a ~50-line adapter per harness, or a stdin shim that translates |
| Permission rules | `allow` / `ask` / `deny` strings like `Bash(just waive-review)` merged into `.claude/settings.json` | Low — every harness has an equivalent (OpenCode: `permissions` / `policies`) |
| Skills | 8 skills as `SKILL.md` front-matter files in `.claude/skills/` | Low — OpenCode loads the same SKILL.md format natively |
| Memory files | `CLAUDE.md`, `CLAUDE.local.md` counted by `budget.rs`/`memory.rs` | Low — OpenCode reads `AGENTS.md` (already synced) |
| Transcripts | `<CLAUDE_CONFIG_DIR or ~/.claude>/projects/<proj>/<session>.jsonl` + `subagents/` | **High** — the evaluation suite reads this exact format; each harness stores sessions its own way |
| Env vars | `CLAUDE_PROJECT_DIR`, `CLAUDE_CONFIG_DIR` | Trivial |

Key structural fact: **"instead of Claude" and "instead of Rust" are separable.** The binary's
sync/check/gates never talk to Claude Code's runtime; only the entry wiring and the evaluation
suite do.

---

## 2. Stack A — OpenCode 2 + DeepSeek, Rust tool kept

| Aspect | Assessment |
| --- | --- |
| Model wiring | DeepSeek is a stock provider in the models.dev catalogue OpenCode uses (provider `deepseek`, `DEEPSEEK_API_KEY`). Current model IDs, checked against models.dev on 2026-11-03: `deepseek-v4-pro`, `deepseek-v4-flash`, `deepseek-flash` — e.g. `model = "deepseek/deepseek-v4-pro"`; no custom provider code |
| Skills | OpenCode 2 loads `SKILL.md` natively. Best: point `skills` at the existing directory (`"skills": [".claude/skills"]`) instead of copying — Cardinal's skill files contain ~24 `.claude/…` references (`cardinal.toml`, `rules/`, `update-docs-map.md`, `settings.json`) that only resolve if the files stay put |
| Instructions | `.claude/rules/*.md` go in `instructions` (`"instructions": [".claude/rules/*.md"]`) — same trick: keep the files where `sync` put them |
| Hooks | An OpenCode plugin (`.opencode/plugins/`, TypeScript, server part) reproduces every Claude hook: pre/post tool-execution hooks replace PreToolUse/PostToolUse; session lifecycle events replace SessionStart/SessionEnd; the plugin bodies just exec `cardinal run <tool>` |
| Permissions | `permissions` rules (`action: "shell"`, patterns like `git push *`) express the same allow/ask/deny set |
| Transcripts | OpenCode stores sessions in its own server store — replace `tools/transcripts.rs` (~470 lines, the shared reader) with one OpenCode-format reader; the 7 reader tools then need their field mapping adjusted, `cardinal-report` keeps working unchanged |
| Runtime overhead | a long-lived JS service (Node/Bun) plus the TUI, seconds of startup, an npm plugin dependency |
| Build/maintenance overhead | Zero build for you; upstream moves fast (the V1→V2 plugin migration exists, so plugin API churn is a real recurring cost) |
| Effort | A plugin (~200–400 lines TS) + config; days, not weeks |

Verdict: least total overhead; keeps every gate and nearly the whole evaluation story by replacing the shared transcript reader.

## 3. Stack B — "DSH", a purpose-built DeepSeek harness, Rust tool kept

*Assumption: DSH = a hand-rolled agent loop over `api.deepseek.com` (OpenAI-compatible, tool
calling), wired to the same `cardinal run` tools.*

| Aspect | Assessment |
| --- | --- |
| What you write | chat streaming client, tool-calling loop, tool schemas, permission prompting, transcript persistence, hook dispatch, context-window management, retries — realistically 2–4k lines before daily-driver quality |
| Gates | trivially wired: the gates are plain CLI commands reading stdin JSON — `cardinal run` is called exactly as Claude Code calls it today |
| Evaluation | you own the transcript format, so the 7 reader tools need your format anyway (same cost as A, paid twice if the format churns) |
| Model lock-in | direct API coupling: a provider change is your code change |
| Runtime overhead | smallest of the three: one thin process |
| Hidden overhead | the highest: you inherit the hardest problems OpenCode solved (loop safety, sub-agents, context compaction, undo) — and DeepSeek's API changes are your maintenance |

Verdict: lowest runtime overhead, highest engineering overhead; only worth it if the harness itself is the product.

## 4. Stack C — plain Nim CLI (harness + tool all Nim)

Two separable ports:

### 4a. `cardinal` → Nim

| Rust dependency | Nim replacement |
| --- | --- |
| `clap` (derive) | `cligen` (nearest in spirit) or `nim-argparse` |
| `serde_json` | `std/json` (or `jsony` for speed) |
| `toml` | `parsetoml` (nimble) |
| `sha2` | `std/sha2` (stdlib) |
| `chrono` | `std/times` (stdlib) |
| `anyhow` | exceptions + `Result` convention; no direct equivalent |
| `std::process` | `osproc` (stdlib) |
| `tempfile` (tests) | `std/os` temp dirs |

| Aspect | Assessment |
| --- | --- |
| Effort | ~14.7k lines in, ~15k lines out; **the 21.7k lines of tests are the real cost** — a strict port is 4–8 developer-weeks, dominated by the transcript/eval modules (JSONL parsing, time zones) |
| Result | one small native binary, MB-scale, instant startup, no toolchain for users |
| Risk | a fork: upstream Cardinal improvements must be re-ported forever (the sync model is built to prevent exactly this kind of drift) |

### 4b. Harness in Nim

| Aspect | Assessment |
| --- | --- |
| Effort | everything of Stack B, plus permission UX, a TUI or REPL, and skill loading — more, not less, than DSH |
| When it makes sense | only if a single-static-binary, no-runtime-dependency agent stack is itself the goal |

Verdict: lowest runtime footprint, by far the highest build and fork-maintenance overhead.

---

## 5. Overhead summary

| Overhead | A: OpenCode 2 | B: DSH | C: plain Nim |
| --- | --- | --- | --- |
| Engineering to first working day | days (plugin + config) | 2–4 kLOC harness | 4–8 weeks port + harness on top |
| Ongoing maintenance | plugin API churn (upstream) | model API churn (yours) | upstream re-porting forever |
| Runtime footprint | Node/Bun service, hundreds of MB | one thin process | one small binary, instant |
| Gates kept | all, as-is | all, as-is | all, after port |
| Evaluation kept | after replacing the ~470-line transcript reader | after a reader for your own format | after a port of 7 modules |
| Model swap later | config line | code change | config/code, your loop |
| Fork risk | none | none | permanent |

## 6. Recommendation

| If you want | Take |
| --- | --- |
| DeepSeek with the least work, everything else intact | **Stack A** — OpenCode 2 with the `deepseek` provider, a thin plugin wrapping `cardinal run`, skills and rules referenced in place |
| To own the harness as a product | Stack B — but budget the 2–4 kLOC honestly |
| A single static binary agent stack as the goal itself | Stack C — port the tool last; the harness in Nim is the expensive half |
| Nothing, really — keep what works | Keep Claude Code + Rust: the installed tooling already passes `cardinal check`, and none of the three stacks reduces today's overhead; each adds some |

Suggested first step for Stack A (a day of work): set `model = "deepseek/deepseek-v4-pro"` (or
`deepseek-v4-flash`) with `DEEPSEEK_API_KEY`, point `skills` at `.claude/skills` and `instructions`
at `.claude/rules/*.md`, and write one plugin that maps OpenCode's tool/session events onto the
existing `cardinal run review-gate|lint-gate|format-code|read-hint|workspace-lock` commands — no
Rust is touched in any scenario that keeps the binary.

## 7. Implementation status — Stack A adopted (2026-11-03)

Built in this repository; `cardinal check` still passes.

| Piece | State |
| --- | --- |
| `opencode.jsonc` | model `deepseek/deepseek-v4-pro`; `skills` → `.claude/skills`; `instructions` → `.claude/rules/*.md`; deny `just waive-review*` and edits of the waiver marker |
| Skills | all 8 load natively (verified live) |
| `.opencode/plugins/cardinal/` | adapter plugin (TypeScript, no dependencies) |
| `permission.evaluate` · shell | `review-gate check` — tested: `git commit --no-verify` refused |
| `permission.evaluate` · read | `read-hint` — tested: 32 KB whole read refused, reason returned to the model |
| `tool.execute.after` · edit/write | `format-code` — fires (verified in event log) |
| `tool.execute.after` · skill | `review-gate record` — wired; no `code-review` skill exists yet |
| `session.experimental.idle` | `lint-gate` — tested: failure injected once, then released |
| `workspace-lock` | not wired — needs `CLAUDE_PID`, absent in OpenCode |
| `language-hint`, evaluation | not wired — need an OpenCode transcript reader |

Model notes: `deepseek/deepseek-v4-pro` and `deepseek/deepseek-flash` answered; `deepseek/deepseek-v4-flash`
is listed by `opencode models` but resolves as unavailable for this account.

