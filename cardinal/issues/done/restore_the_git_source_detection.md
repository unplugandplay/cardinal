# Restore the git source detection

# Goal
Commit `e5352ff` (clippy pedantic) rewrote `is_git_url` and changed which sources count as a git URL rather than a
local directory. `sync`, `update`, `cardinal run` and the launcher all branch on it, so a source can switch from a
clone to a local read (or the reverse) with no change of the project. Bring back the old decision, or decide and
specify a new one, and test it.

# Findings
* Before: `source.contains("://") || source.starts_with("git@") || source.ends_with(".git")`. After
  (`crates/cardinal/src/lib.rs:358`): the last term became `Path::new(source).extension()` equal to `git`, case
  insensitive.
* `Path::extension` of `/x/.git` is none (`.git` is a stem): that source was cloned before, is now read as a local
  directory. `../x.git/` (trailing slash) and `X.GIT` go the other way, from local directory to clone.
* Callers: `lib.rs:706`, `lib.rs:734`, `lib.rs:982` (`update` refuses a local source), `run.rs:87` (cached binary
  vs local build), `launcher.rs:68`.
* Found by the `high` review of `rename_the_harness_named_tools_to_cardinal`; no spec names the rule.

# Approach
1. Restore `ends_with(".git")` with an `#[allow]` and the reason, unless a source form above should change.
2. Write the rule as a requirement (`specs/sync.md`) and test it on the forms above.

# Specs
* `specs/sync.md`: a requirement for what a git source is, new.

# Resources
* `crates/cardinal/src/lib.rs` (`is_git_url`), `run.rs`, `launcher.rs`
* `git show e5352ff -- crates/cardinal/src/lib.rs`

# Tracking

Plan: validated

## Understanding
Bring back the pre-`e5352ff` rule (`ends_with(".git")`, case sensitive) with an `#[allow]` stating why, and specify it as SYN-49.
Out of scope: any new source form (e.g. `file://` already counts as git through `://`).

## Plan
1. [x] Restore and specify the git source rule: `is_git_url` back to `ends_with(".git")` under `#[allow(clippy::case_sensitive_file_extension_comparisons)]`; verified by a binary test of `update` (a source `<repo>/.git` is cloned, `<dir>/x.git/` and `<dir>/X.GIT` are refused as local directories) plus a unit test of the five forms; makes true SYN-49. ~60 lines.
2. [x] Fixes of the `high` review: SYN-49 names `init`, the binary test runs `init` on `<repo>/.git`; the path git source faults go to `issues/done/normalize_a_path_git_source.md`; verified by `just check`; makes true SYN-49.

## Decisions
- Mode: autonomous
- Base commit: 88a0f3ac215b6f3dd2d131675654afbcf9e9950c

## Deliverable 1
- done: `is_git_url` back to `ends_with(".git")` under an `#[allow]` with its reason; SYN-49 specified and traced.
- red: SYN-49: `update` on `<repo>/.git` failed with "`source` is a local directory, which has no revision to update".
- Code review: no findings
- not verified: the `git@` and `https://` forms through the binary (unit test only).
- decisions needed: none

## Deliverable 2
- done: SYN-49 names `init` (it branches on `is_git_url` too) and its binary test runs `init` on `<repo>/.git`.
- red: none (the rule was already fixed by deliverable 1; the added `init` call guards it).
- Code review: no findings (`low`); `high` review, 8 findings: 1 fixed here (SYN-49 omitted `init`); 7 (relative clone cwd, cache key of the raw string, `cargo install --git <path>`, CI installer without `*.git`, test gaps and the altitude note on them) predate this issue and go to `issues/done/normalize_a_path_git_source.md`.
- not verified: none
- decisions needed: none
