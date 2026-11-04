# Demo — hello world, start to finish

`run.sh` copies `hello-world/` to a temporary directory, syncs the harness into it, then exercises
every gate and workflow helper. Each case writes a log under `logs/`; the script asserts 43
outcomes and exits non-zero on any failure.

```sh
sh demo/run.sh
```

## The project

`hello-world/` is two files: `hello.nim` (`echo "hello world"`) and `.claude/cardinal.toml`
(source `../../harness`, lint `nim check`, a formatter that leaves `formatted.log`).

## What is exercised

| Group | Logs | Checks |
| --- | --- | --- |
| build | `unit-sha256.log` | SHA-256 known answers |
| sync | `sync.log`, `check.log`, `dry-run.log`, `commit-adopt.log` | copy, seed, lock, review hook, waived first commit |
| gates | `review-*.log`, `read-*.log`, `language-*.log`, `lint-*.log`, `format.log`, `lock-*.log`, `native-*.log` | refusals and passes for review, read hint, language, lint, format, workspace lock and the native review gate |
| workflow | `commit-*.log`, `issue-*.log`, `next-issue.log`, `close-issue.log`, `review-target.log`, `deliverable-size.log`, `worktree-*.log`, `release-*.log`, `check-specs.log` | commit rules, issue template and queue, close and link rewrite, target/size, worktrees, release proposal/write/tag, specs |
| measurement | `metrics.log` | `session-metrics` reads a mirrored session |
| utility | `fixed-context.log`, `audit-sources.log` | byte counts |

Logs are sanitized: repository paths become `<repo>`, temp paths `<tmp>`, home `~`.

## Result

The last run: **43 passed, 0 failed** (see `logs/` for every command and its exit code).