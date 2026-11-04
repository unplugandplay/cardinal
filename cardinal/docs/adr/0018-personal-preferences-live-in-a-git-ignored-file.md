# 0018. Personal preferences live in a git-ignored file

Status: Accepted
Date: 2026-10-30

## Context
`.claude/cardinal.toml` is committed and shared: every member of a team gets the same values. `user_language` is a
preference of the person who talks to the agent, not of the project, yet it is read from that file by the
`language-hint` hook, rule `user-language` of `rule-breaches` and the skills. Claude Code already splits its own
settings the same way (`settings.json` shared, `settings.local.json` personal).

## Decision
A person's preferences go in `.claude/cardinal.local.toml`, a `[project]` table whose keys override those of
`.claude/cardinal.toml` key by key, read by the readers of a personal key only (`user_language` today). The managed
`.claude/.gitignore` makes git ignore it; `sync` and `check` never touch it, so it is never managed. The shared value
stays the default: a project with no personal file behaves as before. Rule `user-language` reads the personal value as
it is when the measure runs, since the transcripts it reads are personal too and the file has no history; only the
shared value is read at the session's `start`.

## Consequences
Each member sets their language once per clone; `start-issue` copies the file into each issue worktree, which
git would otherwise leave without it. A measure run after a person changed their language judges their
older sessions against the new one. The shared keys (`commit_scopes`, `roadmap`…) stay shared: a personal file that
sets one is not read for it.
