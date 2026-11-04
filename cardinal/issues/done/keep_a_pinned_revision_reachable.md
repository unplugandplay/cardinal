# Keep a pinned revision reachable after the source rewrites its history

# Goal
`rev` pins a full commit hash (ADR 0002) so that the CI drift check is reproducible. If the source force-pushes or
deletes the branch that held that commit, `sync` and `check` must still find it, or fail with a message that
says how to recover, instead of breaking the check with "unknown revision".

# Findings
* `resolve_in` in `crates/harness/src/lib.rs` resolves `<rev>^{commit}` among the refs the fetch brought; `fetch
  --tags origin` brings branches and tags only.
* A commit no branch or tag points to any more is then unknown to the cached clone, and, on a fresh clone (CI),
  absent altogether.
* Not measured: which hosts let a client fetch an unreferenced commit by hash.

# Approach
1. When the revision is a full hash and does not resolve, try `git fetch origin <hash>` (hosts such as GitHub
   allow it for reachable or recently unreferenced commits) before giving up.
2. When it still fails, name the fix in the error: the pinned commit is gone from the source, run `harness update`.
3. Decide whether `check` in CI, which installs the tool at `rev` with `cargo install --git --rev`, has the same
   weakness, and document it.

# Specs
* `specs/sync.md`: a new requirement for the resolution of a pinned hash that no ref points to any more.

# Resources
* `docs/adr/0002-rev-pins-the-resolved-commit.md`, `crates/harness/src/lib.rs` (`fetch`, `resolve_in`)
* `files/.github/workflows/harness.yml`

# Hints
* Tests use a local git repository as the source whose branch is rewritten after the pin; never the network.
* Depends on `update_the_harness_to_a_revision` (the `update` command named in the error).

# Tracking
1. [x] Recover or explain a pinned hash that no ref points to any more (SYN-17): fetch by hash in `resolve_in`, error naming `harness update`, tests on local repositories, docs (ADR 0002, architecture, CI weakness).

## Decisions
* The fetch by hash is tried for a full hash only; an abbreviated hash or a name keeps "unknown revision".
* The error says the pinned commit is gone from the source and to run `harness update`.
* The CI weakness (`cargo install --git --rev`) is documented, the workflow is not changed.
