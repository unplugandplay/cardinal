# Keep the user and architecture docs in step with the specs

# Goal

In the consumers, the specs grow fast while the user docs and the architecture document barely move. The docs step
of `issue-workflow` does run, but it only patches a line or two per deliverable: nothing routes a change of behaviour
to the page that describes the capability, nothing reads the docs over a whole issue, and nothing measures it.
Expected outcome: a change of behaviour updates its spec **and** the doc page and architecture document that describe
it, and `delivered-quality` shows when it does not.

# Findings (2026-10-30, two consumer projects, since their specs started on 2026-10-24)

* The docs step runs: 83 % and 85 % of the `feat`/`fix` commits touch `docs/` (outside ADRs); only 3 commits in each
  project change a spec and no doc, README or `CLAUDE.md`.
* `skill:update-docs` is invoked in 24 of 46 and 39 of 67 deliverables (counted as `commit-message` calls), about
  half; the other deliverables edit the docs inline or not at all.
* Specs: +756 / −0 and +918 / −0 lines. Docs outside ADRs: +151 / −97 in one project, about +175 net in the other
  (leaving out a rename of the doc folder).
* Architecture document: 12 changed lines over 123 `feat`/`fix` commits in one project, 82 in the other.
* The `docs` row of the 3.6 summaries mostly names real pages, but as surgical edits.
* The generic row of `update-docs` for "Behaviour of a capability that has a spec" sends to `specs/<capability>.md`
  only. In one project the `update-docs-map.md` has no other row for a change of behaviour, so a behaviour change
  goes to the spec alone.
* `issue-workflow` step 3.4 touches the `architecture` document only "when the flow changed", and only its diagrams.
  The final `high` review reads `base..HEAD` for code; no docs pass covers the whole issue.
* `delivered-quality` excludes `docs/` from the files of an issue (QUA-4) and has no docs column.

# Approach

Candidate solutions, can be combined:

1. `update-docs`, generic row: a behaviour change of a capability with a spec also updates the `docs/` page that
   describes the capability, and the `architecture` document of `.claude/cardinal.toml` when a boundary, a flow or
   a component changes (not only its diagrams).
2. `issue-workflow`, after the last deliverable: run `skill:update-docs` once on `base..HEAD`, next to the `high`
   review. It reads the touched user pages and the architecture document whole, and consolidates them instead of
   patching them. Its edits go in the fix deliverable of that review.
3. `delivered-quality`: a column counting the commits of an issue that change a file under `specs/` and no
   documentation file (`docs/` outside `docs/adr/`, `README.md`, `CLAUDE.md`), so the commits of 1 and 2 can carry
   `Expected-effect: <column> down` and `skill:evaluate-harness` can judge them.

Measure first for 3: check that the column separates the two consumers' history before relying on it.

# Specs

* `specs/delivered-quality.md`: a new requirement (`QUA-14`) for the column of approach 3, and QUA-1 amended for its
  place in the header.

# Resources

* `files/.claude/skills/update-docs/SKILL.md` (step 2 table, step 4)
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 3.4 and 3.5, the final `high` review)
* `specs/delivered-quality.md`, `specs/cardinal-verdicts.md`
* `docs/architecture.md` (the readers of `cardinal-report`)

# Hints

* Approaches 1 and 2 change `files/`, so every consumer: run `just sync` and give the commits an `Expected-effect`.
* Watch the fixed context: approach 2 adds a step to the longest skill; keep it to one line in `SKILL.md` and put
  the detail in `details.md` if needed.
* A pure refactor or a test seam still needs no doc (the "n/a" rows met in the summaries were justified).

# Tracking

Plan: validated

## Understanding
Make a behaviour change update its user doc and architecture document, not only its spec, and measure it. The
metric is the ratio of doc lines to spec lines changed per issue, not "commits with a spec and no doc": that one
is already near 0 (3 commits per consumer) and would not move. Out of scope: rewriting the consumers' docs.

## Plan
1. [x] Measure: `spec_lines` and `doc_lines` in `delivered-quality`, `doc_lines_per_spec_line` in the quality
   table of `cardinal-report` and among the metrics of `Expected-effect`; verified by tests of the binary on
   git fixtures; makes true QUA-14, QUA-1, REP-6, VRD-1.
2. [x] Route the docs: the generic behaviour row of `update-docs` names the `docs/` page of the capability and the
   `architecture` document; `issue-workflow` runs `update-docs` once on `base..HEAD` after the last deliverable,
   reading those pages whole; `Expected-effect: doc_lines_per_spec_line up`; verified by `just check-cardinal`
   and `just check`.

3. [x] Docs pass and fixes of the `high` review of the issue.

## Decisions
- Mode: autonomous
- Base commit: 5295d9d
- Root docs counted in `doc_lines`: the `root_docs` of `.claude/cardinal.toml` (1 a).
- Metric: the ratio of doc lines to spec lines, not the commits with a spec and no doc (2 a).

## Deliverable 1

- done: `delivered-quality` prints `spec_lines` and `doc_lines` (root docs from `root_docs`), `cardinal-report`
  their ratio `doc_lines_per_spec_line`, accepted as an `Expected-effect` metric; the three specs back to `accepted`.
- | check | result |
  |---|---|
  | tests | pass (`just check`) |
  | lint | pass |
  | specs | pass, QUA-14 traced |
  | docs | `docs/architecture.md` readers table, `root_docs` comment of the seed and of `.claude/cardinal.toml` |
  | code review | see below |
- red: QUA-14, REP-6, VRD-1: `iota.spec_lines: expected "40", got Some("")`; `quality-a.tsv has other columns than
  expected`; `commit_check_refuses_an_expected_effect_it_cannot_read` failed on `doc_lines_per_spec_line up`.
- Code review: no findings
- Measured on the two consumers, per revision, `doc_lines / spec_lines`: 0.36, 2.45, 2.06, 1.24 in one, 0.70, 0.29,
  0.39, 0.19 in the other: the column separates the revisions.
- not verified: the zero divisor of `doc_lines_per_spec_line` is covered by the shared `ratio`, not by its own check.
- decisions needed: none.

## Deliverable 2

- done: the generic behaviour row of `update-docs` names the user page and the `architecture` document;
  `update-docs` reads a commit range and rewrites a page touched over it whole; `issue-workflow` runs it on
  `base..HEAD` before the `high` review; `docs/workflow.md` diagram and note; synced.
- | check | result |
  |---|---|
  | tests | pass (`just check`) |
  | lint | pass |
  | specs | n/a (skill text, no capability of the binary) |
  | docs | `docs/workflow.md` |
  | code review | see below |
- red: none (skill text, verified by `just check-cardinal`).
- Code review: no findings
- not verified: the effect on the consumers, until issues run with this revision (`doc_lines_per_spec_line`).
- decisions needed: none.

## Deliverable 3 (docs pass and high review of the issue)

- done: the docs pass over `5295d9d..HEAD` found `docs/usage.md` (Tools) silent on the new columns: fixed; the
  fixes of the `high` review below.
- Code review (high, `5295d9d..HEAD`): 9 findings: fixed 1 (a root doc that is no markdown counts), 2 (the
  `roadmap` files never count), 3 (the `architecture` file counts), 4 (`--numstat -z`, no hand parser of renames
  and quoted names), 6 (a check for a zero `spec_lines`), 7 (`cardinal.toml` read once; the second `git show`
  stays: `traced` filters its diff on `spec:`), 8 (the grep of `update-docs` step 3 reads the `root_docs` and the
  `architecture`), 9 (the commit range comes first in `update-docs` step 1); not fixed 5 (the `pending` line is
  this deliverable's, rewritten before its commit). QUA-14 amended for 1 to 3.
- | check | result |
  |---|---|
  | tests | pass (`just check`) |
  | lint | pass |
  | specs | pass, QUA-14 amended and traced |
  | docs | `docs/usage.md`, `update-docs` steps 1 and 3 |
  | code review | see below |
- red: QUA-14: `iota.doc_lines: expected "19", got Some("15")` (and 16 got 15: the quoted name) before `DocFiles` and `-z`.
- Code review (low, the fixes): no findings
- not verified: none.
- decisions needed: none.
