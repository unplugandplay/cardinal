# Shrink the review gate to the native hook

# Goal
The review gate costs more than it protects. `require_code_review.rs` parses shell text to find the repository a
commit lands in and to spot the waiver being written, and every review finds a new bypass that becomes a new
requirement. ADR 0004 already made the native `pre-commit` hook the gate the agent cannot route around by naming the
repository differently. Record that the agent is cooperative but fallible, not adversarial, and cut the Claude hook
down to what the native hook cannot do, so the gate stops growing.

# Findings
* `files/.claude/hooks/require_code_review.rs` is 878 lines; its test file is about 55 KB. Together they hold more
  than a third of the bytes under `files/`.
* 18 commits touched the hook, 16 on 2026-10-25 and 2 on 2026-10-26; about 10 of the 30 issues in `issues/done/` are
  about it.
* `specs/review-gate.md` has 18 requirements; REV-1 to REV-8 and REV-16 read the repository from the command text
  (`-C`, `--git-dir`, `GIT_DIR`, `export`, fish `set -x`, `cd`), REV-13, REV-14 and REV-18 read the text for a write
  of the waiver or a run of `just waive-review`.
* ADR 0004, Consequences: the text rules "are no longer what the gate rests on: the native hook decides when they
  cannot read the command".
* `issues/done/make_the_review_waiver_usable.md`: one review returned 10 findings, all about the shell parser, all
  fixed in the same deliverable.

# Approach
1. Write an ADR (superseding the relevant part of 0004) stating the threat model: the gates catch the agent's
   mistakes, not deliberate bypasses; a finding that is a bypass by a cooperative agent's unlikely command is a
   non-goal, not a requirement.
2. Keep in the Claude hook only `record` and, in `check`, the refusal of `--no-verify`, `-n` and `core.hooksPath`
   (REV-12); the native hook (`native`, REV-9 to REV-11, REV-15) judges the repository.
3. Replace the text detection of the waiver and of its recipe by a `permissions.deny` rule in the seeded
   `.claude/settings.json` (for example `Bash(just waive-review*)`), plus the existing instruction in the skill.
4. Withdraw the requirements that no longer hold (`withdrawn:`, numbers kept) and delete their tests and code.
5. Measure before and after: lines of the hook and its tests, latency of `check` on a plain `git status`.

# Specs
* `specs/review-gate.md`: withdraws REV-1 to REV-8, REV-13, REV-14, REV-16 and REV-18; may add a requirement for the
  deny rule of the seeded settings.

# Resources
* `docs/adr/0004-review-gate-moves-from-the-command-to-the-repository.md`
* `specs/review-gate.md`, `files/.claude/hooks/require_code_review.rs`, `files/.claude/hooks/test_require_code_review.rs`
* `seed/.claude/settings.json`, `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)
* `crates/harness/src/gate.rs` (installs the native hook)

# Hints
* A `permissions.deny` rule in a project's settings is applied by Claude Code, not by a script: check the exact
  pattern syntax in the Claude Code docs before relying on it.
* `harness sync` does not overwrite a consumer's `settings.json` (it is seeded): say how existing consumers get the
  deny rule.
* The hook refuses Bash commands that write the waiver file or run its recipe while it still exists: edit with the
  file tools.

# Tracking

1. [x] Deny rules in the seeded settings (REV-19), skill step 4.5 updated.
2. [x] Cut the Claude hook to `record` and REV-12; withdraw REV-1 to REV-8, REV-13, REV-14, REV-16, REV-18; rewrite REV-9; ADR 0005 accepted; measure.
3. [x] `harness check` warns on a missing deny rule (SYN-24); specs accepted then implemented.

## Decisions

- Mode: autonomous.
- Existing consumers learn of the deny rules from a non-blocking warning of `harness check` (SYN-24).
- A repository without the native hook has no review gate: accepted, `harness check` already reports it (SYN-23).
- The user runs `just waive-review` from their own terminal; whether a deny rule applies to a `!` command was not checked.
- SYN-24 goes into `specs/sync.md` with its test in deliverable 3, so that spec stays `implemented` (a `draft` spec is not enforced).
- Baseline: hook 878 lines, its tests 999 lines (55 KB), `files/` 272 KB; `check` on `git status` about 3 ms (cached binary).

## Deliverable 1

done: deny rules in `seed/.claude/settings.json` and in this repository's settings, `DENY_RULES` in `gate.rs` checked by
a REV-19 test, REV spec amended (withdrawals, REV-9 rewritten, REV-19), ADR 0005 accepted and ADR 0004 marked as partly
superseded, skill step 4.5 and `docs/architecture.md` describe the deny rules.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| lint | pass |
| specs | pass |
| docs | updated |
| code review | 9 findings |

Code review:
- `specs/sync.md` set back to `draft` turns its enforcement off: fixed, SYN-24 moves to deliverable 3 with its test.
- architecture, withdrawn requirements and REV-9 describe the hook before deliverable 2 removes its text rules: not
  fixed, deliverable 2 follows in the same run and makes them true.
- this repository's own settings are not checked: deliverable 3 (`harness check` warning) covers it.
- `serde_json` as a runtime dependency: fixed, moved to dev-dependencies until deliverable 3.
- `DENY_RULES` with `expect(dead_code)`: kept, deliverable 3 reads it and drops the attribute.
- "step 4.5" in the tracking: no change, the skill names it so (step 4, item 5).
- ADR 0004 not pointing to 0005: fixed; ADR 0005 accepted (threat model validated at step 1).

Second review (the ADRs were new to the diff): `core.hooksPath` is reported by `sync`, not `check`: fixed in the
spec and ADR 0005. Architecture paragraph on the target repository, ADR 0004 body, stale `// spec:` of withdrawn ids,
REV-9 parity with `check`: deliverable 2. ADR 0004 body kept as written (an ADR is a record; its status points to
0005). A `touch` of the waiver is not refused once deliverable 2 lands: accepted non-goal (ADR 0005).

not verified: that Claude Code refuses `just waive-review` with the new rules (needs a live session).
decisions needed: none.

## Deliverable 2

done: `check` keeps only the refusal of `--no-verify`, `-n` and `core.hooksPath` (REV-12) and no longer judges the
review; the repository parser, the `cd` reader and the waiver detection are gone with their tests; the review
semantics (marker, fixes, `issues/`, waiver, first commit, linked worktree, recipe) are now tested through `native`;
`docs/architecture.md`, skill step 4.5 and the comment of `block_ai_attribution.rs` follow.

Measure (before -> after): hook 878 -> 385 lines (46.6 -> 19.2 KB), its tests 999 -> 560 lines (55.5 -> 28.2 KB),
`files/` 272 -> 218 KB; `check` on `git status` about 3 ms before and after (cached binary): the gain is in code and
review cost, not latency.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| lint | pass |
| specs | pass |
| docs | updated |
| code review | 8 findings |

Code review:
- a quoted `--work-tree="my dir"`, a segment where `git` is first an argument, a continued line: fixed (regressions of
  the cut or cheap to cover), with three REV-12 cases.
- `GIT_VALUE_OPTIONS` said to match `block_ai_attribution.rs` but did not: fixed, same 7 options now.
- REV-12 test built two repositories for a text-only check: fixed, a plain directory.
- `native` introduced twice in `docs/architecture.md`: fixed.
- `--no-verify` after a `$(…)` message or a quoted `|`: not fixed, the line was already split there before the cut;
  a variant the hook does not read is a non-goal (ADR 0005).
- no gate without the native hook (`core.hooksPath`, not synced): not fixed, decided at step 1 (`sync` and `check`
  report it).

not verified: none.
decisions needed: none.

## Deliverable 3

done: `harness check` prints `warning: .claude/settings.json lacks the deny rule <rule>` for each rule of `DENY_RULES`
missing from a project that manages the review hook, without failing (SYN-24, `warnings` in `lib.rs`,
`missing_deny_rules` in `gate.rs`); `serde_json` is a runtime dependency again; `specs/sync.md` gains SYN-24 and stays
`implemented`, as does `specs/review-gate.md`; README and `docs/architecture.md` describe the warning.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| lint | pass |
| specs | pass |
| docs | updated |
| code review | 9 findings |

Code review:
- an unreadable settings file was silent, an invalid one reported every rule missing: fixed, one warning saying why
  (with a SYN-24 scenario and a test).
- the lock read twice and the review hook test duplicated, a lost report when the second read failed, a dead
  "never synced" branch: fixed, `check` returns `Checked { problems, warnings }` from one read of the lock.
- warnings mixed with problems on stdout: fixed, warnings go to stderr.
- the deliverable 3 summary pointed to a missing review section: fixed, this one.
- an equivalent rule written otherwise (`Bash(just waive-review:*)`) still warns: not fixed, the warning names the exact
  rule to add, and matching Claude Code's rule grammar is not worth its code.

Second review (`tests.rs` was new to the diff): the `expected` flag of `missing_deny_rules` and the wording of the
hooks paragraph of `docs/architecture.md`: fixed. Not fixed: no settings file gives no warning (`sync` seeds it again
when missing); rules kept in the user's or the local settings, a `deny` of the wrong JSON type, an equivalent rule
written otherwise and the warning under CI are edge cases a warning does not need to cover; `serde_json` at runtime is
the price of reading the settings.

not verified: none.
decisions needed: none.
