---
priority: medium
why: 'a fix before the comfort of its milestone: three copies of the roadmap parser, and a `## ` line in a comment can set a milestone done'
---
# Share one roadmap grammar between the tools

# Goal
Three tools read the `roadmap` documents, each with its own parser: `release` (`mark_released`, REL-9),
`close-issue` (`milestones_done`, CLS-8, a copy of the former) and `next-issue` (`milestones`, NXT-10). None skips a
`## ` line inside a fenced code block or an HTML comment, so such a line cuts the body of a milestone early: its
`State:` or its `Issues:` list lands in a fake section, and a commented template milestone listing real issues could
be set `done`. The outcome: one reading of the roadmap (milestone name, body lines, state, listed issues) that the
three tools share, fences and comments skipped.

# Findings
* 2026-11-02, the `high` review of
  [set_the_milestone_done_when_its_last_issue_closes](set_the_milestone_done_when_its_last_issue_closes.md)
  (commit acf19bd): two findings left unfixed there, the duplicated section parser and the `## ` line in a fence or
  an HTML comment.
* `seed/ROADMAP.md` describes the shape of a milestone in prose and a fenced block; a project that keeps a template
  milestone in a comment would hit the misread.

# Approach
1. A module of the roadmap grammar under `crates/cardinal/src/tools/` (milestones with their name, body range, state
   line, issue links), skipping fenced blocks and HTML comments like `close_issue::link_targets` skips fences.
2. `release`, `close-issue` and `next-issue` read through it; their specs gain the fence and comment case as a
   scenario where they draw that boundary.

# Specs
* `specs/release.md` (REL-9), `specs/close-issue.md` (CLS-8), `specs/next-issue.md` (NXT-10): a scenario each, or
  one requirement of the roadmap format if the three are amended together.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/release.rs` (`mark_released`), `close_issue.rs` (`milestones_done`, `link_targets`),
  `next_issue.rs` (`milestones`).
* `seed/ROADMAP.md`: the shape of a milestone.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
One module `roadmap.rs` reads a roadmap into milestones (name, body lines, `State:` value, `Issues:` list), skipping
fenced blocks and HTML comments; `release`, `close-issue` and `next-issue` read through it. Out of scope: other
Markdown readers (`link_targets` keeps its own link scan, sharing only the fence tracking).

## Plan
1. [x] Shared roadmap grammar: `tools/roadmap.rs` (milestones outside fences and comments, a body without their
   lines), `mark_released`, `milestones_done` and `next_issue::milestones` on it, the fence tracking shared with
   `link_targets`; verified by a test each through the binary (a fenced `## ` line and a commented template
   milestone); makes true REL-9, CLS-8, NXT-10 (amended). ~400 lines. (autonomous)
2. [x] Fixes of the `high` review: a fence closes on a bare fence of its character at least as long, a comment opened
   after text or a heading's trailing comment is tracked, an indented code block opens none; verified by unit tests
   of `roadmap.rs`; makes true REL-9, CLS-8 (internal). (autonomous)

## Decisions
- Mode: autonomous
- Base commit: d26d4eec24a9d4957de250476ab0363124485ae6
- Lines of a fenced block or a comment inside the body of a real milestone are skipped too (an example `State:` never counts).
- The fence tracking is shared with `close_issue::link_targets`; its link scan stays its own.

## Deliverable 1
done: `tools/roadmap.rs` reads the milestones outside fences and HTML comments; `release`, `close-issue` and
`next-issue` read through it, `link_targets` shares its fence tracking.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (REL-9, CLS-8, NXT-10 amended, accepted) |
| docs | n/a: no page describes the parsing beyond the specs |
| code review | low: no findings |

red: REL-9, CLS-8, NXT-10: `release` listed `### Milestones` (the fenced `Fake` and the commented `Template`);
`close-issue` left `Seven` `in issues` and set the commented `Template` done; `next-issue` printed `Next: issues/a.md`.

Code review: no findings

not verified: a comment opened in the middle of a line (`text <!--`) is not tracked; only a line starting with `<!--`
opens one.

decisions needed: none

## Deliverable 2
done: the `high` review of `d26d4ee..de10fb2` fixed in `roadmap.rs` (fence length and info string, comment opened
mid-line, trailing comment of a heading, indented `<!--`).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass |
| docs | n/a |
| code review | low: 1 finding, fixed |

red: none: the unit tests were written with the fix, not run red first.

High review: 9 findings. Fixed: fence length/info string, comment opened after text, indented `<!--`, heading with a
trailing comment. Not fixed:
- an unclosed `<!--` hides the rest of the document: that is how Markdown renders it, the roadmap shows the same;
- `Issues:` list read across a hidden block: a list continues past a fenced example in Markdown too;
- line indexes in `Milestone`: the rewrites need the indexes, all callers split with `split_inclusive`;
- fence helpers in `roadmap.rs` rather than a `markdown` module: two functions, one more user (`link_targets`);
- linear scan of the marked lines: a handful of milestones per roadmap.

Code review: 1 finding
- a `<!--` in a code span opened a comment: fixed, the comment tracking reads the text outside the code spans.

not verified: none

decisions needed: none
