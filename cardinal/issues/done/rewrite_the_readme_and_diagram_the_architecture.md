# Rewrite the README and diagram the architecture and the processes

# Goal
The README is dense and does not tell at a glance what a reader wants to know: what the harness is, what it gives a
project, how to install and use it in a few commands, where to go next. `docs/architecture.md` states the design and
every edge case in long paragraphs, so neither the user nor an agent reads it whole. Rewrite the README as a short
entry point, make the architecture document skimmable, and show the architecture and the processes (sync, hooks and
gates, issue workflow with its worktrees, evaluation from the transcripts) as mermaid diagrams.

# Findings
* `README.md`: 67 lines, 5.3 kB; after a two-line intro, a command block whose comments run over several lines, then
  long paragraphs on the installed launcher, the transcript readers (`session-metrics`, `rule-breaches`, `delivered-quality`,
  `fixed-context`, `harness-report`) and on the ignore settings of eight tools for `.claude/worktrees/`. No section
  headings, no list of what a consumer project gets (skills, hooks, recipes, CI), no diagram.
* `docs/architecture.md`: 330 lines, 30.4 kB, 9 sections, mostly paragraphs of several hundred words (hooks and
  their declaration: about 100 lines; the evaluation from the transcripts: about 70 lines); no diagram.
* No mermaid diagram anywhere in `README.md` or `docs/`.
* Many edge cases of the architecture document are already requirements of `specs/` (review gate, rule breaches,
  delivered quality, sync) or ADRs (`docs/adr/0001` to `0011`); `specs/README.md` says the spec says what, the ADR
  why, `docs/` how the code is laid out.

# Approach
1. Measure: list what the README and each paragraph of the architecture document state, and mark what is already in
   a spec, an ADR, a skill or a doc comment.
2. README: what it is (one paragraph), what a project gets, install and the main commands (one line each), the daily
   flow (issue to commit), links to the architecture, the specs and the ADRs. Move the reference details (transcript
   readers, worktree ignore settings) to `docs/` (e.g. a usage or reference page) or to the help of the commands.
3. Architecture: an overview per component (role, files, flow, links to the spec and ADRs), duplicated edge cases moved
   or dropped.
4. Diagrams (mermaid, rendered by GitLab and GitHub): the components and what `sync` copies where; the hook and gate
   chain of a commit; the issue workflow and its worktrees (`start-issue`, deliverables, `finish-issue`); the
   evaluation pipeline (transcripts, readers, report). Pick which ones go in the README and which in `docs/`.
5. Check with `skill:update-docs` that nothing that only lived in the old text is lost.

# Resources
* `README.md`, `docs/architecture.md`, `docs/adr/`, `specs/`, `files/.claude/skills/update-docs/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`, `justfile`, `harness.just`

# Hints
* `README.md` and `docs/architecture.md` are project-owned, not copies of `files/`: no `just sync` needed for them.
* The README of a consumer is not touched by `sync`; this issue is about this repository's documentation only.
* Diagrams stay true only if small: one idea per diagram, names that exist in the code.
* `prove_the_gate_specs_through_the_binary` is still open and may move more edge cases into the specs; leave the
  gate sections short and link the spec rather than restating it.

# Tracking

Plan: validated

## Understanding
Make the README a one-screen entry point (what, what a project gets, install, daily commands, links) and the
architecture a set of short component overviews, each with a mermaid diagram, its edge cases left to the specs, ADRs
and doc comments that already hold them. Out of scope: any change of behaviour, the consumers' READMEs, the specs.

## Plan
1. [x] README and usage reference: rewrite `README.md` (what it is, what a project gets, install, one line per
   command, the daily flow, links) with the components diagram; move the reference details (launcher install, transcript
   retention, the transcript readers, the worktree ignore settings) to a new `docs/usage.md`; point the README rows of
   `.claude/update-docs-map.md` at it too; verified by `skill:update-docs` (nothing lost) and the mermaid blocks parsing
   (`mmdc` if available, else a GitHub preview); ~250 lines.
2. [x] Architecture, what is shared and the binary: restructure "What is shared, and how", "Making files identical",
   "Tools and their binary" and "Sources" as overviews with a `sync` flowchart and a `harness run` launcher diagram,
   duplicated edge cases replaced by links to `specs/sync.md`, `specs/tool-binaries.md` and the ADRs; verified by a
   statement-by-statement check that each dropped one lives in a spec, an ADR or a doc comment; ~250 lines.
3. [x] Architecture, hooks and gates: "Hooks and their declaration" as an overview with a sequence diagram of a commit
   (`review-gate` record, check, native; `clippy-gate` at Stop; `format-rust`), edge cases linked to
   `specs/review-gate.md`, `specs/clippy-gate.md`, ADRs 0004, 0005, 0007; same check; ~200 lines.
4. [x] Processes: a new `docs/workflow.md` with the issue lifecycle flowchart (create, queue, steps 0 to 3 with the
   stops and the autonomous mode, merge back, close) and the parallel worktree lifecycle (`start-issue`,
   `finish-issue`); "Parallel issues" of the architecture trimmed to its design with a link; verified by checking each
   step name against `files/.claude/skills/issue-workflow/SKILL.md` and the recipes; ~200 lines.
5. [x] Evaluation: "The instruction audit", "The evaluation from the transcripts" and "The cost of a turn" as an
   overview with the pipeline diagram (transcripts and git history, readers, TSV, `harness-report`,
   `evaluate-harness`), edge cases linked to `specs/evaluation.md`, `specs/rule-breaches.md`,
   `specs/delivered-quality.md`, `specs/harness-report.md`; same check; ~200 lines.
6. [x] Fixes of the `high` review of the whole issue: the ten findings; verified by `just check`, a `low` review; ~40
   lines.

## Decisions
- Mode: autonomous
- Base commit: 0235194
- Reference details out of the README go to a new `docs/usage.md`.
- Edge cases already in a spec or an ADR are dropped from the architecture and linked.
- The README addresses any project adopting the harness, not the author's projects only.
- The processes go to a new `docs/workflow.md`.

## Deliverable 1
- done: README cut to one screen (what a project gets, components and daily flow diagrams, install, commands, links);
  the reference moved to `docs/usage.md` (commands, launcher, evaluation readers, worktree ignore settings);
  `update-docs-map.md`, `docs/architecture.md` and ADR 0009 repointed from the README to it.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | every statement of the old README found in the new README or `docs/usage.md`; relative links resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering (no mermaid CLI allowed here); the links to `docs/workflow.md` wait for
  deliverable 4.
- decisions needed: none

## Deliverable 2
- done: "What is shared" (sync flowchart, kinds table, values), "Sources" moved up next to it, "Tools and their
  binary" (launcher flowchart, exit code table); edge cases replaced by the SYN and BIN ids that hold them.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | each dropped statement found in `specs/sync.md` or `specs/tool-binaries.md`; the two found nowhere (a branch `rev` resolved without fetch, exit 2 before BIN-7 and BIN-17) kept; links resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering
- decisions needed: none

## Deliverable 3
- done: "Hooks and their declaration" as a hook table, the declaration and warnings in two paragraphs, the review gate
  as a sequence diagram with one bullet per mode, the Rust hooks in two paragraphs; edge cases linked to the SYN, REV
  and CLG ids.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | each dropped statement found in `specs/sync.md`, `specs/review-gate.md`, `specs/clippy-gate.md` or the doc comments of `review_gate.rs` (`--get`); the rest (marker content, waiver stamp, `GIT_INDEX_FILE`, nested project) kept; links resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering
- decisions needed: none

## Deliverable 4
- done: new `docs/workflow.md` (life of an issue, inside `issue-workflow`, parallel issues as a sequence diagram,
  the evaluation skills); "Parallel issues" of the architecture cut to its design in bullets with the PAR ids; the
  README and `docs/usage.md` link to it.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | step names and numbers checked against `issue-workflow` SKILL.md and details.md ("Merge back", "Worktree of an issue"); dropped statements found in `specs/parallel-issues.md` (PAR-11, PAR-15, PAR-16) or moved to `workflow.md`; links and anchors resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering
- decisions needed: none

## Deliverable 5
- done: "The evaluation from the transcripts" as a pipeline diagram, a table of the readers and five design choices;
  "The instruction audit" folded into it as one paragraph; edge cases linked to the EVA, BRE, QUA and REP ids.
  `docs/architecture.md`: 330 lines, 30.4 kB at the base, 286 lines, 19.3 kB now, with 4 diagrams.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | dropped statements spot-checked in the four specs (medians, `self:`, the 20-word language rule, commits in no session, sub-agents, `--window-days`, `--anonymous`); links resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering
- decisions needed: none

## Deliverable 6
- Code review: 10 findings (the `high` review of `0235194..HEAD`), each fixed:
  1. `--archive` edge label quoted in the evaluation diagram.
  2. Launcher diagram: the digest mismatch fails (BIN-10), cargo covers any other download failure.
  3. The exit 2 of old binaries and launchers names its cases again.
  4. `workflow.md`: the fixes after the `high` review go back to step 3.2, not 3.1.
  5. `update-docs-map.md`: rows for the README "What a project gets" table and for `docs/workflow.md`.
  6. Evaluation diagram: `session-metrics` reads the git history for the revision in force.
  7. README: `update --force`, and `check` failing on a hook that needs a newer `harness`.
  8. README diagram: `harness sync` installs the gate, in each clone.
  9. `workflow.md`: the size limits and the autonomous mode rules no longer copied, the skill named instead; the
     launcher paragraph of `usage.md` and the architecture's say different things (what to do, how it works) and stay.
  10. Sync diagram: SYN-4 out of the managed range; the prose names the lock and the gate.

| check | result |
|---|---|
| tests, lint, specs | `just check` passes |
| docs | links resolve |
| code review | see below |

- red: none (docs only, no test)
- Code review: no findings
- not verified: the mermaid rendering
- decisions needed: none

## Status
Done: all six deliverables committed; left: the mermaid rendering to check in a GitHub preview.
