---
priority: medium
why: 'the roadmap grows with every release and its open milestones get hard to find; nothing waits on it'
---
# Archive the released milestones

# Goal
Released milestones stay in the roadmap forever: `close-issue` sets them `done` (CLS-8), `release` adds
`- Released: v<version>` (REL-9), and nothing ever moves them. The roadmap soon holds more finished milestones than
open ones, and the user can no longer see which are open. The outcome: `release <version>` moves each milestone it
marks released out of the main roadmap into an archive document, so the main one holds only the milestones still to
land, with no manual step.

# Findings
* 2026-11-02, this repository: `ROADMAP.md` holds 6 milestones, 5 of them `done` with a `Released:` line, one
  `in issues`.
* `release` already lists the released milestones under `### Milestones` of `CHANGELOG.md` (REL-9), by name only:
  their goal, why and issue list live only in the roadmap.
* `next-issue` reports a milestone no roadmap heading names (NXT-13), and `close-issue` rewrites the issue links of
  the `roadmap` documents (CLS-2): an archive left out of `roadmap` would break both for a late issue of a released
  milestone.
* REL-5 restores the `roadmap` documents when `release_version_command` fails.

# Approach
1. Decided with the user: the archive stays listed under `roadmap`, so NXT-10/13 and CLS-2 keep reading it.
2. Name the archive, candidates:
   a. a new key `roadmap_archive` of `.claude/cardinal.toml` (no archive without it, today's behaviour);
   b. the last document of `roadmap` when it lists two or more;
   c. a fixed name next to the first document (`ROADMAP-done.md`).
3. `release <version>` moves the whole section of each milestone it marks (heading to the next heading, `Released:`
   line included) to the top of the archive (newest first) or its end, creating the archive with a short header
   when missing; links stay valid when both documents share a directory, else they are rewritten.
4. Seed: the seeded `ROADMAP.md` names the archive in its states paragraph; a seeded archive is optional.

# Specs
* `specs/release.md`: REL-9 amended or a new REL requirement for the move, REL-5 extended to the archive.

# Resources
* `crates/cardinal/src/tools/release.rs` (the roadmap writes), `crates/cardinal/src/tools/roadmap.rs` (the shared
  grammar).
* `specs/release.md`, `specs/close-issue.md` (CLS-2, CLS-8), `specs/next-issue.md` (NXT-10, NXT-13).
* `seed/ROADMAP.md`, `docs/workflow.md`, `docs/usage.md`.

# Hints
* A milestone section ends at the next `##` heading outside fenced blocks and HTML comments: reuse the grammar of
  `roadmap.rs`, never a raw text search (the template milestone of the seed sits in an HTML comment).
* The move must keep the order of the milestones still in the main roadmap: `next-issue` orders by it.
