---
priority: low
depends_on: [order_the_issues_by_priority_and_dependencies]
why: 'a guard for a rare mistake: next-issue already lists the blocked issues apart'
---
# Warn when starting a blocked issue

# Goal
`next-issue` never offers an issue whose `depends_on` is not all done (NXT-7), but `start-issue` opens a worktree for
any committed issue: a user who names a blocked issue starts work whose prerequisite is not done, with no word.
The outcome: `start-issue` refuses or warns on a blocked issue, naming what blocks it.

# Findings
* Review of `order_the_issues_by_priority_and_dependencies`, 2026-11-01: `issue_worktree.rs` checks the issue file
  and the branch, not `depends_on`.

# Approach
1. `start-issue` reads the readiness through `next_issue.rs` and refuses a blocked issue, with a `--force`; or
2. it only warns, keeping its exit code.

# Specs
* `specs/parallel-issues.md`: a new requirement for `start-issue` on a blocked issue.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/issue_worktree.rs`, `crates/cardinal/src/tools/next_issue.rs`, `specs/next-issue.md`.

# Tracking

Withdrawn: `open-worktree` replaced `start-issue` and takes no issue (ADR 0023,
`issues/done/reuse_a_worktree_across_issues.md`), so there is no issue to check the readiness of; `next-issue` still
never offers a blocked issue (NXT-7).
