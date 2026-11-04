---
rank: 62
why: 'cheap, and each close of an issue named in a roadmap pays a review that cannot find anything; before the chained issues, which close more often'
---
# Let the close of an issue commit without a review

# Goal
The native `pre-commit` hook (`review-gate native`, REV-9) lets a commit through without a review only when its
diff touches `issues/` alone. `close-issue` also rewrites the references to the closed issue in the other documents
(CLS-1, CLS-2), the roadmap first, since every issue of a milestone is linked there. So the commit that closes such an
issue is refused until a `/code-review low` runs on a diff of rewritten links, which it cannot find fault with. The
expected outcome: the commit of a close goes through when its changes outside `issues/` are only the rewrites
`close-issue` made, and is still refused as soon as anything else is in it.

# Findings
* Met on 2026-10-30 at the close of `issues/done/count_the_advisor_calls_of_each_session.md`: the staged diff was the
  `git mv` of the issue, the rewritten references in two other issues and one line of `ROADMAP.md`
  (`issues/<stem>.md` to `issues/done/<stem>.md`); the hook refused it, and the review it required found nothing.
* `records_issues_only` of `crates/cardinal/src/tools/review_gate.rs` checks that every staged path starts with
  `issues/`; nothing else is exempt.
* Setting a milestone `done` in the same commit (the Rules of `issue-workflow`) is an edit of the roadmap by the agent,
  not a rewrite by the tool.

# Approach
1. Exempt a commit whose staged diff, outside `issues/`, holds only changed lines that become equal once each
   `issues/done/<stem>.md` is read back as `issues/<stem>.md`, for a `<stem>` whose move to `issues/done/` is staged
   in the same commit (the relative links of CLS-2 included).
2. Or have `close-issue` record the files and lines it rewrote (e.g. a file under `.git/`), and let the gate accept
   exactly those.
3. Decide whether a milestone set `done` in the same commit stays gated (a review of one word) or goes to a commit of
   its own.

# Specs
* `specs/review-gate.md`: amends REV-9.
* `specs/close-issue.md`: only if option 2 makes `close-issue` record its rewrites.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/review_gate.rs` (`records_issues_only`), `crates/cardinal/src/tools/close_issue.rs`.
* `specs/review-gate.md` (REV-9), `specs/close-issue.md` (CLS-1, CLS-2).
* `files/.claude/skills/issue-workflow/SKILL.md` (Rules: the close), `files/.claude/skills/issue-workflow/details.md`
  ("Review waiver").

# Hints
* The exemption must never let through a change the tool did not make: a link rewrite next to an edited sentence on
  the same line is a review case.

# Tracking

Plan: validated

## Understanding
The native hook lets through a commit whose changes outside `issues/` are exactly what `close-issue` would rewrite in
the `HEAD` content for the issues whose move to `issues/done/` is staged. Out of scope: the milestone set `done`
(stays gated). Means: approach 1, made strict: the gate calls the rewrite of `close-issue` on the `HEAD` blob and
requires the index blob to be equal (no normalisation, no record under `.git/`).

## Plan
1. [x] Close exemption: extract the per-file rewrite of `close_issue::relink` into a pure function; `review_gate`
   finds the staged moves (`issues/<stem>.md` deleted, `issues/done/<stem>.md` added) and accepts the index when every
   other path outside `issues/` is modified and equals the rewrite of its `HEAD` content; the docs of the waiver of
   `issue-workflow` say so; verified by tests of `native` in a temporary repository (close passes; milestone edit,
   edited word on a rewritten line, added file, no staged move refused); makes true REV-20.
2. [x] Fixes of the `high` review: the gate reads `git diff --raw` (modes, object ids) and refuses a submodule, a
   changed mode, a failed read and a file `close-issue` never rewrites (`close_issue::rewrites`); verified by three
   more cases of `close_checks`; makes true REV-20.

## Decisions
- Mode: autonomous
- Base commit: c9bc30e
- Milestone set `done` in the close commit: stays in that commit and stays gated (1 a).
- Detection: strict equality with the rewrite of `close-issue` on the `HEAD` content (2 a).

## Deliverable 1

- done: `native` lets a close through when its index, outside `issues/`, equals the rewrite `close_issue::rewritten`
  (extracted from `relink`) gives of the `HEAD` content for the staged moves; docs: `issue-workflow` step 3.5,
  `docs/architecture.md`, the module doc of `review_gate.rs`.

| check | result |
|---|---|
| tests | `just check` green (`close_checks`: 7 cases) |
| lint | clippy -D warnings clean |
| specs | REV-20 traced |
| docs | skill, architecture, module doc |
| code review | see below |

- red: REV-20: `FAIL - native lets through the close of an issue with the references close-issue rewrote`
- Code review: no findings (`low`; the test file was out of its scope).
- not verified: a real close through the installed hook (needs `cargo install` after the commit).
- decisions needed: none.

## Deliverable 2

- done: the fixes of the `high` review of `c9bc30e..ece8b3f` (8 findings).

| check | result |
|---|---|
| tests | `just check` green (`close_checks`: 10 cases) |
| lint | clippy -D warnings clean |
| specs | REV-20 reworded (regular files, mode kept, files `close-issue` rewrites) |
| docs | spec, doc comments |
| code review | see below |

- red: REV-20: `FAIL - native refuses a close with a file whose mode changed`, `... a submodule moved`, `... a reference
  rewritten in a file close-issue never rewrites`
- High review: 8 findings. Fixed: failed read passing as empty (submodule, non-UTF-8 path), mode change, files
  `close-issue` never rewrites, `staged` computed eagerly, two `git show` per file (now `cat-file` on the ids of
  `--raw`), test tempdir leaked on panic. Not fixed: a nested `issues/` directory (REV-9 and REV-20 cover the root
  `issues/` only, as the existing exemption); `on_index` kept beside `git` since it must tell a failed read from an
  empty one (said in its doc).
- Code review: no findings (`low` on the fixes).
- not verified: none beyond deliverable 1.
- decisions needed: none.
