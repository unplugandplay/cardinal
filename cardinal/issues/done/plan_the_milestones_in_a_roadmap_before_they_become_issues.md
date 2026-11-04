# Plan the milestones in a roadmap before they become issues

# Goal
What is planned but not yet an issue has no home in the repository: it stays in the user's head or in the agent's
memory, out of reach of the skills that rank and write the issues. Give each project a roadmap document that groups
the planned work into milestones, each milestone turning into issues when it is shaped, so the queue follows a macro plan
instead of being the plan.

# Findings
* The `roadmap` key of `.claude/cardinal.toml` exists and three skills read it (`issue-workflow` step 1,
  `create-issue` step 2, `prioritize-issues` step 2), but nothing writes it: no template, no seeded file, no skill.
* This repository points it at its own queue (`roadmap = ["issues/queue.md"]`); the seeded default is
  `docs/TODO.md`, a file that is never seeded.
* No link exists between a planned theme and the issues it produced: an issue can't say what it serves, and nothing
  says a theme is done.
* Planned work that is not an issue lands in the agent's memory today (e.g. an article planned after a series of
  issues), invisible to the other sessions and to the user's review.

# Approach
Decided with the user before the issue was written: the word "milestone" (not "epic", too connoted), one file,
milestones ordered by the user, no mechanical check yet. A milestone carries no date unless the project gives it one.
The file is `ROADMAP.md` at the root, the usual place beside `README.md` and `CHANGELOG.md`.

1. Seed a `ROADMAP.md` (copied once, owned by the project) and point the seeded `roadmap` key at it. One section
   per milestone, in the order the user wants: goal, why, state (`idea`, `shaped`, `in issues`, `done`), the issues it
   produced (links).
2. `skill:create-issue` attaches the new issue to a milestone (adds the link under it and names the milestone in the issue), or
   says it has none.
3. `skill:prioritize-issues` ranks the queue along the order of the milestones; it never reorders the milestones themselves
   (the user's call).
4. Turning a `shaped` milestone into issues sized by the limit of `skill:issue-workflow` step 1: a step of
   `skill:create-issue` or a skill of its own, to choose at step 1.
5. Optional: `just close-issue` sets the milestone `done` when its last issue closes.

Out of scope for now: a mechanical check of the milestone links (an issue with no milestone, a `done` milestone with an open issue)
in `just check-issue`; reconsider once the roadmap is in use and drift is measured.

This repository adopts it too: its `ROADMAP.md` was drafted by hand with the user on 2026-10-30, before this issue;
step 1 starts from that draft (format, states). Point its `roadmap` key at it (keeping `issues/queue.md` if the skills
still need it there).

# Resources
* `.claude/cardinal.toml` and `seed/.claude/cardinal.toml` (the `roadmap` key), `docs/architecture.md` (seeded versus
  managed content).
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`.
* `specs/delivered-quality.md` (QUA-14: the `roadmap` files never count as doc lines).
* `docs/adr/0017-the-source-declares-its-migrations.md`.

# Hints
* Changing the seeded default of `roadmap` changes a seeded text: check whether ADR 0017 asks for a migration
  (existing consumers keep their own value).
* The roadmap is a document of the project, never managed content: `cardinal sync` must not rewrite it.
* `delivered-quality` excludes the `roadmap` files from the doc lines: keep the new file listed in the key so editing
  the roadmap is not counted as documentation work.
* A root document that the skills cite goes in `root_docs`, whose files `delivered-quality` counts as doc lines;
  QUA-14 subtracts the `roadmap` files, so `ROADMAP.md` stays uncounted as long as the `roadmap` key lists it.

# Tracking

Plan: validated

## Understanding
Seed a `ROADMAP.md` of milestones and point the seeded `roadmap` key at it; teach `create-issue`, `prioritize-issues`
and the closing of an issue to keep it. Out of scope: any mechanical check of the milestone links. No migration: the
format does not change and the `roadmap` value of an existing project is its own (the old default was never seeded).

## Plan
1. [x] Seed the roadmap: `seed/ROADMAP.md` (template from this repository's draft: intro, states, one example
   milestone), seeded `roadmap = ["ROADMAP.md"]`, this repository's key moved to `ROADMAP.md`, the seeded table of
   `docs/architecture.md`; verified by a test of `tests/skills.rs` that every file the seeded `roadmap`
   key lists is seeded by `seed/` (~60 lines).
2. [x] Keep the roadmap from the skills: `create-issue` attaches each new issue to a milestone (link under it, an
   optional `# Milestone` section in the issue) or says it has none, and cuts a `shaped` milestone into issues sized by
   the limit of `issue-workflow` step 1 (state set to `in issues`); `prioritize-issues` ranks along the order of the
   milestones and never reorders them; the closing rule of `issue-workflow` sets a milestone `done` when its last
   issue closes; verified by new `REQUIRED_PASSAGES` of `tests/skills.rs`, then `just sync` (~90 lines).

## Decisions
- Mode: autonomous
- Base commit: 107a8ccad8dde13bd052e9601fb01e55896e7f4a
- Shaped milestone into issues: a step of `create-issue`, no new skill (fixed context).
- Milestone `done`: an instruction of the closing rule of `issue-workflow`, no code in `close-issue`.
- This repository's `roadmap` key: `["ROADMAP.md"]` alone; the skills read `issues/queue.md` directly.
- An issue names its milestone in an optional `# Milestone` section (`check-issue` accepts unknown sections).

## Summary of deliverable 1
- done: `seed/ROADMAP.md` seeded, the seeded and this repository's `roadmap` keys point at `ROADMAP.md`, the seeded table of `docs/architecture.md` lists it.

| check | result |
|---|---|
| tests | pass (`seeded_roadmap_is_seeded` new) |
| lint | pass |
| specs | pass (no spec: harness content) |
| docs | `docs/architecture.md` |
| code review | low, clean |

- red: none (no spec id); the test failed first with `the seeded roadmap 'docs/TODO.md' is not seeded`.
- Code review: no findings
- not verified: a real `cardinal init` in a fresh project (SYN-4 already copies any file of `seed/`).
- decisions needed: none.

## Summary of deliverable 2
- done: `create-issue` attaches each issue to a milestone (`# Milestone` section, link in the roadmap) and cuts a shaped milestone into issues; `prioritize-issues` ranks along the milestones and never reorders them; `issue-workflow` sets a milestone `done` at the close of its last issue; `docs/workflow.md` and `docs/usage.md` follow.

| check | result |
|---|---|
| tests | pass (4 new `REQUIRED_PASSAGES`) |
| lint | pass |
| specs | pass (no spec: harness content) |
| docs | `docs/workflow.md`, `docs/usage.md` (projects seeded before keep `docs/TODO.md`) |
| code review | low, clean |

- red: none (no spec id); the test failed first with the 4 `FAIL - <skill>: missing '<passage>'` lines.
- Code review: no findings
- not verified: an agent following the new steps in a real session.
- decisions needed: none.

## Review of the whole issue (high)
3. [x] Fix the findings of the `high` review: the milestone set `done` on the Merge back path, the ask before
   committing a roadmap file with changes not of `create-issue`, no state change of an `idea` or `done` milestone
   unasked, `check-issue` re-run after the `# Milestone` section, the rank of an issue with no milestone, the seeded
   example milestone in a comment.

- done: the six fixes above.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | none to change (the docs state the behaviour, not these details) |
| code review | high: 9 findings; low on the fixes, see below |

- red: none (wording fixes of the harness content).
- Code review: no findings (low, on the fixes). High: 9 findings; 6 fixed above; not fixed: the orphan `ROADMAP.md` of a project seeded before
  (its `roadmap` key is its own, a rewrite would need a format raise; `docs/usage.md` says to point it by hand), the
  `done` state set by code in `close-issue` (decision 2 a: no code), the `low` review a close now needs when
  `close-issue` rewrites `ROADMAP.md` (the gate's rule for any document outside `issues/`).
- not verified: an agent following the new steps in a real session.
- decisions needed: none.
