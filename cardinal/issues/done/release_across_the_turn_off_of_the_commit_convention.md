---
priority: low
depends_on: [release_a_project_without_the_commit_convention]
why: 'only the one release spanning the commit that turns `conventional-commits` off is affected, and the user can still pass a higher version'
---
# Release across the turn off of the commit convention

# Goal
`release` reads whether `conventional-commits` is off from the current `.claude/cardinal.toml` and applies it to
every commit since the last tag (REL-11). The release that spans the commit turning the feature off reads a
`feat!: drop the old API` written under the convention as a plain subject: it proposes a patch and lists it under
`### Changed`. The outcome: each commit is read with the convention its project followed when it was written.

# Findings
* Found by the `high` review of `release_a_project_without_the_commit_convention`.
* `rule-breaches` already reads the configuration as of each commit for rule `commit-format` (FEA-22).
* With the feature off and no `release_breaking_files`, every release is proposed as a patch; the warning of
  REL-11 names the key without saying the project sets none.

# Approach
1. Read `.claude/cardinal.toml` as of each commit (`git show <commit>:.claude/cardinal.toml`), as FEA-22 does, and
   classify it with the convention of that revision.
2. Add to the warning of REL-11, when `release_breaking_files` is empty, that every release is proposed as a patch.

# Specs
* `specs/release.md`: amends REL-11.

# Milestone
Optional features

# Resources
* `crates/cardinal/src/tools/release.rs` (`commits`, `classify`), `crates/cardinal/src/tools/rule_breaches.rs`
  (FEA-22), `specs/release.md`, `specs/features.md`.

# Tracking

Plan: validated by Cedric Brancourt <cedric.brancourt@gmail.com> on 2026-11-02

## Understanding
`release` reads each commit since the last tag with the `conventional-commits` setting of its own revision (as
FEA-22), not of the current `.claude/cardinal.toml`; a mixed release fills Breaking, Added, Fixed and Changed.
The warning of REL-11 gains a line when `release_breaking_files` lists no file. `release_breaking_files` stays read
from the current configuration (out of scope).

## Plan
1. [x] Per-commit convention: `commits` reads the configuration of each commit (`transcripts::shown` with
   `CONFIGS`, `features::turns_off`) and `classify` applies it; the warning is printed when the current config
   turns the feature off or a commit is read without it, with the second line when no file is listed; verified by
   tests of `crates/cardinal/tests/release.rs` (the scenarios of REL-11 and REL-12); makes true REL-11, REL-12. (autonomous)
2. [x] Fixes of the `high` review: the warning follows the commits read without the convention only;
   `commit_config` shared by `release` and `rule-breaches`; `release_breaking_files` read once; `docs/usage.md`
   names the minor of a release spanning the turn off; verified by `just check`. (autonomous)

## Decisions
- Mode: autonomous
- Base commit: 2e96b3b6750619d5fe3b3c3224c4ef67804cd484
- `release_breaking_files` stays read from the current configuration (question 1: a).

## Deliverable 1
- done: `release` reads each commit with the `conventional-commits` setting of its revision; the warning names an
  empty `release_breaking_files`.

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | REL-11, REL-12 traced, spec `accepted` |
| docs | `docs/usage.md` (Releases), doc comments of `release.rs` |
| code review | no findings |

- red: REL-11, REL-12: `left: "0.2.1\n" right: "1.0.0\n"` (the breaking mark written under the convention ignored),
  the second warning line missing, the section with no `### Added`.
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 2
- done: fixes of the `high` review (6 findings).

| check | result |
|---|---|
| tests | `just check` green |
| lint | clean |
| specs | REL-11 amended: the warning when a commit is read without the convention |
| docs | `docs/usage.md` |
| code review | no findings |

- red: none (refactor and a narrower warning condition, kept green by the REL-11 tests).
- High review: 6 findings; fixed: the warning on the working-tree config, the duplicated `commit_config`, the double
  read of `release_breaking_files`, the docs saying major or patch only; not fixed: the `[harness]` table (features
  came after the rename of ADR 0015, no `[harness] disabled` ever existed) and one `git show` per commit (a release
  spans tens of commits).
- Code review: no findings
