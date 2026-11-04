# 0004. The review gate moves from the command to the repository

Status: Accepted, partly superseded by 0005 (the Claude hook no longer reads the command for the repository)
Date: 2026-10-25

## Context
`require_code_review.rs` refuses a `git commit` until the diff was reviewed. To know which repository a commit lands in
it reads the text of the command (`-C`, `--git-dir`, `--work-tree`, `GIT_DIR=`, `export`, fish `set -x`). Every new way
to name a repository (`$X`, `$(…)`, a variable exported by an earlier command) was a bypass until the parser learned
it, and some cannot be learned: the hook never sees the environment of the shell. Validating the commit after its
creation and rolling back was considered and rejected: a `post-commit` hook cannot tell which repository to reset.

## Decision
A native git `pre-commit` hook judges the repository git really commits into, with the same marker, snapshot,
exemptions and waiver as the Claude hook (`require_code_review.rs native`; one code, no second implementation).
`harness sync` installs it (`.git/hooks` is not versioned, so each clone needs its own sync): a marked `sh` script that
runs the review hook, then the user's own `pre-commit`, which is moved to `pre-commit.user` and never overwritten. For a
linked worktree it goes to the hooks of the common git directory. When `core.hooksPath` is set, `sync` installs nothing
and says so. `harness check` reports a missing or edited gate, except under `CI`.

The Claude hook stays, for what a native hook cannot do: it refuses `--no-verify`, `-n` (alone or bundled, on every
commit of the line) and any command that sets `core.hooksPath`, and keeps reading the command text for the repository.
It is now the first line of defence for the agent, and the native hook the last resort when the text could not be read.

## Consequences
A commit made through an unreadable repository option or variable is refused by git itself, not guessed. The text
parser can stay approximate without being the only barrier. The remaining bypasses, none stopped by either hook: a
`pre-commit` edited or deleted with a file tool; `core.hooksPath` moved by editing `.git/config`, by an alias, a shell
variable, `GIT_CONFIG_GLOBAL` or `-c include.path=…`; a gate left in `.git/hooks` before `core.hooksPath` was set (not
checked, not updated); a commit made by a tool other than the agent; a machine without `rust-script` or a branch
without the review hook, where the script warns and lets the commit through (a refusal would block every commit of
a repository that predates the harness). The native hook judges the whole working tree like the Claude hook, which is
stricter than a commit of staged files only. `check` depends on the local git hooks, hence the exception under `CI`
(the shared workflow would be red on every fresh clone); a CI that does not set `CI` must run `sync` first. The
user's `pre-commit` runs through one more `sh` process, and a symlinked one is kept as a link to its resolved target.
The installed hook is an `sh` script and nothing was run on Windows: a consumer there gets a gate only where git
provides `sh` (Git for Windows does). The text rules of the Claude hook (REV-1 to REV-8) stay requirements of that hook
and stay tested, but they are no longer what the gate rests on: the native hook decides when they cannot read the
command.
