# 0002. `rev` pins the resolved commit

Status: Accepted
Date: 2026-10-24

## Context
A project pins the revision of the shared harness it was synced from (`rev` in `.claude/harness.toml`) so that
the CI drift check is reproducible. A branch name moves under the project; a tag can be moved too. Updating by
hand means looking up a hash and editing a file the project owns, comments included. `cargo install --git
--rev` in the CI step accepts a hash, a tag or a branch.

## Decision
`harness update` resolves the requested revision (branch, tag or commit; the source's default branch head when
omitted) and writes the full commit hash to `rev`, the requested name in a trailing comment
(`rev = "<hash>"  # v1.2`). The line is rewritten textually, or inserted after `source`: no TOML
serialiser, so the project's comments and layout survive.

## Consequences
* The drift check stays reproducible whatever happens to the branch or tag afterwards.
* `rev` is less readable; the comment compensates, and a hand-written name is still read by `sync` and
  `check`, it is just no longer what the tool produces.
* Going back to a tag is one `update --rev <tag>`.
* The comment is informational: only the hash is read.
* A pin outlives its branch only while the source still serves the commit. `sync` and `update` fetch a full
  hash that no ref points to by hash; hosts differ on serving it, and when it is gone the error says to run
  `harness update`. The CI `cargo install --rev` has the same dependency on the host.
