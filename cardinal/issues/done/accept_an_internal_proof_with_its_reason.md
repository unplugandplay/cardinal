# Accept an internal proof with its reason

# Goal
A requirement must stay writable and provable when a test through the binary is impossible, slow or fragile. Today
`specs/README.md` says a test under `src` "does not count as the proof" and `check-specs` warns of each such
requirement (TRC-1) with no way to say the choice is deliberate, so the warning pushes the agent toward long and
fragile end-to-end tests where a test at the boundary of the module was enough. A boundary test through the binary
must be an improvement made when it is cheap and stable, not a rule.

# Findings
* Raised by the user at the end of `prove_the_run_and_gate_specs_through_the_binary` (2026-10-27): the remaining
  warnings (SYN-18 to SYN-24, REV-19, BIN-9) read as work to drive to zero.
* The internal gate tests call `sync()` and `check_with()`, the public functions of the library: a boundary of the
  module, not its internals.

# Approach
1. A citation may carry its reason, `// spec: BIN-4 (internal: <reason>)`: `check-specs` reads the ids as today and
   leaves out of the TRC-1 warning a requirement whose internal citations all give a reason; an empty reason is a
   problem.
2. Reword `specs/README.md`: a test through the binary is preferred when it is cheap and stable; otherwise an internal
   test is the proof, with its reason on the citation.
3. Say the same where the agent decides the test (`skill:write-spec`, the "test first" step of
   `skill:issue-workflow`), in `files/`, then `just sync`.

# Specs
* `specs/spec-tracing.md` (TRC): a new requirement for the reasoned internal citation; TRC-1 amended.

# Resources
* `specs/README.md`, the TRC spec, `crates/harness/src/tools/check_specs.rs` (`boundary_warnings`, the citation
  parser), `crates/harness/tests/check_specs.rs`, `files/.claude/skills/write-spec/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`, `harness run delivered-quality` (reads the citations too).

# Hints
* The citation parser is shared with `delivered-quality`: the reason must not change the ids it reads.

# Tracking

Plan: validated

## Understanding
A citation under `src` may give its reason, `// spec: ID (internal: <reason>)`; `check-specs` then leaves the
requirement out of the TRC-1 warning, fails on a blank reason, and the reason never changes the ids read (by
`check-specs` nor `delivered-quality`). The docs and skills present the boundary test as preferred, not required.

## Plan
1. [x] Reasoned citation: one shared citation parser (ids + reason) in `check_specs.rs`, used by `delivered-quality`;
   boundary warning skips reasoned requirements and names the syntax; blank reason is a problem; verified by
   `tests/check_specs.rs` and a `delivered-quality` test; makes true TRC-1, TRC-6, TRC-7, TRC-8 (and the new QUA-11
   scenario). ~250 lines.
2. [x] Wording: `specs/README.md`, `files/.claude/skills/write-spec/SKILL.md`, the "test first" step of
   `files/.claude/skills/issue-workflow/SKILL.md`, then `just sync`; verified by `just check-harness`. ~40 lines.
3. [x] Reasons on this repo's internal-only citations (SYN-18 to SYN-24, REV-19, BIN-9), so `just check-specs` warns
   of nothing; verified by `just check-specs`. ~20 lines.

4. [x] Fixes of the `high` review: a reason that is misplaced, unclosed or follows no id is a problem (TRC-7
   amended); verified by `tests/check_specs.rs`.

## Decisions
- Mode: autonomous
- Base commit: f34ae899b66e605f52aace8dc3303ddc004b6567
- TRC goes `draft` while TRC-6 to TRC-8 are untraced, back to `accepted` in deliverable 1.
- The boundary warning names the reason syntax.
- Deliverable 3 gives reasons to this repo's internal-only citations, each from what its test does.
- Deliverable 3 reverted after the `high` review (user, 2026-10-28): its reasons said a binary test would be cheap,
  against the rule of deliverable 2, and silenced the warnings `prove_the_gate_specs_through_the_binary` works from;
  that issue decides proof by proof.

## Deliverable 1
- done: `citation()` in `check_specs.rs` splits ids and reason, shared with `delivered-quality`; the boundary warning
  skips reasoned requirements and names the syntax; a blank reason is a problem; TRC back to `accepted`.

| check | result |
|---|---|
| tests | ok |
| lint | ok |
| specs | ok (9 warnings left, deliverable 3) |
| docs | module doc; README wording in deliverable 2 |
| code review | low: no findings |

- red: TRC-6, TRC-7, TRC-8: `crates/a/src/t.rs: cites \`(internal:\`, which no spec defines`; QUA-11: `traced` 4, not 3
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: `specs/README.md`, `skill:write-spec` (step "Trace") and `skill:issue-workflow` (step "Test first") say a
  binary test is preferred when cheap and stable, else an internal one with its reason; synced.

| check | result |
|---|---|
| tests | ok |
| lint | ok |
| specs | ok |
| docs | no other doc states the old rule |
| code review | low: 1 finding, fixed |

- red: none
- Code review: 1 finding: "when one of those citations gives no reason" read either way; reworded "as long as one
  of those citations at least gives no reason".
- not verified: none
- decisions needed: none

## Deliverable 3
- done: the 20 citations of `crates/harness/src/tests/install_gate.rs` give their reason (public `sync()` and
  `check_with()`, the hook run as git runs it, a property of the shipped entries file); `just check-specs` warns of
  nothing.

| check | result |
|---|---|
| tests | ok |
| lint | ok |
| specs | ok, no warning |
| docs | none needed |
| code review | low: no findings |

- red: none
- Code review: no findings (the test comments are not read at `low`)
- not verified: none
- decisions needed: none

## Deliverable 4 (fixes of the `high` review)
- done: TRC-7 also flags `(internal: ...)` with ids after it, no closing `)` or no id before it; doc of
  `citations()` and `citation()` brought up to date; `reasoned` computed once per line.

| check | result |
|---|---|
| tests | ok |
| lint | ok |
| specs | ok, no warning |
| docs | doc comments |
| code review | low: 1 finding, fixed |

- red: TRC-7: the 4 malformed reasons passed the checker
- Code review: 1 finding: `(internal: slow) RER-2)` passed (a second `)` ended the line); fixed, a reason's own
  parentheses must balance.
- High review findings: 8. Fixed: ids after the reason dropped, reason with no id, stale doc, recomputed
  `reasoned`. Not fixed: near-miss spellings (`(Internal :`) already fail as unknown ids naming the text; a reason
  outside `src` is harmless; closing the issue is the user's call; the reasons of deliverable 3, a decision for
  the user (they say a binary test would be cheap, and they silence the warnings `prove_the_gate_specs_through_the_binary`
  works from).
- not verified: none
- decisions needed: none (deliverable 3 reverted, see below)
