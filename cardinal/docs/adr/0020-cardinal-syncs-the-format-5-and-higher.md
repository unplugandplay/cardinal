# 0020. Cardinal syncs the format 5 and higher

Status: Accepted
Date: 2026-11-01

## Context
ADR 0019 kept, for the sources of the format 4, the reading of `issues/queue.md` by `next-issue`, its removal by
`close-issue`, and `next-issue --convert` that turns it into the ranks of the issue files; `migrations/4.toml` and
`migrations/5.toml` moved a lock of the formats 3 and 4 forward. On 2026-10-31 every consumer ranks its issue files
and is synced from a source of the format 5: nothing needs that code, its tests, NXT-4 to NXT-6 or those migrations
any more. ADR 0016 foresaw it: a minimum format, raised by an ADR once no consumer needs the lower ones.

## Decision
`cardinal` syncs the format 5 and higher: `init`, `sync` and `update` refuse a source below it, write nothing, and
name its format (SYN-51). No tool reads or writes `issues/queue.md`, and `--convert` is gone. The migrations to the
formats 4 and 5 are removed from the source. This supersedes the minimum format 3 of ADR 0016, and the part of ADR
0019 that kept the queue and its conversion.

## Consequences
A source below the format 5 needs the `cardinal` of its own revision. A project whose lock is below the format 5 is
not refused, but no migration moves it forward any more: it updates with a `cardinal` of a revision that still has
them, then with this one. A leftover `issues/queue.md` is read as an issue without a rank, which `next-issue`
reports.
