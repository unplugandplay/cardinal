# hello-world

A minimal project that adopts the Nim Cardinal harness: one file, one lint, one formatter hook.

## Files

| Path | Role |
| --- | --- |
| `hello.nim` | the whole program |
| `.claude/cardinal.toml` | source `../../harness`, lint `nim check`, format `touch formatted.log` |

## Run

```sh
nim r hello.nim
```

`demo/run.sh` copies this project to a temporary directory, syncs the harness, and exercises every
gate and workflow helper; see `../logs/` and `../README.md`.