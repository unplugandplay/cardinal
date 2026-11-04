# 0015. The consumer files are named after the format

Status: Accepted, partly superseded by 0016 (the old names and the migration are dropped)
Date: 2026-10-28

## Context
The product is `cardinal` (ADR 0012), but the files it keeps in each project still carry the old name:
`.claude/harness.toml` and its `[harness]` table, `.claude/harness.lock`, `harness.just`. A project pinned to an older
revision keeps running the tools of that revision, which `cardinal run` fetches, and they read only the old names:
renaming the files of such a project, whichever `cardinal` syncs it, breaks its hooks and recipes until its `update`.

## Decision
The names follow the format of the source (ADR 0014), which is raised to 3 with the rename: `sync` and `update` from a
source of the format 3 or higher write `.claude/cardinal.toml` (table `[cardinal]`), `.claude/cardinal.lock` and manage
`cardinal.just`, and migrate a project that still has the old names (the configuration moved with its table line
renamed, the lock rewritten under its new name, the `import` line of the `justfile` rewritten); from a lower format
they keep the old names. What reads no source (`check`, `run`, the tools, the native gate) reads the new name when
it exists, else the old one; the history readers do the same at each commit.

## Consequences
A project migrates at the `sync` or `update` that brings it the content written for the new names, never before.
A `cardinal` of the format 2 finds no configuration in a migrated project: the one of its revision must be installed.
The old names are read as long as `cardinal` reads the format 2; dropping them drops that format.
