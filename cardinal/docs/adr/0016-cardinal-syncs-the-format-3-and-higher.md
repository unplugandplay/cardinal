# 0016. Cardinal syncs the format 3 and higher

Status: Superseded by 0020 (the minimum format is 5)
Date: 2026-10-29

## Context
ADR 0015 kept the names before the rename (`.claude/harness.toml` and its `[harness]` table, `.claude/harness.lock`,
`harness.just`) for a source below the format 3, and the migration that moves a project to the new names, because
the tools of such a revision read only the old ones. On 2026-10-29 every consumer (this repository, RetroDoc,
Assistant) is pinned to a revision of the format 3 and migrated: nothing reads or writes the old names any more but
that fallback and that migration, which cost code, tests and five requirements.

## Decision
`cardinal` writes and reads the new names only. `init`, `sync` and `update` refuse a source below the format 3, write
nothing, and name its format: a project that needs such a revision installs the `cardinal` of that revision. `check`,
`run`, the tools and the native gate read `.claude/cardinal.toml` and `.claude/cardinal.lock` only. The readers of the
history (the evaluation, the rule breaches, the delivered quality) keep both names: past commits hold the old ones.

This supersedes the part of ADR 0014 that has a `cardinal` read every lower format, and the old-name branch and the
migration of ADR 0015.

## Consequences
A source below the format 3 needs the `cardinal` of its own revision: this one refuses it instead of misreading it. A
project still on the old names is not detected: this one finds no configuration in it (`check`, `sync` and `update`
fail naming `.claude/cardinal.toml`), and the `cardinal` of its revision reads it. The next format that drops what an older one needs does the same: a minimum
format, raised by an ADR once no consumer needs the lower ones.
