# Read the fences of the issue sections as the specs say

# Goal
`section` in `crates/harness/src/tools/delivered_quality.rs` cuts the sections of a done issue file (`## Plan`,
`# Tracking`) for QUA-3, QUA-6 and QUA-11 of `specs/delivered-quality.md`. Its fence handling and the wording of the
three requirements disagree, so a test written from the spec can fail against the code, and some issue files are
read wrong. Expected: one fence rule, worded once and shared by QUA-3, QUA-6 and QUA-11, that says what the code
does, and a heading search that skips fenced lines.

# Findings
* Found by the `high` review of `issues/done/align_the_tracking_section_end_of_qua_6.md`.
* `section` treats a line as a fence only when it starts at column 0 with three backticks: a `~~~` fence or an
  indented fence is not a fence. QUA-3 and QUA-6 say "outside a fenced code block": a `~~~` fence holding a
  column-0 `# build` ends `# Tracking` there, and the `Code review` lines after it are dropped.
* QUA-11 says the section ends "at the next line starting with `# `", with no fence clause, though its code goes
  through the same fence-aware `section`.
* `section` finds its heading with `skip_while` without tracking fences: a `# Tracking` quoted in a fence earlier in
  the file (a Goal quoting the template) starts the section there, and the closing fence then flips `fenced`, so
  every line up to the next fence is kept.

# Approach
1. Define the fence once in the spec (which lines open and close one), and refer QUA-3, QUA-6 and QUA-11 to it.
2. Either widen `section` to that definition (`~~~`, indented fences) or narrow the wording to what it reads today.
3. Track fences in the heading search too, with a test whose Goal quotes `# Tracking` in a fence.

# Specs
* `specs/delivered-quality.md`: amends QUA-3, QUA-6, QUA-11.

# Resources
* `specs/delivered-quality.md`, `crates/harness/src/tools/delivered_quality.rs` (`section`, `deliverables`,
  `reviews`, the red-run reader), `crates/harness/tests/delivered_quality.rs`

# Tracking

Plan: validated

## Understanding
One fence rule, QUA-12, shared by QUA-3, QUA-6 and QUA-11: a fence opens on a line starting (indent allowed) with
three or more backticks or tildes and closes on a line of only the same character, at least as many; headings are
found and ended outside fences only. The code is widened to it (option "widen" of the Approach), not the wording narrowed.

## Plan
1. [x] Fence-aware sections: `section` tracks fences from the first line (heading search included) with the QUA-12
   rule, and `deliverables` picks `## Plan` only outside a fence; verified by a fixture issue whose Goal quotes
   `# Tracking` in a backtick fence and whose Tracking holds an indented `~~~` fence with `# build`; makes true QUA-12
   (and QUA-3, QUA-6, QUA-11 as amended). ~80 lines.
2. [x] Fixes of the `high` review: a line opening with backticks that holds more backticks is inline code, not a
   fence; one fence reader shared by `delivered_quality` and `check_specs` (`tools/mod.rs`); the fixture also proves
   the `red:` items of QUA-11 outside fences; makes true QUA-11, QUA-12.

## Decisions
- Mode: autonomous
- Base commit: 11740bfd2394a8f416481ee1067298465e39449a
- Fence indentation: any (question 1 a), fences sit under list items in the issues.

## Deliverable 1
- done: `fence`/`fenced_lines` read the QUA-12 fences; `section` finds and ends its heading outside them and returns
  `None` without one, so `deliverables` falls back to `# Tracking` only when no `## Plan` stands outside a fence.
- red: QUA-12: `delta.deliverables: expected "2", got Some("4")` (the `## Plan` quoted in the Goal fence was read).
- Code review: no findings (low).
- not verified: none.
- decisions needed: none.

## Deliverable 2
- done: `fence`/`fenced` moved to `crates/harness/src/tools/mod.rs` and used by `check_specs` too; a backtick line
  with more backticks after its run no longer opens a fence (QUA-12 amended); the fixture adds inline code at a line
  start and `red:` items inside and outside a fence.
- red: QUA-11, QUA-12: `delta.deliverables: expected "2", got Some("0")`, `delta.traced_without_red: expected "1",
  got Some("2")` (the inline code opened a fence to the end of the file).
- High review (base..HEAD): 7 findings. Fixed: inline code opening a fence, the duplicated fence reader, the
  untested QUA-11 path, the `if`/`else` of `fence`. Not fixed: the double scan of `deliverables` (cost negligible
  at issue sizes). Moved to `issues/done/tell_unread_issue_sections_from_empty_ones.md`: the fence-blind QUA-2 start and
  the zeros printed for a section not found (both outside QUA-3, QUA-6, QUA-11, QUA-12 as amended).
- Code review: no findings (low).
- not verified: none.
- decisions needed: none.
