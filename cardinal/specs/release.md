---
id: REL
status: accepted
adrs: []
---

# Releasing a version

## Intent

`cardinal run release` (`just release`) prepares a numbered release of the project from its Conventional Commits: it
proposes the version, writes its section of `CHANGELOG.md` for the user to reread, and once the release is committed
creates the annotated tag `v<version>` locally. A consumer of Cardinal pins such a tag (`cardinal update --rev`) and
reads in the changelog what changed since its last pin.

## Requirements

- **REL-1** (deterministic) `release` with no version must print the proposed version: from the last tag `vX.Y.Z` reachable from `HEAD`, its major raised when a commit since it is breaking (REL-2), else its minor when one is a `feat`, else its patch; with no such tag, `0.1.0`.
- **REL-2** (deterministic) A commit must count as breaking when its subject marks it (`<type>!:` or `<type>(<scope>)!:`), when its message has a footer line starting `BREAKING CHANGE:` or `BREAKING-CHANGE:`, or when it changes a file named in the list `release_breaking_files` of `.claude/cardinal.toml`.
- **REL-3** (deterministic) `release <version>` must refuse, with a non-zero exit and no file changed, a version that is not `X.Y.Z` (numbers without a leading zero, an optional leading `v`), one whose tag `v<version>` exists, one that `CHANGELOG.md` already has a section for, and, once a version tag exists, one lower than the version REL-1 proposes, and any version when no commit follows that tag.
- **REL-4** (deterministic) `release <version>` must write in `CHANGELOG.md`, above its first `## ` section other than `## [Unreleased]` (at its end when it has none), a section `## [<version>] - <YYYY-MM-DD>` (today's date) holding, in this order and each only when not empty, `### Breaking`, `### Added` and `### Fixed`: one line `- <description>` per non-merge commit since the last version tag, the breaking ones under Breaking, the other `feat` under Added, the other `fix` under Fixed; the commits of the other types are not listed. With no version tag yet, the section holds the single line `First numbered release.` instead of these subsections. A missing `CHANGELOG.md` is created with the header of the seeded one.
- **REL-5** (deterministic) When `.claude/cardinal.toml` names a `release_version_command`, `release <version>` must run it (`sh -c`, from the root of the work tree) with the version as last argument after writing the changelog, and when it fails exit non-zero with `CHANGELOG.md` and the `roadmap` documents as they were before.
- **REL-6** (deterministic) `release <version>` must not commit, tag nor push, and prints the next steps: reread the section, commit, then `release <version> --tag`.
- **REL-7** (deterministic) `release <version> --tag` must create the annotated tag `v<version>` on `HEAD`, its message the changelog section of the version, and print the command that pushes it.
- **REL-10** (deterministic) `release <version> --tag` must refuse, with a non-zero exit and no tag, when the work tree has uncommitted changes or untracked files, when `CHANGELOG.md` of `HEAD` has no section for the version, or when the tag exists.
- **REL-8** (deterministic) `release` must never push.
- **REL-11** (deterministic) `release` must read each commit since the last version tag with the convention of its own revision: a commit whose `.claude/cardinal.toml` (`.claude/harness.toml` before its rename, ADR 0015), as of that commit, turns off the feature `conventional-commits` counts as breaking only when it changes a file of `release_breaking_files`, with no commit type nor `BREAKING CHANGE:` footer read, and is otherwise neither a `feat` nor a `fix`; a commit with no readable configuration follows the convention. When it proposes a version (no argument) or prepares one (`release <version>`) and a commit since the last tag is read without the convention, it prints to stderr the warning `release: conventional-commits is off: the version is proposed from release_breaking_files only`, followed, when `release_breaking_files` lists no file, by the line `release: release_breaking_files lists no file: the commits written with conventional-commits off are a patch`; its stdout with no version stays the version alone.
- **REL-12** (deterministic) `release <version>` must list each non-merge commit since the last version tag read without the convention (REL-11) that is not breaking under a subsection `### Changed`, after `### Fixed`, one line `- <subject>` (the whole subject) per commit, oldest first; the lines of `### Breaking` for such commits hold the whole subject too. The commits read with the convention stay listed as REL-4 says.
- **REL-9** (deterministic) `release <version>` must list under `### Milestones`, first in the section, every milestone (a `## <name>` heading outside fenced code blocks and HTML comments, its section read without the lines inside them) of the `roadmap` documents of `.claude/cardinal.toml` whose `State:` is `done` and that has no `Released:` line, and add to each of them the line `- Released: v<version>` after its `State:` line.

## Scenarios

- (REL-1) Given the tag `v0.2.0` and since it the commits `fix: a` and `feat(x): b`, when `release` runs, then it prints `0.3.0`.
- (REL-1) Given the tag `v1.4.2` and since it only `docs: a` and `fix: b`, when `release` runs, then it prints `1.4.3`.
- (REL-2) Given `release_breaking_files = ["cardinal-format"]` and since the tag `v0.2.0` a commit `feat: raise the format` that changes `cardinal-format`, when `release` runs, then it prints `1.0.0` and `release 0.3.0` is refused.
- (REL-3) Given the tag `v0.2.0` and since it a `feat`, when `release 0.2.1` runs, then it is refused and `CHANGELOG.md` is unchanged; `release 0.3.0` and `release 1.0.0` are accepted.
- (REL-3) Given the tag `v0.3.0` on `HEAD`, when `release 0.3.1` runs, then it is refused: nothing to release.
- (REL-4) Given a `CHANGELOG.md` whose first section is `## [Unreleased]` above `## [0.2.0]`, when `release 0.3.0` runs, then its section lands between them.
- (REL-4) Given no version tag and any commits, when `release 0.2.0` runs, then the section `0.2.0` holds only `First numbered release.`.
- (REL-4) Given the commits `feat!: drop the old flag`, `feat: add a flag`, `fix: repair a link` and `chore: tidy` since the last tag, when `release` writes the section, then Breaking holds `- drop the old flag`, Added `- add a flag`, Fixed `- repair a link`, and `tidy` is not listed.
- (REL-11) Given `disabled = ["conventional-commits"]`, the tag `v0.2.0` and since it the commits `feat: a` and `Add b`, when `release` runs, then it prints `0.2.1` and warns on stderr.
- (REL-11) Given `disabled = ["conventional-commits"]`, `release_breaking_files = ["format"]` and since the tag `v0.2.0` a commit `Raise the format` that changes `format`, when `release` runs, then it prints `1.0.0`.
- (REL-11) Given the tag `v0.2.0`, since it a commit `feat!: drop the old API`, then a commit that turns off `conventional-commits`, then `Add b`, when `release` runs, then it prints `1.0.0` and warns on stderr.
- (REL-11) Given `disabled = ["conventional-commits"]`, no `release_breaking_files` and a commit since the last tag, when `release` runs, then both warning lines are printed on stderr.
- (REL-12) Given `disabled = ["conventional-commits"]` and since the last tag the commits `Raise the format` (changing a listed file), `Add a flag` and `fix: repair a link`, when `release 1.0.0` runs, then Breaking holds `- Raise the format`, Changed `- Add a flag` and `- fix: repair a link`, and there is no Added nor Fixed.
- (REL-12) Given the tag `v0.2.0`, since it `feat!: drop the old API` and `feat: add a`, then a commit `chore: turn the convention off` that turns off `conventional-commits`, then `Add b`, when `release 1.0.0` runs, then Breaking holds `- drop the old API`, Added `- add a`, Changed `- chore: turn the convention off` and `- Add b`.
- (REL-9) Given `ROADMAP.md` with a milestone `done` released as `v0.2.0`, one `done` with no `Released:` line and one `in issues`, when `release 0.3.0` runs, then only the second is listed and gains `- Released: v0.3.0`.
- (REL-9) Given `ROADMAP.md` whose milestone `Real` (`in issues`) holds a fenced block with the lines `## Fake` and `- State: done`, followed by an HTML comment holding a template milestone `## Template` with `- State: done`, when `release 0.3.0` runs, then no milestone is listed and `ROADMAP.md` is unchanged.
- (REL-10) Given `release 0.3.0` ran and its changes are not committed yet, when `release 0.3.0 --tag` runs, then it is refused and no tag exists.

## Non-goals

- Pushing the tag or the branch, or creating a GitHub release: the user pushes.
- Rewording the commit subjects for the consumers: the user rereads and edits the section before the tag.
- Writing the version into a manifest itself: the project's `release_version_command` does it.
- Pre-release and build suffixes (`1.0.0-rc.1`).
