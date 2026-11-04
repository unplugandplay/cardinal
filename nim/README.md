# Cardinal in Nim — phases 1–2

`sync`/`check` and the gates, with the same layout, lock format, stdin contract and exit codes
(0 pass, 2 refusal, 3 tool/usage error).

## Build and test

```sh
nim c -d:release --hints:off -o:nim/bin/cardinal-nim nim/cardinal.nim
nim c -r --hints:off -o:nim/bin/sha256_test nim/sha256_test.nim
```

No dependencies: SHA-256 is vendored in `sha256.nim` and tested against the standard vectors.

## Commands

| Command | Does |
| --- | --- |
| `cardinal-nim check` | fails (exit 1) when a managed file is missing or drifted |
| `cardinal-nim sync` | copies `source/files/` (managed), seeds `source/seed/` once, rewrites the lock |
| `cardinal-nim sync --dry-run` | prints the plan, writes nothing |
| `cardinal-nim sync --force` | overwrites local edits of managed files |
| `cardinal-nim run <tool> [args]` | a gate, reading the event JSON on stdin |

## Gates

| Tool | Modes | Ported |
| --- | --- | --- |
| `review-gate` | `check`, `record`, `native` | yes — snapshot, marker, waiver, hook-skip refusal |
| `lint-gate` | — | yes — declared command or clippy, stamp, diagnostics |
| `read-hint` | — | yes — size estimate, once per session/file |
| `workspace-lock` | `check`, `start`, `release` | yes — lock file, liveness, dirty-tree check |
| `format-code` | — | yes — declared formatter or `cargo fmt` |
| `language-hint` | — | **no** — needs the language word lists |

## Parity with the Rust binary (2026-11-04)

| Test | Result |
| --- | --- |
| `check` on the synced project | exit 0; all 41 lock hashes match |
| Drift / missing managed file | both exit 1 and name the file |
| Fresh project `sync` | identical file tree, lock lines, settings and hook; Rust `check` exits 0 |
| Settings / hook drift | `cardinal-nim check` names the missing entry or gate |
| `review-gate check` | 6/6 commands: identical exit code and messages |
| `review-gate native` | no marker 2, covered 0, unseen file 2, waiver 0 — both |
| `lint-gate` | failing command 2, passing 0 — both |
| `read-hint` | identical deny JSON and byte estimate |
| `workspace-lock` | locks are cross-compatible: Rust lock + Nim check, Nim lock + Rust check both refuse |
| SHA-256 | empty, `abc`, quick-brown-fox vectors pass |

## Not yet in Nim

| Gap | Phase |
| --- | --- |
| migrations, features, `--rev`/`update` | 2b |
| issue queue, release, worktrees, transcripts | 3+ |

`sync` now merges the settings entries and installs the native review hook; the Rust binary
accepts a project synced by `cardinal-nim` (`cardinal check` exits 0), with identical entry lines,
settings and hook. The Rust `cardinal` remains installed only for the evaluation readers and
`update`.