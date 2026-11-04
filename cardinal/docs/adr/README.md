# Architecture decision records

One file per decision, `NNNN-short-title.md`, numbered in order, never renumbered. A decision that changes
an earlier one gets a new ADR that says so; the old one is marked `Superseded by NNNN`. A new ADR adds its line to the
index below, which also follows the status of the ones it supersedes.

Template:

```markdown
# NNNN. Title

Status: Proposed | Accepted | Superseded by NNNN
Date: YYYY-MM-DD

## Context
What forces the decision (constraints, measurements).

## Decision
What we do, in a few sentences.

## Consequences
What gets easier, what gets harder, what must be redone (for example a full re-ingest).
```

Write an ADR for: the layout of the source or of a consumer (file names, what is managed, seeded or merged), the
format of the managed content and its migrations, where a tool runs and how it is installed, a limit the gates keep
on purpose, anything a future reader would otherwise undo by accident.

## Index

| ADR | Status |
|---|---|
| [0001. Shared harness, synchronised by a Rust tool](0001-shared-harness-synced-by-a-rust-tool.md) | Accepted |
| [0002. `rev` pins the resolved commit](0002-rev-pins-the-resolved-commit.md) | Accepted |
| [0003. One worktree per issue](0003-one-worktree-per-issue.md) | Superseded by 0008 |
| [0004. The review gate moves from the command to the repository](0004-review-gate-moves-from-the-command-to-the-repository.md) | Accepted, partly superseded by 0005 |
| [0005. The gates catch mistakes, not bypasses](0005-the-gates-catch-mistakes-not-bypasses.md) | Accepted |
| [0006. The transcript readers are managed scripts sharing one module](0006-transcript-readers-are-managed-scripts.md) | Superseded by 0010 |
| [0007. Sync manages entries of the seeded settings](0007-sync-manages-entries-of-the-seeded-settings.md) | Accepted |
| [0008. Issue worktrees nest under .claude/worktrees](0008-issue-worktrees-nest-under-claude-worktrees.md) | Superseded by 0023 |
| [0009. The scanners list the repository files through git](0009-scanners-list-the-repository-files-through-git.md) | Accepted |
| [0010. The tools ship in the `harness` binary of the pinned revision](0010-the-tools-ship-in-the-harness-binary.md) | Accepted |
| [0011. The user opens the issue worktree](0011-the-user-opens-the-issue-worktree.md) | Accepted |
| [0012. The product is named Cardinal](0012-the-product-is-named-cardinal.md) | Accepted |
| [0013. Sync manages the allow rules too](0013-sync-manages-the-allow-rules-too.md) | Accepted |
| [0014. The source declares the format cardinal must read](0014-the-source-declares-the-format-cardinal-must-read.md) | Accepted, partly superseded by 0016 |
| [0015. The consumer files are named after the format](0015-the-consumer-files-are-named-after-the-format.md) | Accepted, partly superseded by 0016 |
| [0016. Cardinal syncs the format 3 and higher](0016-cardinal-syncs-the-format-3-and-higher.md) | Superseded by 0020 |
| [0017. The source declares its migrations](0017-the-source-declares-its-migrations.md) | Accepted |
| [0018. Personal preferences live in a git-ignored file](0018-personal-preferences-live-in-a-git-ignored-file.md) | Accepted |
| [0019. Each issue file carries its rank](0019-each-issue-file-carries-its-rank.md) | Accepted, partly superseded by 0020 and 0021 |
| [0020. Cardinal syncs the format 5 and higher](0020-cardinal-syncs-the-format-5-and-higher.md) | Accepted |
| [0021. Each issue declares its priority and dependencies](0021-each-issue-declares-its-priority-and-dependencies.md) | Accepted |
| [0022. A session claims its issue and locks its workspace](0022-a-session-claims-its-issue-and-locks-its-workspace.md) | Accepted, partly superseded by 0024 |
| [0023. Worktrees open next to the main one, with no issue](0023-worktrees-open-next-to-the-main-one-with-no-issue.md) | Accepted |
| [0024. A session with nothing in progress hands over its worktree lock](0024-a-session-with-nothing-in-progress-hands-over-its-worktree-lock.md) | Accepted |
| [0025. A project turns off a feature at sync](0025-a-project-turns-off-a-feature-at-sync.md) | Accepted |
| [0026. A queue of issues is chained by the main loop](0026-a-queue-of-issues-is-chained-by-the-main-loop.md) | Accepted |
