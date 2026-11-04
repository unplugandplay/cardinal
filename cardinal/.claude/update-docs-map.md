# Which documents to update after a change

Read by `skill:update-docs`.

| Change | Update |
|---|---|
| `cardinal` command or option | `README.md` (Commands), `docs/usage.md`, `docs/architecture.md`, `specs/sync.md` |
| Tool added, ported or dropped under `crates/cardinal/src/tools/`, or a change of `cardinal run` | `README.md` (What a project gets), `docs/usage.md`, `docs/architecture.md` (Tools and their binary), `specs/tool-binaries.md`, the recipes and skills that call it |
| File added, moved or removed under `files/` | `docs/architecture.md` (what is shared), `README.md` (What a project gets) for a skill or a recipe, then run `just sync` here and in the consumers |
| Step, stop or recipe of the issue process (`issue-workflow`, `create-issue`, `cardinal*.just`) | `docs/workflow.md` |
| New key read from `.claude/cardinal.toml` | `seed/.claude/cardinal.toml` (with its comment), `docs/architecture.md` |
| Project-owned file added under `seed/` | `docs/architecture.md` |
| Content of a seeded file changed | `docs/usage.md` (what a project seeded before changes by hand: it owns the file) |
| Hook added, renamed or dropped (a `cardinal run` tool), or an allow, ask or deny rule of the harness | `entries/.claude/settings.json`, `docs/architecture.md` (Hooks and their declaration), `docs/gates.md`, `README.md` (What a project gets, Gates), then `just sync` |
| Meaning of the managed content an older `cardinal` would misread (a key of `entries/`, a line of the lock, a directory of the source) | `cardinal-format` and `FORMAT` in `crates/cardinal/src/lib.rs`, raised together (ADR 0014), `specs/sync.md`; a managed name or a seeded text it replaces goes to `migrations/<n>.toml` (ADR 0017) |
| Feature added to `features.toml`, or a file, entry or check of the binary added to one | `features.toml`, `specs/features.md` (a check of the binary is a requirement), `docs/usage.md` (Features a project turns off), `docs/gates.md` when it holds a gate, `README.md` (Optional features) |
| Column or metric of a transcript reader (`session-metrics`, `cardinal-report`, `Expected-effect:` metrics) | its spec (`specs/evaluation.md`, `cardinal-report.md`, `cardinal-verdicts.md`), `docs/evaluation.md` (the readers table) |
| Fixture under `fixtures/` or recipe of this repository's `justfile` | `CLAUDE.md` (Index), `docs/evaluation.md` (The replay) |
| ADR added, or one superseded | `docs/adr/README.md` (Index: its line, and the status of the ADR it supersedes) |
