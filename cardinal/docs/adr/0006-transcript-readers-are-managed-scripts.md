# 0006. The transcript readers are managed scripts sharing one module

Status: Superseded by 0010
Date: 2026-10-26

## Context
The evaluation of the harness reads the Claude Code transcripts of every project; `audit_sources.rs` (a managed
`rust-script` of `audit-instructions`) already does. A `harness` subcommand would share code with the crate, but
the readers are run by skills, in any consumer, where the binary may be older than the managed files, and
`audit_sources.rs` would still parse the transcripts on its own. `rust-script` resolves `#[path = "…"] mod` from
the script's own directory.

## Decision
The transcript readers are managed scripts under `files/.claude/transcripts/`, next to one module,
`transcripts.rs`, that holds the line reading, the typed-message filter and the walk of projects and sessions.
`audit_sources.rs` includes it by a relative `#[path]`. The scripts print rows of numbers; the skills that read
them stay where they are.

## Consequences
* One place to adapt when the transcript format moves.
* A script must stay at a fixed path relative to the module: moving one moves the `#[path]`.
* The module cannot be unit-tested by `cargo test`; the behaviour tests of the scripts cover it (`just test-harness`).
* `rust-script` rebuilds a script only when its own text changes, not when an included module does (a `path`
  dependency is no better). Each including script therefore carries `// transcripts.rs sha256: <hex>`, and the test
  fails with the line to write when it is stale: a change of the module edits every script that includes it.
