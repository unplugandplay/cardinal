# Update a project to a revision of the harness with one command

# Goal
A project pins the revision of the shared harness it was synced from (`rev` in `.claude/harness.toml`), so the CI
drift check is reproducible. Moving to a newer revision currently means looking up a hash, editing `harness.toml` by
hand in every project, then running `harness sync`. One command should do it: update to the latest revision of the
source, or to a given one (a tag, for example), then sync.

# Findings
* `rev` is read by `sync` and `check` for a git source (branch, tag or commit); nothing writes it after `init`.
* A pinned branch name moves under the project, so only a commit or a tag keeps the check reproducible.
* The CI step of `harness.yml` installs the tool with `cargo install --git <source> --rev <rev>`; an untested case
  is a branch name as `rev`.

# Approach
1. Add `harness update [--rev <tag|branch|commit>]`: resolve the revision (the source's default branch head when
   omitted), write it to `rev` in `harness.toml`, then run the `sync`.
2. Decide what `rev` stores: the resolved commit hash (reproducible; recommended), keeping the requested name in a
   comment, or the name as given.
3. Decide what to do on a conflict: stop before writing `rev`, so the project is left as it was.
4. Report the previous and the new revision, and the files changed.

# Specs
* `specs/sync.md`: new requirements for `update` (revision resolution, `rev` written only on success, failure leaves
  the project untouched).

# Resources
* `docs/architecture.md` (Sources), `specs/sync.md`, `crates/harness/`
* `files/.github/workflows/harness.yml` (how CI installs the tool at `rev`)

# Hints
* Tests use a local git repository as the source, never the network.
* Each project gets its first `rev` by hand from the adoption of the harness; this command replaces that gesture.

# Tracking

1. [x] Resolve a revision to a commit hash and rewrite `rev` textually, with ADR 0002 (SYN-11 in part, SYN-12)
2. [x] `harness update`, nominal path: local source refused, report of previous and new revision (SYN-11, SYN-14, SYN-16)
3. [x] Atomicity: conflict, unknown revision, `--dry-run`, `--force` leave the project untouched (SYN-13, SYN-15); spec `accepted` then `implemented`

## Decisions

* `rev` stores the full resolved commit hash, the requested name in a trailing comment (ADR 0002, written in deliverable 1).
* `update` takes `--rev`, `--dry-run` and `--force`; the last two are passed to the `sync`.
* `rev` is rewritten textually (insert under `source` when absent), no new dependency, comments preserved.
* `update` does a `sync --dry-run` first and writes nothing on a conflict; a local-path source is refused.
* Requirements SYN-11 to SYN-16 in `specs/sync.md` (validated, still `draft`).
