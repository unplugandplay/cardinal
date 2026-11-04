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
harness/            files/ seed/ entries/ + LICENSE: prompts, seeds and managed settings data
nim/                one binary (no runtime)
  sync | check      copy managed files, SHA-256 manifest, settings entries, review hook
  update            sync the local source
  run review|lint|read|lock|language|format|session-metrics
.env                OPENCODE_MODEL=<provider>/<model>
.opencode/plugins/cardinal/index.js   plain JS shim, no build, no deps
```

Two structural simplifications:

| Now | After |
| --- | --- |
| Claude hook event (`tool_name`, `tool_input`, `session_id`, exit 2) | one neutral JSON: `{action, input, session, cwd}`; exits are the only contract |
| `.claude/settings.json` merging | ported to `entries.nim`; the lock keeps its 15 `entry` lines |
| git `--path-format` | absolute paths computed in Nim (`getCurrentDir()` + `absolutePath`) |
| 14.7k Rust + 21.7k test lines + TS plugin | target ~2–3k Nim + ~150 JS, tests in Nim |
| transcript readers tied to Claude JSONL | neutral mirror under `$XDG_DATA_HOME/cardinal/projects` |

## Port map

| Nim file | Role |
| --- | --- |
| `cardinal.nim` | CLI: `sync`, `check`, `update`, `run` |
| `config.nim` | the TOML keys the harness reads |
| `entries.nim` | settings entries, lock lines, review hook |
| `gates.nim` | review, lint, read, lock, language, format, dispatch |
| `metrics.nim` | `session-metrics` |
| `shell.nim` | process and git helpers |
| `sha256.nim` | vendored hash |

The Rust modules not ported (issue queue, release, worktree, specs, commits, legacy readers)
answer with a "not ported" message and exit 3.

## Phases

| Phase | Deliverable | Status |
| --- | --- | --- |
| 1 | Nim `sync`/`check` + manifest, same prompts | **done 2026-11-04** — tree and lock lines match; `nim/README.md` |
| 2 | Nim gates: review, lint, read, lock | **done 2026-11-04** — parity table in `nim/README.md` |
| 3 | Plain-JS adapter calling the Nim CLI | **done 2026-11-04** — `index.js`, neutral events, `language-hint` ported |
| 2b | Sync completeness | **done 2026-11-04** — entries merged, review hook installed, Rust `check` accepts the result |
| 4 | Metrics and update | **done 2026-11-04** — identical `session-metrics` rows; local `update` |
| 5 | Remove Rust and TypeScript from the tree | **done 2026-11-04** — prompts in `harness/`, Rust tree and binary deleted, CLI installed as `cardinal` |

Unported helpers stub with exit 3; the daily loop (prompts, gates, sync, metrics) is Nim.
Effort: 3–4 weeks including parity tests; phases 1–2 are the
bulk.

## Git

No `--path-format` anywhere: the runtime requirement is now git 2.23+ (tested on 2.23 and 2.56).
`git init -b` remains only in Rust test fixtures, not in the product.