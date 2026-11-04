# Keep the session transcripts long enough to evaluate the harness

# Goal
Evaluating the harness from the session transcripts needs a history that spans several harness revisions.
Claude Code deletes the transcripts after a retention period, so the history is lost before it can be compared.
Make sure the transcripts (or what the evaluation needs from them) survive long enough.

# Findings
* 2026-10-26: 188 session transcripts across the projects directory of Claude Code; the oldest one dates from
  2026-09-26, exactly 30 days earlier, which matches the default retention (`cleanupPeriodDays`, 30 days).
* No `cleanupPeriodDays` is set in the user settings nor in `files/.claude/settings.json`.
* The transcripts hold private content: `skill:audit-instructions` already forbids writing it in a repository.

# Approach
Measure first: confirm from the Claude Code docs where `cleanupPeriodDays` is read (user settings only, or
project settings too) and whether a project value changes the cleanup of every project.
1. Document in `README.md` that the evaluation needs a longer retention, with the setting to raise in the user
   settings (the harness does not own them).
2. Have `harness check` (or `sync`, like the settings warning of `issues/done/warn_of_the_settings_at_sync.md`)
   warn when the retention is the default.
3. Archive, outside the repositories, a compact extract of each session (metrics only, no message text) so the
   history no longer depends on the retention; to combine with the metrics extractor of
   `extract_session_metrics_from_the_transcripts`.

# Resources
* `files/.claude/skills/audit-instructions/SKILL.md` (confidentiality of the transcripts)
* `issues/done/warn_of_the_settings_at_sync.md`
* Claude Code settings documentation (`cleanupPeriodDays`)

# Hints
* Urgent by nature: every day without it loses one day of history.
* The harness must not rewrite the user settings; warn or document only.

# Specs
* `specs/sync.md`: adds SYN-28 (retention warning of `check`, `sync` and `update`).

# Tracking

Plan: validated

## Understanding
Measured (Claude Code docs, settings reference): `cleanupPeriodDays` is read from any settings file, but the sweep
deletes the transcripts of every project with the value of the session that runs it, so only the user settings protect
them all; a project value below it shortens the history of every project. Out of scope: rewriting the user settings,
archiving compact extracts (moved to `extract_session_metrics_from_the_transcripts`, which owns the metrics).

## Plan
1. [x] Retention warning: `check`, `sync` and `update` warn when the user settings keep the transcripts less than
   365 days or a project settings file lowers it; verified by unit tests on fixture settings in a tempdir; makes true
   SYN-28.
2. [x] Docs: `README.md` says the evaluation needs a longer retention and which user setting to raise; the archive
   point moves to `issues/done/extract_session_metrics_from_the_transcripts.md`; verified by `just check`.

## Decisions
- Mode: autonomous
- Base commit: b410ad5
- 2026-10-26: `cleanupPeriodDays` set to 365 in the user settings by the agent, with the user's agreement.
- Warning threshold: 365 days.
- The archive of compact extracts moves to `extract_session_metrics_from_the_transcripts`.

## Deliverable 1

* done: `settings::retention` and `harness::retention_warnings`, printed by `main` after `check`, `sync` and `update`
  (once, not under CI); `specs/sync.md` SYN-28 `accepted`; README and `docs/architecture.md` updated.
* red: `cannot find function retention_with` / `user_settings` (8 errors) before the code.
* checks: `just check` ok, `just test-harness` ok.
* code review: no findings.
* smoke test: `harness check` silent with the user settings at 365; with `CLAUDE_CONFIG_DIR=/tmp/nowhere` it warns
  and exits 0; with `CI=1` it is silent.
* not verified: nothing.

## Deliverable 2

* done: README says why and how to keep the transcripts a year; the archive point is now point 5 of
  `issues/done/extract_session_metrics_from_the_transcripts.md`.
* red: none (docs only).
* checks: `just check` ok.
* code review: no findings.
* not verified: nothing.

## Deliverable 3 (fixes of the high review)

* done: unreadable or invalid user settings get their own warning instead of a false 30 days; no `HOME` nor
  `CLAUDE_CONFIG_DIR` gives a "cannot be located" warning instead of a relative `~` path; a test of the binary proves
  the warning comes out once after `check` and `sync`, never after `init`; the days are an `f64` throughout.
* not fixed: folding the retention into `Report.warnings` / `Checked.warnings` (the library `check` and `sync` read
  the project only, so their tests do not depend on the user's settings; the test of the binary covers `main`);
  reading `.claude/settings.json` twice (two error policies on purpose: SYN-24 to SYN-26 already report a broken file).
* red: type errors on `retention_with(.., Option<&Path>)` and `user_settings(..) -> Option<PathBuf>` before the code.
* checks: `just check` ok, `just test-harness` ok.
* code review: no findings.
* not verified: nothing.
