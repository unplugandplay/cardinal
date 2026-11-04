# 0010. The tools ship in the `harness` binary of the pinned revision

Status: Accepted
Date: 2026-10-27

## Context
Every hook, check and transcript reader is a `rust-script` (ADR 0001, 0006), so a consumer needs a Rust toolchain,
`rust-script` and a first compile of each script, even when it is not written in Rust. Measured on this machine
(`require_code_review.rs`, `clippy_gate.rs`): a cold first run takes 2.2 to 3.5 s, the same script at a new path (a
new issue worktree) 0.4 s, a warm run 3 to 6 ms against 1 ms for the compiled binary. Start time is not the cost; the
prerequisites are. ADR 0006 kept the readers out of `harness` because the installed binary may be older than the
managed files; it also forced a SHA-256 stamp on each script that includes a module.

## Decision
The scripts become modules of the `harness` crate, run as `harness run <tool> [args]`. `run` executes the `harness`
binary of the commit pinned in `harness.toml`, kept in a cache per commit and platform, fetched prebuilt from the
source's release of that commit (checksum verified) or built from its checkout with `cargo` when none is published.
A local directory source (this repository) builds its own crate, which replaces `rust-script` where the tools are
developed. The installed `harness` is only a launcher; the pinned commit selects the code that runs. Supersedes the
script form of ADR 0006 (its shared module stays one module).

## Consequences
* A consumer needs `harness` and `just`; Rust only for a commit with no published binary.
* The script tests become `cargo test` of this repository; consumers no longer run them.
* The source's CI must publish a release per commit a project can pin, for each platform.
* The hook commands change in every project's settings (`sync` replaces them, ADR 0007).
* The SHA-256 stamps of the transcript readers go away.
