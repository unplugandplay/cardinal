# 0007. Sync manages entries of the seeded settings

Status: Accepted
Date: 2026-10-27

## Context
The hooks are managed files, but Claude Code runs them only once declared in the project's `.claude/settings.json`,
a seed copied once and owned by the project afterwards. A hook the source adds, renames or drops, and a deny rule of
the review waiver added later, reach a project only by a hand edit; `check` and `sync` merely warn. Switching the
hooks to prebuilt binaries would change every hook command at once. ADR 0001 rejected merging managed files: a local
edit of a shared file is a conflict, fixed in the source.

Claude Code merges the settings of several scopes, but a project has one shared file (`.claude/settings.json`)
besides the personal, git-ignored `settings.local.json`; a plugin can carry hooks but no deny rule, and needs its
own install and trust. So there is no separate file the harness could own whole.

## Decision
The source declares the entries it manages in `entries/.claude/settings.json`: strings of `permissions.deny` and hook
objects of the `hooks` table. `sync` adds those the project's settings lack, removes those the lock recorded and the
source dropped, and leaves every other key and entry as the project wrote it. The lock records the managed entries,
as it records the hashes of the managed files, so a dropped or changed entry is recognised without guessing from the
command. The unit is the entry, not the file: this is not the file merge ADR 0001 rejected, since the project keeps
everything else of its file.

Rejected: a separate settings file (none exists per project besides the personal one); a plugin (no deny rules,
installation and trust outside the harness); recognising the harness's hooks by the script they name (a prebuilt
binary names none).

## Consequences
A new hook reaches every project at its next `sync`, and the switch to prebuilt binaries becomes one change in the
source. The seed no longer carries the managed entries: `sync` adds them right after seeding. The settings file is
rewritten by a JSON writer when an entry changes, so its formatting follows that writer. A project cannot keep a
managed hook off by deleting its declaration: it asks for a value in the source, as for a managed file.
