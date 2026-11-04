# Measure the delivered quality from the git history

# Goal
The cost of a session says nothing of what it delivered. Measure the quality of the delivered work by what
happened to it afterwards: fixes, reverts, review findings, failed checks, rework asked by the user.

# Findings
* 2026-10-26, `git log` of this repository: `3b72198 feat(files): prepare an issue in its files...` is followed by
  `c56fa40 fix(files): close the gaps of the single issue validation stop` on the same subject.
* Issue files under `issues/done/` record per deliverable the code review level and what it found, and the checks
  run (`# Tracking`, "Summary of deliverable n").
* The commit scopes are constrained (`commit_scopes` of `.claude/harness.toml`, `just check-commit`), so a fix and
  the feature it fixes can be matched by scope and files touched.

# Approach
1. From git: per issue, the `fix` and `revert` commits touching the files of its `feat` commits within a window
   (to pick), the number of commits per deliverable.
2. From the issue files: the review findings and fixes recorded per deliverable.
3. From the transcripts (with `extract_session_metrics_from_the_transcripts`): failed `just check` runs before a
   commit, and the user's answers at the validation stops that ask for rework.
4. Report per issue and per harness revision; leave the interpretation to the report of
   `report_the_harness_evolution_by_revision`.

# Specs
* `specs/evaluation.md`, new requirements for the quality signals.

# Resources
* `AGENTS.md` (commit format), `issues/done/`, `issues/queue.md`
* `files/.claude/skills/issue-workflow/SKILL.md` (what `# Tracking` records)
* `issues/done/extract_session_metrics_from_the_transcripts.md`

# Hints
* A fix after a feature may come from a new need, not a defect: report it as a signal, not a verdict.
* Consumers have their own scopes and history; read their git log, never copy it into this repository.

# Tracking

Plan: validated

Status: done (2026-10-26)

## Understanding
One TSV row of quality signals per done issue of a repository: fixes and reverts of its files after the close,
fixes inside it, review findings from its file, failed checks and typed messages from its sessions. Out of scope:
judging an issue, the report (`report_the_harness_evolution_by_revision`). Means: a managed script
`delivered_quality.rs` next to `transcripts.rs` (ADR 0006), with its own spec `specs/delivered-quality.md` (QUA)
rather than new EVA requirements, as `rule-breaches.md` did.

## Plan
1. [x] Git signals: `delivered_quality.rs` with the issue commits, `deliverables`, `fixes_in_issue`, `fixes_after`,
   `reverts_after`, `revision`; verified by `test_delivered_quality.rs` on fixture repositories; makes true QUA-1 to
   QUA-5, QUA-10.
2. [x] Review findings read from the done issue file, and `issue-workflow` step 3.6 records the count first
   (`Code review: <n> findings`); verified by fixture issue files and `test_skills.rs`; makes true QUA-6.
3. [x] Transcript signals: sessions, failed checks, typed messages per issue; verified by fixture transcripts with
   commits in their span; makes true QUA-7, QUA-8.
4. [x] `--summary` per revision, docs (architecture, README, report issue resources); verified by the test; makes
   true QUA-9 (spec `accepted`).
5. [x] Fixes of the `high` review: review table rows, sessions of parallel issues, fixes rebased after the close,
   reordered commits, cached repositories, fenced headings, quoted paths, summary in time order, argument checks,
   one `git log` per issue; verified by `test_delivered_quality.rs`; makes true QUA-3, QUA-4, QUA-6 to QUA-9.

## Decisions
- Mode: autonomous
- Base commit: 041d1a71c21479f93b3e9e93ec166e5f5ed4985f
- Window of the later fixes and reverts: 30 days after the close, `--window-days` to change it.
- Review findings: read the count when the line has one, else empty; `issue-workflow` step 3.6 now asks for the
  count first.
- Commits of an issue: those touching its file from the first holding `# Tracking` to the close (QUA-2), not
  `Base commit` (recorded in 9 of 42 done issues).

## Deliverable 1
- done: `delivered_quality.rs` (issue commits, deliverables, fixes inside, fixes and reverts after, revision),
  `default_branch` of `transcripts.rs` made public (scripts restamped), architecture and README.
- red: the script did not exist, every check failed; then the old issues with their plan under `# Tracking` (no
  `## Plan`) and the revision taken at the issue's own first commit (QUA-3 and QUA-5 amended).
- Smoke run on this repository: 39 rows in 0.6 s; the three issues closed after `origin/HEAD` have none (not pushed).
- Code review: 4 findings, all fixed: a hash pasted into the doc comment of `transcripts.rs` (scripts restamped), a
  dead filter, a fix in the same second as the close missed (QUA-4 now reads "after it in the history"),
  `--projects` accepted but unused (left to deliverable 3).

## Deliverable 2
- done: `reviews` and `review_findings` read from the `# Tracking` section (QUA-6 now scoped to it); `issue-workflow`
  step 3.6 asks for `Code review: <n> findings` first (passage required by `test_skills.rs`); architecture.
- red: the review columns were empty; the skill lacked the passage.
- Smoke run: 27 of 39 issues record a review, 22 with a count.
- Code review: 1 finding, fixed: a line summed every count it held (`3 findings, 1 finding dismissed` read as 4);
  only the first count of a line is read now (QUA-6 amended).

## Deliverable 3
- done: `--projects`, the sessions of each issue, its failed checks and typed messages (QUA-7, QUA-8);
  `is_validation` moved from `session_metrics.rs` to `transcripts.rs` (scripts restamped); architecture and README.
- red: `unknown option --projects`, the six transcript checks failed.
- Smoke run on the real transcripts: 0.6 s; the recent issues have 1 session each, 0 to 1 failed check, 1 to 7
  other typed messages.
- Code review: no findings (two questions checked: the log is read oldest first, `--reverse`; a repository that
  cannot be resolved has no commits, so no row shows its zeros).

## Deliverable 4
- done: `--summary` per revision (QUA-9), spec `accepted`, architecture, README, resources of
  `report_the_harness_evolution_by_revision`.
- red: `unknown option --summary`; then the fixture added the work tree copy of `gamma` one commit early (test fixed).
- Smoke run: 33 revisions over the 39 issues of this repository (one `self:` revision per issue or two).
- Code review: no findings.

## Deliverable 5 (fixes of the `high` review)
- Fixed (10 findings): `check | result` table rows no longer count as reviews (QUA-6); a session on the issue's
  branch is preferred and counts from its start (QUA-7, QUA-8); fixes rebased after the close count whatever their
  author date (QUA-4); first and last commit dates by min and max; repositories cached by `cwd`; headings inside
  fenced blocks end no section (QUA-3); `core.quotePath=false`; summary in the order the revisions were used
  (QUA-9); `--window-days` from 0 to 3650 and `--projects` without a value refused (QUA-1); one `git log -G` per
  issue instead of a `git show` per commit.
- Not tested: the quoted paths and the reordered commits (one-line changes, no fixture cheap enough).
- red: seven checks failed (table row, fenced comment, rebased fix, branch session, summary order, arguments).
- Code review: no findings.
