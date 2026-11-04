---
priority: medium
why: 'telling the open milestones from the closed ones means reading every section; nothing waits on it'
---
# Show the state of the milestones

# Goal
The state of a milestone is a `- State:` line inside its section: to know which milestones are open, the user reads
the whole roadmap. The outcome: one read-only command, `cardinal run roadmap` (`just roadmap`), prints a table of the
milestones of the `roadmap` documents in their order: name, state, release, issues open over listed, document.

# Findings
* 2026-11-02, this repository: `ROADMAP.md` holds 6 milestones over 106 lines; the one open milestone is the last.
* The grammar of milestones (`## <name>` outside fenced blocks and HTML comments, `State:`, `Released:`, `Issues:`)
  is already shared by `release`, `close-issue` and `next-issue` in `crates/cardinal/src/tools/roadmap.rs`.
* `close-issue` already resolves the issue links of a milestone against `issues/done/` (CLS-8).

# Approach
1. A tool `roadmap` under `crates/cardinal/src/tools/` and a recipe in `files/cardinal-issues.just`, writing nothing.
2. Columns, candidates: name, state, `Released:` version, open / listed issues (a link to `issues/done/` counts
   closed), document; an option to hide the released ones (`--open`), or hide them by default.
3. Report a milestone with no `State:` line or an unknown state rather than skip it.

# Specs
* A new `specs/roadmap.md` (written at `issue-workflow` step 1).

# Resources
* `crates/cardinal/src/tools/roadmap.rs`, `crates/cardinal/src/tools/close_issue.rs` (link resolution),
  `crates/cardinal/src/tools/next_issue.rs` (how the `roadmap` documents are found).
* `files/cardinal-issues.just`, `features.toml` (the recipe belongs to the issue-files feature).
* `seed/ROADMAP.md`, `docs/usage.md`.

# Hints
* Reuse the link resolution of `close-issue` rather than a second one, so both count a closed issue the same way.
* Works with an archive document too ([archive_the_released_milestones](archive_the_released_milestones.md)): read
  every `roadmap` document in order.
