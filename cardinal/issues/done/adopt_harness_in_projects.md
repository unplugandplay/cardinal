# Adopt the shared harness in the existing projects

# Goal
Two existing projects each carry their own copy of the harness, adapted by hand. Move both to `harness sync`
so that an improvement made here reaches them with one command, and a drifted copy is caught by `just check`.

# Findings
* A comparison of the two harnesses showed identical hooks and check scripts, and differences limited to constants
  (commit scopes, root docs, skipped directories), a few sentences of the skills, `ci.yml` and the `justfile`.
* `quality-benchmark` and `smoke-test` differ entirely (retrieval versus documentation pipeline) and stay in the projects.

# Approach
1. Per project, on a branch: `harness init`, fill `.claude/harness.toml` and `.claude/update-docs-map.md` from the
   current skills, `harness sync` and resolve every conflict by deciding whether the project's variant belongs here
   (then change `files/`) or becomes a value.
2. Replace the harness jobs and recipes duplicated by `harness.just` and `.github/workflows/harness.yml`; keep the
   project-specific ones. Add `check-harness` to the project's `check`.
3. Check that `just check` and `just test-harness` pass in each project.

# Resources
* `docs/architecture.md`, `docs/adr/0001-shared-harness-synced-by-a-rust-tool.md`
* `specs/sync.md`

# Hints
* Adopt one project first; the second shows what the first missed in `files/`.

# Tracking
1. [x] Install the `harness` binary (`cargo install --path crates/harness`)
2. [x] Adopt the first project on branch `adopt-harness` (`harness check`, `just check`, `just test-harness` pass)
3. [x] Carry into `files/` what the first project revealed (commit here, push on request, re-sync the first project without conflict)
4. [x] Adopt the second project, same checks; redo 3 if new conflicts
5. [x] Report the state of both projects; ask whether to close the issue

## Decisions
* Source of each project: git URL of this repository with a `rev`.
* Branch `adopt-harness` in each project; nothing is pushed without an explicit ask.
* Installing the binary in `~/.cargo/bin` is accepted.
* No spec: the issue changes no behaviour.
* Project names in this issue are made generic when it is closed.
* Pinning a `rev` in each project makes the CI drift check reproducible (decided after the second adoption).
* Follow-ups left open: `cargo install --git ... --rev <branch>` is untested with a branch name; the tool is rebuilt on every cache miss in CI (a prebuilt binary would be a separate issue).
