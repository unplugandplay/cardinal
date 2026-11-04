# 0014. The source declares the format cardinal must read

Status: Accepted, partly superseded by 0016 (a `cardinal` reads the lower formats down to 3 only)
Date: 2026-10-28

## Context
The meaning of the managed content grows with `cardinal`: ADR 0013 added the allow rules to
`entries/.claude/settings.json` and an `allow` line to the lock. A `cardinal` built before it ignores the allow rules
of a newer source (the rule is dropped without a word) and fails on its lock with `not a settings entry`. The guard of
`update` (BIN-16) only asks the installed launcher about the hook subcommands: it does not see a change of the
fragment or of the lock. The crate version is never raised, so it cannot tell the two binaries apart.

## Decision
A whole number, the format, names what a `cardinal` can read of a source and of a lock. The source declares the
format its content needs in `cardinal-format` at its root; the binary reads up to the format compiled in it, which a
test keeps equal to the `cardinal-format` of its own revision; the lock records the format of the source it was
synced from, as a line `format <n>` that a binary built before this decision skips: its lines come from the content of
that source, and a binary of a higher format must not make the lock unreadable by the binary pinned with the source
(the one of the CI, of the teammates). `sync` and `update` refuse a source of a higher format, and `sync`, `update` and
`check` a lock of a higher one, with the command that installs a binary that reads it. A source or lock with no format has the format 1; this decision starts at 2, the format of ADR 0013.

The format is raised in the commit that changes the meaning of the managed content (a key of the fragment, a line of
the lock, a directory of the source) in a way an older binary would misread; an addition an older binary reads
correctly or ignores harmlessly does not raise it.

## Consequences
A project whose installed `cardinal` is too old is told to install the matching one instead of being synced wrongly.
The binaries released before this decision cannot refuse anything. Raising the format is a manual step: a change
that misses it is not caught by any check, only by the review.
