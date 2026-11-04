# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project

Cardinal, the shared agent harness (skills, hooks, scripts, recipes, CI) of the user's projects, and the Rust tool
`cardinal` that copies it into each of them. → Design: [docs/architecture.md](docs/architecture.md)

## Index

- `files/`: the managed content, copied to the same path of every consumer. Never edit the copies in a
  consumer: fix them here.
- `seed/`: project-owned starting points, copied once.
- `entries/.claude/settings.json`: the hooks and allow, ask and deny rules `sync` keeps in each project's `.claude/settings.json`
  (declare a new hook here).
- `features.toml`: the features of the managed content a project may turn off (`disabled`, `specs/features.md`).
- `crates/cardinal/`: the `cardinal` binary (`init`, `sync`, `check`, `run` for the tools of `src/tools/`).
- `fixtures/replay/`: the project `just replay <rev> <runs>` copies to run headless (`claude -p`, billed) against a
  revision and grade: its issue is handed with a validated plan, its held-out test is in `grade/` (`skipped_dirs`).
- `specs/`: what the tool and its shared hooks must do (one file per capability, see `specs/README.md`), traced to
  its tests by `// spec: ID`.
- `issues/`: open issues, ordered by the `priority` and `depends_on` of their front matter (`just next-issue`); `docs/adr/`: decisions.
- Parallel agents: the user opens a worktree (`git worktree add` or `just open-worktree`) and launches
  `claude` in it; the agent claims each issue (`cardinal run issue-claim`), one session per worktree (`workspace-lock`
  hooks), and never creates or enters a worktree, see `docs/architecture.md`.
- `.claude/skills/commit-message/conventional-commits.md`: commit message format (Conventional Commits, enforced by
  `just check-commit`);
  `.claude/rules/user-language.md` and `.claude/rules/reports-to-the-user.md`: the language and the format of the reports
  to the user, loaded in every session.

This repository is its own first consumer: its `.claude/`, `AGENTS.md`, `cardinal*.just`, `script/` and
`specs/README.md` are copies of `files/`. After changing `files/`, run `just sync`; `just check-cardinal`
fails when a copy drifted.

Essentials: `just check` (fmt + clippy -D warnings + tests + spec traceability) before
committing (it runs the tests of the tools and of the shared content too); no `Co-Authored-By` or AI attribution in commits;
everything in English; the shared files hold nothing specific to one project (project values go through
`.claude/cardinal.toml`). A commit that changes `files/` may add `Expected-effect: <metric> <up|down>` to its
footer, one line per metric of `cardinal-report` it should move: a direction, never a target; `skill:evaluate-harness`
gives it a verdict ([specs/cardinal-verdicts.md](specs/cardinal-verdicts.md)). A change of the meaning of
the managed content (fragment keys, lock lines, source layout) that an older `cardinal` would misread raises
`cardinal-format` and `FORMAT` together (ADR 0014), and declares the names and seeded texts it replaces in
`migrations/<n>.toml` (ADR 0017).

## Test conventions

No network in unit tests; filesystem tests use `tempfile::tempdir()`. A test that proves a requirement of
`specs/sync.md` carries `// spec: SYN-n` right above it.
