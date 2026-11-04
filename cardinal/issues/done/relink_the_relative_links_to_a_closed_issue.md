# Relink the relative links to a closed issue

# Goal
`just close-issue issues/foo.md` rewrites `issues/foo.md` to `issues/done/foo.md` in the text files of the repo, but
leaves the relative Markdown links of `docs/` (`[x](../issues/foo.md)`, `../../issues/foo.md` from `docs/adr/`)
pointing at the old path. After a close, every link to the issue, relative or not, must resolve to
`issues/done/foo.md`.

# Findings
* Reproduced on 2026-10-28 in a temporary git repo with `harness run close-issue issues/foo.md`: `` `issues/foo.md` ``
  in `docs/x.md` and `docs/adr/y.md` became `issues/done/foo.md`; `[a](../issues/foo.md)` and
  `[b](../../issues/foo.md)` stayed unchanged.
* Cause in `relink_text` (`crates/harness/src/tools/close_issue.rs`): `issues/` is rewritten only when the character
  before it is whitespace or one of `` `"'([<{=,: ``; a `/` (as in `../issues/`) is not a boundary, on purpose so
  that `docs/issues/` is not touched.
* The match is textual: it does not resolve a link against the directory of the file that holds it, so it cannot
  tell `../issues/foo.md` from `docs/issues/foo.md`.
* No spec covers the relinking of `close-issue` (only PAR-8 and PAR-14 of `specs/parallel-issues.md` name it).
* `--relink` uses the same `relink_text`, so it does not repair the old relative links either.

# Approach
1. Resolve each candidate link against the directory of the file that holds it (`../issues/foo.md` from `docs/`
   is `<root>/issues/foo.md`), and rewrite it when it designates the closed issue file, keeping its relative form
   (`../issues/done/foo.md`).
2. Keep the textual match for the paths written from the root (`issues/foo.md` in code spans and prose), as today.
3. Write the requirements of the relinking in a spec (amend `specs/parallel-issues.md` or a new
   `specs/close-issue.md`), with a test per form: root path, `../`, `../../`, and `docs/issues/foo.md` left alone.

# Specs
* `specs/close-issue.md` (new): CLS-1 to CLS-5.

# Resources
* `crates/harness/src/tools/close_issue.rs` (`relink_text`, `relink`, `--relink`)
* `specs/parallel-issues.md` (PAR-8, PAR-14), `crates/harness/src/tools/repo_files.rs`

# Hints
* A link between two issues (`[x](bar.md)` in `issues/foo.md`) also breaks once one of them moves to `done/`: in
  scope or not is a decision of step 1.
* Run `--relink` on this repository once fixed, to repair the relative links already dangling (none found in
  `docs/` on 2026-10-28).

# Tracking

Plan: validated

## Understanding
After a close, every link to the issue resolves to `issues/done/<stem>.md`: root paths as today, plus the Markdown
link targets resolved against the directory of their file. HTML links, URLs and bare file names are out of scope.

## Plan
1. [x] Relink the relative Markdown links: resolve each link target of a `.md` file against its directory in
   `relink_text`, for `close-issue` and `--relink`; verified by new checks in `crates/harness/tests/close_issue.rs`
   (`../`, `../../`, `foo.md` from `issues/`, `#anchor`, targets left alone), then `--relink` on this repository;
   makes true CLS-1 to CLS-5. About 200 lines.
2. [x] Fixes of the `high` review: links in code spans, fenced blocks and footnotes left alone (CLS-6), `<...>`
   reference targets, `done/` and trailing `/` kept, links above the root rebased (CLS-4 amended), no silent
   fallback when the issues directory is not under the root, files that cite no closed issue skipped; verified by
   new checks in `crates/harness/tests/close_issue.rs`; makes true CLS-4, CLS-6.

## Decisions
- Mode: autonomous
- Base commit: 19c72ea
- Relative links of the moved issue itself are rewritten (CLS-4).

## Deliverable 1
- done: `close-issue` and `--relink` rewrite the Markdown link targets that resolve to a closed issue, and rebase
  the relative links of the moved issue; new spec `specs/close-issue.md` (accepted).
- red: CLS-2, CLS-3, CLS-4, CLS-5: `FAIL - a relative link from docs/ and docs/adr/ follows the issue, anchor kept`
  (and the inter-issue, moved-issue and `--relink` checks).
- `--relink` on this repository: no relative link to repair; it only rewrote a quoted command in
  `issues/done/move_tracked_issues_with_git_mv.md` by CLS-1 (pre-existing behaviour), reverted.
- checks: `just check` ok (167 tests, clippy, fmt, specs `--strict`); docs: `issue-workflow` rule on `close-issue`
  (`files/` and its synced copy).
- Code review: no findings
- not verified: none. decisions needed: none.

## Deliverable 2
- `high` review of `19c72ea..519d75f`: 9 findings. Fixed: empty target for `done/`, footnotes, links above the
  root, code spans and fences, `<...>` reference targets, silent fallback of the issues directory, trailing `/`,
  files that cite no closed issue parsed for nothing. Not fixed: a root path `issues/<stem>.md` written in a
  Markdown link of a subdirectory is still rewritten by CLS-1 (pre-existing behaviour, which CLS-3 leaves open).
- red: CLS-4, CLS-6: `FAIL - the links of the moved issue still resolve from done/` and `FAIL - a relative link
  from docs/ and docs/adr/ follows the issue, anchor kept` (code span and `<...>` reference).
- checks: `just check` ok; docs: `specs/close-issue.md` (CLS-4 amended, CLS-6 added, accepted).
- Code review: no findings
- not verified: none. decisions needed: none.
