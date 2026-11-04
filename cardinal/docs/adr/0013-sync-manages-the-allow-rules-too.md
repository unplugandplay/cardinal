# 0013. Sync manages the allow rules too

Status: Accepted
Date: 2026-10-28

## Context
ADR 0007 lets `sync` manage the deny rules and hooks of the project's `.claude/settings.json`; the allow rules stayed
in the seed, copied once. The managed skills tell the agent to run `cardinal run <tool>`, but a project seeded before
the binary was renamed (ADR 0012) allows `Bash(harness run *)` only: each `cardinal run` asks for a permission, which
a non-interactive run denies. A warning from `check` would leave every project to edit its settings by hand.

## Decision
The managed entries of `entries/.claude/settings.json` include the strings of `permissions.allow`, handled as those of
`permissions.deny`: added when missing, recorded in the lock, removed when the source drops them and the settings
still hold them. The source manages only the allow rules its managed content needs (`Bash(cardinal run *)`); the
other allow rules of the seed stay the project's.

## Consequences
A command a managed skill starts to run reaches every project with its allow rule at the next `sync`. The seed no
longer carries `Bash(cardinal run *)`. The old `Bash(harness run *)` of a project stays until the project removes it:
it was never recorded as managed.
