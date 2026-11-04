---
priority: low
depends_on: [tell_the_needed_oversized_results_from_the_waste]
why: 'the session''s own oversized results cost 3.7 KB per session, many of them whole reads a range would bound'
---
# Hint before a whole read of a large file

# Goal
A third of the session's own tool results over 25,000 bytes are a `Read`, among them the `tool-results/`
file where Claude Code saved a result it had cut. A line in the fixed context would cost every session more than
the waste it saves; a hook costs nothing until it fires. The outcome: before a whole `Read` of a large file the agent
is told to read a range, and `oversized-result` falls.

# Findings
* Measured 2026-11-03 (`issues/done/tell_the_needed_oversized_results_from_the_waste.md`, deliverable 3): 36
  `oversized-result` breaches in 30 of 290 sessions, 3,708 bytes per session; over every session of the machine,
  69 such results: `Read` 24 (10 whole code files, 8 whole `tool-results/` files), `cat` 22, `cd …;` commands 8,
  `WebFetch` 7, others 8.

# Approach
1. A `PreToolUse` hook on `Read` with no `offset` nor `limit` of a file over 25,000 bytes: a hint (or an `ask`)
   naming the size and the range options, never a refusal.
2. Measure whether a hint on a `cat` of a large file through `Bash` is worth its parsing (`shell.rs`).
3. Declare the hook in `entries/.claude/settings.json` and give it a feature to turn it off.

# Specs
* A new spec of the hook.

# Milestone
Lean agent loop

# Resources
* `entries/.claude/settings.json`, `features.toml`, `crates/cardinal/src/tools/language_hint.rs` (a hook that hints).
* `specs/rule-breaches.md` BRE-19.

# Hints
* A file read whole may be needed (an edit of most of it): the hint must not block.
* Verdict: `Expected-effect: bytes_per_session:oversized-result down`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-03

## Understanding
Before a whole `Read` of a file over 25,000 bytes, a `PreToolUse` hook sends the agent back once (a `deny` naming the
size, `offset`/`limit`, `Grep`); the same call repeated goes through. A hint by `additionalContext` alone would not
spare the read it fires on (the tool still runs). The `cat` through `Bash` is measured, not hooked, here.

## Plan
1. [x] Hook `read-hint`: the tool, its declaration in `entries/.claude/settings.json`, the feature `read-hint`, the docs; verified by `tests/read_hint.rs` and the features test; makes true RDH-1 to RDH-6. (autonomous)
2. [x] Measure the `cat` of a large file: of the `cat` results over 25,000 bytes, how many are a lone `cat <file>` that `shell.rs` reads; the verdict written in the issue, a follow-up issue if worth it; verified by the numbers in the issue. (autonomous)
3. [x] Fixes of the `high` review and the docs of the whole issue: the estimate of the result as Claude Code reads (RDH-7), the 256 KB cap, unreadable files and notebooks pass, a private record directory; verified by `tests/read_hint.rs`; makes true RDH-1, RDH-2, RDH-5, RDH-7. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 36988d8f0c78800637aaa41311f7551982db242b
- Hook: a `deny` once per file and session, the repeat goes through (1 a); the record a file per `session_id` under `TMPDIR` (2 a); `cat` measured only (3 a).

## Deliverable 1
done: hook `read-hint` (`PreToolUse` `Read`): a whole read of a file over 25,000 bytes denied once per file and
session with its size, `offset`/`limit` and `Grep`; the repeat goes through. Declared in `entries/`, feature
`read-hint`, spec RDH accepted, docs (`architecture.md`, `gates.md`, `usage.md`, `README.md`).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (RDH accepted) |
| docs | `docs/architecture.md`, `docs/gates.md`, `docs/usage.md`, `README.md` |
| code review | see below |

red: RDH-1, RDH-2, RDH-3, RDH-4, RDH-5, RDH-6: `cardinal: unknown tool `read-hint`` (exit 3, every test failed)

Code review: no findings

not verified: the hook in a live session (the pinned `cardinal` predates it until this commit is pinned).

## Deliverable 2
Measured 2026-11-03 on every transcript of this machine: the session's own `Bash` results over 25,000 bytes whose
command holds `cat`.

| shape of the command | results |
|---|---|
| a lone `cat <file>` (after a `cd`, no pipe nor redirection) | 3 |
| a lone `cat` of several files | 3 |
| `cat` in a list or a pipe (`cat a; cat b`, a `for` loop, `sed …; cat …`) | 20 |
| total | 26 |

Verdict: not worth a hook. The shape a hook could judge from the file size alone (a lone `cat` of one file) is 3 of
26; the others would need the sizes of every file of a list, or the output of a loop, before the call. No follow-up
issue. To watch: if `read-hint` turns whole reads into `cat` of the same files, rule `oversized-result` shows it, and
the `cat` hint is worth measuring again.

not verified: the shapes are read by a split on `;`, `&&`, `||` and newlines, not by `shell.rs` (a measure, not a gate).

## Deliverable 3
done: the fixes of the `high` review of the whole issue (10 findings).

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (RDH-1, RDH-2, RDH-5 amended, RDH-7 added) |
| docs | `docs/gates.md`, `docs/architecture.md`, `docs/usage.md`, `README.md` |
| code review | see below |

red: RDH-1, RDH-2, RDH-5, RDH-7: `reason names 30000 bytes, not the estimate 32100`; a minified file, a 400 KB log, a notebook and an unreadable file denied; no `cardinal-read-hint-tester` directory

High review: 10 findings:
1. An unreadable file was denied: fixed, the hook opens the file (RDH-2).
2. Over 256 KB Claude Code refuses a whole read itself, so the promised repeat failed: fixed, such files pass (RDH-1).
3. A fixed directory in the shared `/tmp`: fixed, `XDG_RUNTIME_DIR` first, else a per-user directory, mode 0700, unused when it lets others in (RDH-5).
4. The file size overstated a minified file or one over 2,000 lines: fixed, the estimate of RDH-7.
5. Notebooks are rendered: fixed, `ipynb` passes (RDH-2).
6. The threshold copied BRE-19's: fixed, `OVERSIZED` of `rule_breaches.rs` shared.
7. Two spellings of one path are two files: not fixed, a non-goal (the hint asks to repeat the same call).
8. Two session ids differing in a non-alphanumeric character share a record: not fixed, Claude Code's ids are UUIDs.
9. The RDH-6 test duplicated the features test: fixed, removed (the features test traces RDH-6).
10. The records are never removed: not fixed, a non-goal (a runtime or temporary directory the system clears).

Code review: no findings

not verified: the hook in a live session (the pinned `cardinal` predates it); the 256 KB cap is Claude Code's default
`maxSizeBytes`, read in its binary (2.1.312), not its exact value.
