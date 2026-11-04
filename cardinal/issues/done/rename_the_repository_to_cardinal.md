# Rename the repository to cardinal

# Goal
The project is renamed "Cardinal". Rename the GitHub repository (`agent-harness` → `cardinal`) and the local
directory, and point every reference to the old URL at the new one, so later renames of the cardinal series start
from the right source.

# Findings
* Origin remote: `git@github.com:Electron-libre/agent-harness.git` (2026-10-28).
* `agent-harness` appears in 2 lines of `README.md` (title and the mermaid `source` subgraph) outside `issues/done/`.
* The consumers' `.claude/harness.toml` name the source by its git URL; GitHub redirects a renamed repository, but
  the redirect breaks as soon as a new repository takes the old name.

# Approach
1. `gh repo rename cardinal`, then `git remote set-url origin` to the new URL.
2. Rename the local directory (`agent-harness` → `cardinal`); the Claude Code project memory is keyed by the path,
   so move it along.
3. Update the source URL in each consumer's `.claude/harness.toml` (or let the redirect work until their next
   `harness update`); decide which.
4. Check the release assets (`release.yml`) and `install_harness.sh` still resolve with the new URL.

# Resources
* `README.md`, `.github/install_harness.sh`, `.github/workflows/release.yml`
* `docs/architecture.md` (source and consumers)

# Hints
* Outward-facing: the GitHub rename and the consumers' changes are confirmed with the user before running.
* Open worktrees of the repository hold the old absolute path: finish them first.

# Tracking

Plan: validated

## Understanding
Rename the GitHub repository and the local directory to `cardinal`, and the references to the repository name.
Out of scope: the product name in the docs and every `harness` command, file or path (the later issues).
The release downloads build their URL from the consumer's `source`, so GitHub's redirect covers them meanwhile.

## Plan
1. [x] Repository: `gh repo rename cardinal`, `git remote set-url origin git@github.com:Electron-libre/cardinal.git`,
   `README.md` title and mermaid subgraph to `cardinal`; verified by `gh repo view`, `git fetch` and `just check`.
2. [x] Consumer: the `source` of the one local consumer's `.claude/harness.toml` points to the new URL, left
   uncommitted in that project; verified by its `harness check`.
3. [x] Local directory: the commands to rename the local `agent-harness` directory to `cardinal` and move the Claude Code
   project memory, printed for the user to run once this session ends (the session runs inside the directory);
   verified by the user.

## Decisions
- Mode: autonomous
- Base commit: 06ee54ceef8c960464ae91263b7151100138cbaa
- GitHub rename by the agent; consumer `source` edited uncommitted; directory rename left to the user after the session.

## Summary
- done: GitHub repository renamed to `Electron-libre/cardinal`, origin remote updated, README names `cardinal`; the
  local consumer's `source` points to the new URL (uncommitted in that project).

| check | result |
|---|---|
| `gh repo view`, `git fetch` | pass |
| consumer `harness check` | pass |
| `just check` | pass |
| docs | README only |

- red: none (no test: a rename outside the code)
- Code review: no findings
- not verified: the release download through the new URL, until the next commit of `main` publishes one.
- decisions needed: none
