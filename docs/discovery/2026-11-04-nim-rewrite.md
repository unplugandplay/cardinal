# Cardinal as prompts, one Nim CLI, one plain-JS adapter

Date: 2026-11-04 · Decision: replace the Rust binary and the TypeScript plugin, drop the Claude-shaped
contract, keep the philosophy.

## What is actually essential

The claim under test: *the prompts and their ordering are the important part; the rest is executable
machinery helping them.* Mostly true, with one correction.

| Layer | Role | Can it be removed? |
| --- | --- | --- |
| Prompts (skills, rules) + their order | define what the agent does, in what sequence, with which stop for validation | no — this is the behaviour |
| Harness (OpenCode) | agent loop, tool schemas, model calls | no — prompts are inert without it |
| Repository state (issues, specs, code, roadmap) | the context the prompts act on | no |
| Gates (review, lint, lock, read) | prevent irreversible mistakes a prompt cannot guarantee | trim to these four |
| Sync / drift check | keeps the copies honest | keep, much smaller |
| Evaluation / transcripts | measures, does not steer | optional, cut for v1 |
| Claude settings, hook schema, `CLAUDE_*` names | protocol of another harness | delete |

Correction: prompts shape *quality*; gates shape *trust*. A prompt can ask for a review before a
commit; only an executable can refuse the commit when the review did not happen. The philosophy
survives with four gates, not with prose alone.

## Target shape

```text
prompts/            skills/*.md, rules/*.md, order: roadmap -> issues -> plan -> deliverables
bin/cardinal        one Nim binary (no runtime)
  sync | check      copy managed files, SHA-256 manifest
  gate review       review marker vs staged diff hash
  gate lint         run the project's lint
  gate read         size check, per-session memory
  gate lock         worktree lock with heartbeat
  mirror            write a neutral JSONL transcript
.env                OPENCODE_MODEL=<provider>/<model>
.opencode/plugins/cardinal/index.js   plain JS shim, no build, no deps
```

Two structural simplifications:

| Now | After |
| --- | --- |
| Claude hook event (`tool_name`, `tool_input`, `session_id`, exit 2) | one neutral JSON: `{action, input, session, cwd}`; exits are the only contract |
| `.claude/settings.json` merging, `CLAUDE_PID`, `CLAUDE_PROJECT_DIR`, `CLAUDE_CONFIG_DIR` | gone; the adapter passes `session` and `cwd`, the CLI owns everything else |
| git `--path-format` | absolute paths computed in Nim (`getCurrentDir()` + `absolutePath`) |
| 14.7k Rust + 21.7k test lines + TS plugin | target ~2–3k Nim + ~150 JS, tests in Nim |
| transcript readers tied to Claude JSONL | neutral mirror under `$XDG_DATA_HOME/cardinal/projects` |

## Port map

| Rust module (lines) | Nim |
| --- | --- |
| `lib.rs` sync/check/hash (1.3k) | `sync.nim`, `manifest.nim` |
| `gate.rs`, `entries.rs`, `settings.rs` (0.9k) | drop settings merging; `gate.nim` |
| `tools/review_gate.rs` (0.7k) | `gates/review.nim` |
| `tools/lint_gate.rs` (0.3k), `format_code.rs` (0.2k) | `gates/lint.nim`, `gates/format.nim` |
| `tools/read_hint.rs` (0.2k), `language_hint.rs` (0.1k) | `gates/read.nim`, `gates/language.nim` |
| `tools/workspace_lock.rs` (0.4k) | `gates/lock.nim` (heartbeat file, not PID liveness) |
| `tools/next_issue.rs`, `close_issue.rs`, `issue_claim.rs` (1.5k) | `issues.nim` (or `just` recipes) |
| transcript readers (4.5k) | `mirror.nim` only for v1; metrics later |
| `release.rs`, `worktree.rs`, `check_specs.rs` (1.7k) | keep only if the philosophy needs them; recipes otherwise |

## Phases

| Phase | Deliverable | Status |
| --- | --- | --- |
| 1 | Nim `sync`/`check` + manifest, same prompts | **done 2026-11-04** — tree and lock lines match; `nim/README.md` |
| 2 | Nim gates: review, lint, read, lock | **done 2026-11-04** — parity table in `nim/README.md` |
| 3 | Plain-JS adapter calling the Nim CLI | **done 2026-11-04** — `index.js`, neutral events, `language-hint` ported |
| 2b | Sync completeness | **done 2026-11-04** — entries merged, review hook installed, Rust `check` accepts the result |
| 4 | Port issue queue / release or replace with recipes | **done 2026-11-04** for metrics + update; issue queue/release remain recipes |
| 5 | Remove Rust and TypeScript from the tree | next |

Keep the Rust binary installed until phase 5; run the Nim binary beside it and diff verdicts.
Effort: 3–4 weeks including parity tests; phases 1–2 are the
bulk.

## Git

No `--path-format` anywhere: the runtime requirement is now git 2.23+ (tested on 2.23 and 2.56).
`git init -b` remains only in Rust test fixtures, not in the product.