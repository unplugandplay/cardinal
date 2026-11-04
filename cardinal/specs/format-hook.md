---
id: FMT
status: accepted
adrs: []
---

# Format hook

## Intent

The `PostToolUse` hook `cardinal run format-code` (Edit|Write) formats the file the agent just edited with the
project's format command, `cargo fmt` when the project declares none, so the agent never spends a turn on formatting.

## Requirements

- **FMT-1** (deterministic) The hook must read the format command from the string `format_command` and the format patterns from the list of strings `format_files` of the `[project]` table of the `.claude/cardinal.toml` at the root of the work tree holding the edited file (`tool_input.file_path` of the event), each pattern matching a file name, `*` standing for any run of characters; `format_files` absent, every file name matches. Without `format_command`, or when the file is missing, does not parse or gives a key of another type, the format command is `cargo fmt --all` over the patterns `*.rs`.
- **FMT-2** (deterministic) The hook must run nothing when the event has no `file_path`, when the edited file is under a `.claude` directory of its work tree (Cardinal's copies) or its name matches no format pattern, and when the declared format command is empty or blank.
- **FMT-3** (deterministic) The hook must run a declared format command with `sh -c`, from the root of the work tree holding the edited file, with the path of the edited file as its last argument.
- **FMT-4** (deterministic) The hook must exit 0 and print nothing, whatever the format command prints or returns: formatting is best-effort, and a syntax error mid-edit does not block the agent.

## Scenarios

- (FMT-1) Given a Rust project with no `format_command`, when the agent edits a `.rs` file, then `cargo fmt --all` runs from the directory of that file (PAR-12).
- (FMT-3) Given `format_command = "npx prettier --write"` and `format_files = ["*.ts"]`, when the agent edits `src/a.ts`, then `npx prettier --write <path of src/a.ts>` runs from the root of the work tree.
- (FMT-2) Given the same project, when the agent edits `README.md`, then nothing runs.

## Non-goals

- Formatting the files the agent changes otherwise than by Edit or Write (a `sed`, a generator): `just check` catches them.
- Reporting a formatter that fails: the lint gate and `just check` report what is wrong with the code.
