# 0017. The source declares its migrations

Status: Accepted
Date: 2026-10-29

## Context
The two migrations of the consumers to `cardinal` (ADR 0015) renamed the managed names, but the project-owned files
kept citing the old ones (the `check-harness` dependency of the seeded `check` recipe, the header comments of the
seeded configuration, `CLAUDE.md`, `docs/`): the agent of each consumer found and fixed them by hand, about six
`grep`/`sed` calls each, after learning from the ADRs that `update` migrates at all. The migration code was in the
binary and knew its old names as hard-coded strings; it went with ADR 0016.

## Decision
The source that raises its format to `n` declares what changed in `migrations/<n>.toml`: the managed names replaced
(`names`, old and new) and the seeded texts to rewrite in the project files (`rewrite`, path, old and new, matched
as whole lines). `sync` and `update` apply every migration between the format of the lock and the format of the
source: they rewrite the seeded texts that are still byte-identical, and warn with `path:line` of each line of a
project-owned file tracked by git that still cites an old name, without rewriting it. `check` and `sync` hint at
`cardinal update` when the lock is of a lower format than the running `cardinal` reads.

The declaration is data: an older `cardinal` ignores `migrations/` harmlessly, so it does not raise the format
itself (ADR 0014). A change the data cannot express (moving the configuration or the lock) stays code of the
`cardinal` of the new format, which brings its own detection.

## Consequences
The author of a format change writes the old names once, in the source, where the rename is made; every consumer
gets the list of its leftovers instead of searching them. A cited name the project wrote itself is never rewritten:
the warning is printed once, by the run that applies the migration, so the agent of that run has to act on it.
