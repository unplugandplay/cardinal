---
id: CLG
status: accepted
adrs: [0005]
---

# Lint gate

## Intent

The Stop hook `cardinal run lint-gate` keeps the code of a project warning-free: it blocks the end of a turn while the
project's lint command reports a problem, `cargo clippy` when the project declares none. Everything it prints is read
by the agent and billed as tokens, so it runs the lint only when there is something new to check and reports the
problems in few lines.

## Requirements

- **CLG-1** (deterministic) The gate must exit 0 without running the lint when the event sets `stop_hook_active`, when the declared lint command is empty or blank (CLG-7), when the project declares no lint command and has no `Cargo.toml`, and when `git status` shows no changed or untracked file whose name matches the lint patterns (CLG-7) outside the `.claude` directories (the scripts of Cardinal are not project code).
- **CLG-2** (deterministic) When clippy, the default lint, fails, the gate must exit 2 and write to stderr one line per diagnostic, each naming the file, line and column of the problem with its message, plus the errors that have no location (a build script, a missing crate), and nothing of the build progress (`Checking`, `Compiling`, `could not compile`).
- **CLG-3** (deterministic) The gate must report at most 40 diagnostics of clippy, ending, when there are more, with one line giving how many were left out.
- **CLG-4** (deterministic) After a passing run, the gate must record a stamp of the lint state as the run leaves it (the lint command, the `HEAD` commit and the path and content of every changed or untracked file matching the lint patterns) in the git directory of the worktree; while that state is unchanged, the gate exits 0 without running the lint.
- **CLG-5** (deterministic) After a failing run (it records no stamp), and after any change of a file matching the lint patterns, of the `HEAD` commit, of the set of untracked files matching them or of the lint command since the stamp, the gate must run the lint again.
- **CLG-6** (deterministic) The gate must check the directory that sits, in the work tree holding the `cwd` of the event, where `CLAUDE_PROJECT_DIR` sits in its own work tree, and `CLAUDE_PROJECT_DIR` itself when the event has no `cwd` or one of the two is outside a git work tree.
- **CLG-7** (deterministic) The gate must read the lint command from the string `lint_command` and the lint patterns from the list of strings `lint_files` of the `[project]` table of the `.claude/cardinal.toml` at the root of the work tree of CLG-6, each pattern matching a file name, `*` standing for any run of characters; `lint_files` absent, every file name matches. Without `lint_command`, or when the file is missing, does not parse or gives a key of another type, the lint is `cargo clippy --workspace --all-targets -- -D warnings` over the patterns `*.rs`.
- **CLG-8** (deterministic) A declared lint command, run with `sh -c` from the directory of CLG-6, passes when it exits 0; when it fails, the gate must exit 2 and write to stderr one line naming the command, then the first 40 non-blank lines of its standard output followed by its standard error and, when there are more, one line giving how many were left out.

## Scenarios

- (CLG-4) Given a passing run, when the agent ends another turn having edited only Markdown files, then the lint does not run.
- (CLG-5) Given a failing run, when the agent ends the next turn without touching the code, then the lint runs again and reports the same problems.
- (CLG-5) Given a passing run, when the agent commits the changes, then the next end of turn with a dirty matching file runs the lint (the `HEAD` changed).
- (CLG-5) Given a passing run, when the project changes its `lint_command`, then the next end of turn with a dirty matching file runs the new command.
- (CLG-6) Given a crate in the subdirectory `backend/` of a repository and a session opened in it that entered an issue worktree, when the agent ends a turn, then the gate checks `backend/` of the issue worktree.
- (CLG-6) Given a session opened in the main worktree that entered an issue worktree, when the agent ends a turn with a clippy warning in the issue worktree, then the gate reports it, whatever the state of the main worktree.
- (CLG-7) Given a TypeScript project declaring `lint_command = "npx eslint ."` and `lint_files = ["*.ts"]`, when the agent ends a turn having edited a `.ts` file, then eslint runs and clippy does not.
- (CLG-4) Given a lint that fixes the files it checks (`eslint --fix`), when it passes, then the next end of turn with no new edit does not run it.
- (CLG-1) Given a project declaring `lint_command = ""`, when the agent ends a turn, then nothing runs.
- (CLG-8) Given a linter that prints 100 lines and exits 1, then the agent reads one header line, 40 lines and `... 60 more lines`.

## Non-goals

- Detecting a change that only touches `Cargo.toml`, `clippy.toml`, the toolchain or the configuration of the linter: the next edit of a matching file or `just check` catches it.
- Several lint commands, one per stack: the project chains them in its one command (a `just lint` recipe).
- Parsing the output of a linter other than clippy per diagnostic: its lines are cut, not parsed.
- The output of the `just` recipes (`check`): they are recipes of Cardinal, not a requirement of this hook.
