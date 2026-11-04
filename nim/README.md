# Cardinal in Nim — phase 1

The first slice of the Rust → Nim port: `sync` and `check` with the same layout, lock
format and exit codes.

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

## Parity with the Rust binary (2026-11-04)

| Test | Result |
| --- | --- |
| `check` on the synced project | exit 0, every one of the 41 lock hashes matches |
| drift a managed file | both binaries exit 1 and name the file |
| delete a managed file | both exit 1 and report it missing |
| fresh project `sync` | identical file tree; identical lock file lines |
| SHA-256 | empty, `abc` and the quick-brown-fox vector match |

## Not in phase 1

| Gap | Phase |
| --- | --- |
| settings entries (15 `entry` lines) are preserved but not generated | 2 |
| review-gate git hook install, migrations, features, `--rev` | 2 |
| gates (`run` tools): review, lint, read, lock, language, format | 2 |
| issue queue, release, worktrees, transcripts | 3+ |

The Rust `cardinal` stays the installed binary until phase 2 reaches parity; `cardinal-nim`
runs beside it for diffing.