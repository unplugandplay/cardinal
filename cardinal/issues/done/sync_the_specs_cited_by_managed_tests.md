# Make the specs cited by managed tests available in the projects

# Goal
After `harness update`, `just check` of a project must pass. The managed tests of the parallel-issues work cite
`PAR-1` to `PAR-9` with `// spec: ID`, but `specs/parallel-issues.md` is not a managed file, so the spec checker of
the project fails.

# Findings
* A project updated from the previous revision to the head of the default branch fails `just check-specs` with
  nine errors of the form "cites `PAR-n`, which no spec defines" (test_issue_worktree.rs, test_next_issue.rs,
  test_close_issue.rs, test_require_code_review.rs).
* Copying `specs/parallel-issues.md` into the project is not enough: its front matter lists `adrs: [0003]`, and the
  checker then fails with "adr 0003 not found in docs/adr". The ADR belongs to the harness repository too.
* `files/specs/` only holds `README.md` and `TEMPLATE.md`.

# Approach
1. Measure: list every id cited by a managed test and where its spec lives.
2. Candidate solutions, can be combined:
   1. Sync the specs of the managed scripts (and the ADRs they link) as managed files.
   2. Let the checker skip a citation under `.claude/` whose spec is not in the project, since the harness repository
      already traces it.
   3. Make `harness sync` or `harness check` report the gap when it copies tests whose spec is missing.

# Specs
* `specs/sync.md` if the set of managed files changes; `specs/parallel-issues.md` and `specs/README.md` for the checker rule.

# Resources
* `files/.claude/specs/check_specs.rs`, `specs/parallel-issues.md`, `docs/adr/0003-one-worktree-per-issue.md`, `specs/sync.md`

# Hints
* The shared files hold nothing specific to one project: a synced spec must not mention one.
