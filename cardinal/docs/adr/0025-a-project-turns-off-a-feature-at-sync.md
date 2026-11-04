# 0025. A project turns off a feature at sync

Status: Accepted
Date: 2026-11-01

## Context
Every consumer gets the whole harness. A user whose flow differs (an AI co-author trailer wanted, no parallel agents,
issues tracked elsewhere) cannot drop a part: a managed entry removed from the settings is a conflict at `sync`
(SYN-33), a managed file edited is a drift. Turning a part off must not cost more tokens nor time than keeping it: a
flag tested by a hook would still start a process at each tool call, and a conditional text would still be loaded.

## Decision
The source groups managed files and settings entries into named features in `features.toml` at its root. A project
lists the features it turns off in the `disabled` list of the `[cardinal]` table of `.claude/cardinal.toml`; `sync`
handles their files and entries as ones the source does not manage, so the existing removal, conflict and lock rules
apply unchanged and `check` needs no feature knowledge. A check of the binary that a feature carries reads `disabled`
from the configuration it already reads. A text of a shared file that belongs to a feature moves to a file of its
own (the rule against AI attribution leaves `AGENTS.md` for `.claude/rules/no-ai-attribution.md`) rather than being
cut out of a larger file at `sync`, which would make the lock hash a rendered text. The format rises to 7 (ADR 0014):
a `cardinal` that reads up to 6 would ignore `disabled` and install everything.

## Consequences
A feature turned off removes its tokens and hook calls; the default stays every feature on. The binary hard-codes the
names of the features its checks carry, which is safe since a project runs the binary of the revision it pins. A
project that edited a file of a feature it turns off keeps the file (reported `kept`). A feature lists the features it
needs in `needs`, and `sync` refuses a `disabled` that turns off one a feature left on needs (FEA-18). A `cardinal`
older than `needs` refuses the key (FEA-7) rather than misreading it, so the key alone raises no format; the first
source revision that declares a `needs` raises it, so that `update` names the `cardinal` to install.
