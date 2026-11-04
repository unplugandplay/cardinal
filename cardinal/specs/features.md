---
id: FEA
status: accepted
adrs: [0014, 0025]
---

# Features a project turns off

## Intent

A source groups its managed files and settings entries into named features; a project lists the ones it turns off in
`.claude/cardinal.toml`, and `sync` leaves them out, so a feature turned off costs no token and no hook call. The
checks of the binary that a feature carries stay quiet in a project that turned it off.

## Requirements

- **FEA-1** (deterministic) `sync` must read the features of the source from `features.toml` at its root: one table per feature, named after it, whose `files` lists paths under `files/` and whose `entries` lists managed entries of `entries/.claude/settings.json` written as `sync` prints them (`allow <rule>`, `deny <rule>` or `hook <event> <matcher> <command>`, SYN-34); a source with no `features.toml` declares no feature.
- **FEA-2** (deterministic) `sync` must read the features the project turns off from the `disabled` list of strings of the `[cardinal]` table of `.claude/cardinal.toml`; a project with no `disabled` key turns off no feature.
- **FEA-3** (deterministic) `sync` must handle each file and each entry of a feature the project turns off as one the source does not manage: not copied nor added, removed when the lock recorded it and the project left it unchanged (SYN-31), kept otherwise, and not recorded in the lock.
- **FEA-4** (deterministic) `check` must not report a file or an entry of a feature the project turned off, once a `sync` ran with it turned off.
- **FEA-5** (deterministic) `sync` must fail with the error `<config>: disabled names the unknown feature <name>; the source declares <names>` (exit 3, SYN-37) and write nothing, `--dry-run` included, when `disabled` names a feature the source does not declare.
- **FEA-6** (deterministic) `sync` must fail with an error (exit 3) naming the feature and the path or entry, and write nothing, when `features.toml` names a file absent from `files/` or an entry absent from `entries/.claude/settings.json`.
- **FEA-7** (deterministic) `sync` must fail with an error (exit 3) and write nothing when `features.toml` is not valid TOML in the shape of FEA-1 and FEA-17, or when `disabled` is not a list of strings.
- **FEA-8** (deterministic) `commit-check` and `check-commits` must not report an attribution line when the project turns off the feature `no-ai-attribution`.
- **FEA-9** (deterministic) `check-issue` must not report an attribution line when the project turns off the feature `no-ai-attribution`.
- **FEA-10** (deterministic) `check`, `sync` and `update` must not print the warning of SYN-26 when the project turns off the feature `no-ai-attribution`.
- **FEA-11** (deterministic) `rule-breaches` must not apply rule `commit-attribution` (BRE-9) to a commit whose `.claude/cardinal.toml`, as of that commit, turns off the feature `no-ai-attribution`.
- **FEA-12** (deterministic) The source of Cardinal must declare the feature `no-ai-attribution`, whose files hold the rule against AI attribution in commits.
- **FEA-13** (deterministic) `files/AGENTS.md` must not hold the rule against AI attribution: a project that turns the feature off loads it nowhere.
- **FEA-14** (deterministic) `update` must not refuse a revision for a hook the installed `cardinal` cannot run (BIN-16) when that hook is an entry of a feature the project turns off.
- **FEA-15** (deterministic) `sync` must not seed a path of `seed/` that is also a file of a feature the project turns off.
- **FEA-16** (deterministic) The source of Cardinal must declare the features `lint-gate`, `format-code` and `language-hint`, each made of the one hook entry of `entries/.claude/settings.json` whose command is `cardinal run <feature>`, and no file.
- **FEA-17** (deterministic) `sync` must read the features a feature needs from the optional `needs` list of strings of its table in `features.toml`, and fail with an error (exit 3) naming the feature and the name, writing nothing, when `needs` names a feature `features.toml` does not declare.
- **FEA-18** (deterministic) `sync` must fail with the error `<config>: disabled turns off <needed>, which <feature> needs` (exit 3, SYN-37) and write nothing, `--dry-run` included, when `disabled` turns off a feature that a feature left on needs.
- **FEA-19** (deterministic) The source of Cardinal must keep the recipes of a part a feature may leave out in a managed file of their own, which `files/cardinal.just` imports with `import?`: `cardinal-commits.just` (`check-commit`, `check-commits`), `cardinal-issues.just` (`next-issue`, `close-issue`, `check-issue`), `cardinal-specs.just` (`check-specs`, `spec-coverage`) and `cardinal-review.just` (`waive-review`); `cardinal.just` keeps `open-worktree`, `close-worktree`, `release` and `check-cardinal`, and `just` must load a `justfile` importing `cardinal.just` with any of the four missing.
- **FEA-20** (deterministic) The source of Cardinal must declare the feature `conventional-commits`, whose files hold the commit message convention (Conventional Commits: subject, type, scope, body, footer, style); none of them being `files/AGENTS.md` nor the `SKILL.md` of `skill:commit-message`, which the other skills call to commit.
- **FEA-21** (deterministic) `commit-check` and `check-commits` must report nothing but the attribution lines (FEA-8) and the malformed `Expected-effect:` lines, reading no `commit_scopes`, when the project turns off the feature `conventional-commits`.
- **FEA-22** (deterministic) `rule-breaches` must not apply rule `commit-format` (BRE-9) to a commit whose `.claude/cardinal.toml`, as of that commit, turns off the feature `conventional-commits`.
- **FEA-23** (deterministic) The source of Cardinal must declare the feature `review-gate`, made of the hook entries of `entries/.claude/settings.json` whose command is `cardinal run review-gate check` or `cardinal run review-gate record`, the deny rules of REV-19 and the file `cardinal-review.just` (`waive-review`); turned off, `sync` removes the native gate it installed (SYN-60).
- **FEA-24** (deterministic) Once a `sync` ran with the feature `review-gate` turned off, `check` must not report `missing: native review gate`, `modified: native review gate` nor `conflict: native review gate (...)` (SYN-23).
- **FEA-25** (deterministic) Once a `sync` ran with the feature `review-gate` turned off, `check`, `sync` and `update` must not print the warnings of SYN-24.
- **FEA-26** (deterministic) `rule-breaches` must not apply rules `no-verify`, `stage-everything`, `review-waiver` and `review-before-commit` (BRE-4, BRE-5, BRE-6, BRE-21) to a session whose `.claude/cardinal.toml` committed at its `start` (as BRE-14 reads it) turns off the feature `review-gate`: no finding of theirs, and they apply to no such session.
- **FEA-27** (deterministic) The source of Cardinal must declare the features `harness-measure`, made of the files of `files/.claude/skills/evaluate-harness/` and `files/.claude/skills/audit-instructions/`, `report-style`, made of the file `.claude/rules/reports-to-the-user.md`, and `github-ci`, made of the files of `files/.github/`; none of them with an entry.
- **FEA-28** (deterministic) `check`, `sync` and `update` must not print the warnings of SYN-28 on the user settings when the project turns off the feature `harness-measure`; those on the project's own settings stay, since they sweep the transcripts of every project.
- **FEA-29** (deterministic) The rule that the agent talks to the user in the `user_language` must be in a managed file of no feature, `.claude/rules/user-language.md`, and not in `.claude/rules/reports-to-the-user.md`, which holds only the format of the reports.
- **FEA-30** (deterministic) The source of Cardinal must declare the feature `issue-files`, made of the files of `files/.claude/skills/issue-workflow/`, `files/.claude/skills/create-issue/` and `files/.claude/skills/prioritize-issues/` and the file `cardinal-issues.just` (`next-issue`, `close-issue`, `check-issue`), with no entry and no `needs`.
- **FEA-31** (deterministic) The source of Cardinal must declare the feature `specs`, made of the files of `files/specs/`, of `files/.claude/skills/write-spec/` and the file `cardinal-specs.just` (`check-specs`, `spec-coverage`), with no entry and no `needs`.
- **FEA-32** (deterministic) The managed CI workflow must run `just check-specs` only when the project has `cardinal-specs.just`, and must not fail for its absence.
- **FEA-33** (deterministic) The `check` recipe of the seeded `justfile` must run `check-specs` when the project has `cardinal-specs.just`, and must succeed without it when the file is missing.
- **FEA-34** (deterministic) `skill:issue-workflow` must write a spec, and trace its tests to one, only in a project that has `specs/README.md`, the file the feature `specs` leaves out.
- **FEA-35** (deterministic) The source of Cardinal must declare the feature `workspace-lock`, made of the three hook entries of `entries/.claude/settings.json` whose command is `cardinal run workspace-lock check`, `cardinal run workspace-lock start` or `cardinal run workspace-lock release` (WSL-8), with no file and no `needs`.
- **FEA-36** (deterministic) `cardinal run review-gate` must judge the files under `issues/` as any other file when the project turns off the feature `issue-files`: no commit goes through with no review for touching `issues/` only (REV-9), nor for moving `issues/<stem>.md` to `issues/done/<stem>.md` with the rewrites of `close-issue` (REV-20).
- **FEA-37** (deterministic) `deliverable-size` must count the files under `issues/` as any other file when the project turns off the feature `issue-files`; the lock files stay out (SIZ-4).
- **FEA-38** (deterministic) `rule-breaches` must not apply rule `validation-stop` (BRE-8) to a session whose `.claude/cardinal.toml` committed at its `start` (as BRE-14 reads it) turns off the feature `issue-files`: no finding of it, and it applies to no such session.
- **FEA-39** (deterministic) `review-target` must not exclude `issues/` from the target when the project turns off the feature `issue-files` (RVT-3); the managed files stay out.

## Scenarios

- (FEA-3) Given a project synced with every feature on, when it adds `no-ai-attribution` to `disabled` and runs `sync`, then the files of the feature are reported `removed` and are gone, and the lock no longer records them.
- (FEA-3) Given a project that edited a file of a feature, when it turns the feature off and runs `sync`, then the file is reported `kept` and stays as the project wrote it.
- (FEA-3) Given a project that turned a feature off, when it removes the name from `disabled` and runs `sync`, then the files and entries of the feature are added back.
- (FEA-3) Given a feature whose entry is a hook, when the project turns it off, then `sync` removes the hook object from `.claude/settings.json`, and a group left empty with it (SYN-31).
- (FEA-5) Given `disabled = ["no-ai-atribution"]`, when `sync` runs, then it fails naming the unknown feature and the features the source declares.
- (FEA-18) Given `a` needs `b`, when `disabled = ["b"]`, then `sync` fails naming `b` and `a`; when `disabled = ["a", "b"]`, then it succeeds: only the features left on are judged.
- (FEA-18) Given `a` and `b` need each other, when `disabled = ["a", "b"]`, then `sync` succeeds: a cycle in `needs` is harmless, each feature being judged against the ones left on; when `disabled = ["a"]`, then it fails naming `a` and `b`.
- (FEA-17) Given `a` with `needs = ["a"]`, then `sync` accepts it: a feature needing itself is never turned off while left on.
- (FEA-19) Given a `justfile` importing `cardinal.just` and no `cardinal-specs.just`, when `just --list` runs, then it lists the other recipes and fails on none.
- (FEA-21) Given `disabled = ["conventional-commits"]` and the message `Added the parser.` with a body line of 90 characters, when `commit-check` reads it, then it prints nothing and exits 0; with a `Co-Authored-By:` line, it reports that line only.
- (FEA-8) Given `disabled = ["no-ai-attribution"]` and a message with a co-author trailer, when `commit-check` reads it, then it prints nothing about the trailer and still checks the subject.
- (FEA-23, SYN-60) Given a project that turned `review-gate` off, when it removes the name from `disabled` and runs `sync`, then the hooks, the deny rules, `cardinal-review.just` and the native gate are back, a user's `pre-commit` chained again (SYN-19).
- (FEA-33) Given the seeded `justfile` and `cardinal.just` with no `cardinal-specs.just`, when `just check` runs, then it succeeds; with `cardinal-specs.just`, it runs `check-specs` and fails when it fails.
- (FEA-26) Given a session started after a commit that sets `disabled = ["review-gate"]`, with a `git add -A` and a commit skipping the git hooks, when `rule-breaches` reads it, then it reports no `stage-everything` nor `no-verify` finding and its summary has no row for the four rules.
- (FEA-36) Given `disabled = ["issue-files"]`, a reviewed diff and a new `issues/x.md`, when the native hook judges a commit of `issues/x.md` alone, then it refuses it as holding a file the review never saw.
- (FEA-37) Given `disabled = ["issue-files"]` and changes to `issues/a.md`, `Cargo.lock` and `src/a.rs` of 1 added line each, when `deliverable-size` runs, then `issues/a.md` and `src/a.rs` are listed and the total is 2.
- (FEA-38) Given a session started after a commit that sets `disabled = ["issue-files"]`, which writes a line `Plan: proposed` in `issues/x.md` then edits `src/main.rs`, when `rule-breaches` reads it, then it reports no `validation-stop` finding and its summary has no row for the rule.

## Non-goals

- Turning on or off a needed feature by itself: `sync` refuses the list and leaves the project to complete it.
- A person turning off a feature for themselves in `.claude/cardinal.local.toml`: `disabled` changes the committed
  files of the project, so it is the project's choice (ADR 0018).
- Turning off the core of `cardinal` (`sync`, `check`, the lock, the formats and migrations).
- Rewriting the seeded files of a project whose feature is turned off: the `attribution` of its seeded
  `.claude/settings.json` stays the project's to change.
- Rewriting the `justfile` of a project seeded with `check: check-specs` when it turns `specs` off: the seed is the
  project's (SYN-4), which drops the dependency itself; the seeds of FEA-33 serve the projects seeded after it.
- Checking a project's own commit convention when it turns off `conventional-commits`: `commit-check` and the CI job
  `commits` stay, for the attribution and the `Expected-effect:` footers that the measure reads.
- The commit types that `delivered-quality` reads (QUA-3): a project with its own convention counts no `fix`.
- Declaring the features other than `no-ai-attribution`, `conventional-commits`, the hooks of FEA-16,
  `review-gate`, those of FEA-27, `issue-files`, `specs` and `workspace-lock`: each comes with its own issue.
- Rewording the skills of `harness-measure`, which cite each other: a citation of a file left out reads as absent,
  and nothing loads or checks it.
- Keeping the refusals of `cardinal run review-gate check` (`--no-verify`, `core.hooksPath`, a `git add` of every
  file: REV-12, REV-21) when `review-gate` is off: they go with its hook.
- Rewording the skills that mention the gate (`skill:issue-workflow` keeps its mandatory review): a project that
  turns the gate off reviews as its workflow says, nothing refuses a commit.
- Silencing rule `user-language` of `rule-breaches` when `language-hint` is off: the rule is the user's and stays
  loaded; the feature only drops the hook that enforces it, whose hints then no longer appear (BRE-13).
- Gating the issue tools of the binary when `issue-files` is off: `next-issue`, `close-issue`, `check-issue` and
  `issue-claim` stay callable, and nothing left on calls them; `workspace-lock` reads no claim, and rule
  `skill-routing-issue-workflow` of `rule-breaches` (BRE-17) applies only where `skill:issue-workflow` is synced.
- A lighter workflow skill replacing `skill:issue-workflow` when `issue-files` is off: a project tracking its issues
  elsewhere writes its own.
- Leaving out the seeded `ROADMAP.md` or its guidance when `issue-files` is off: a seed is the project's (SYN-4), and
  `release` still reads its milestones.
- Rewording the skills that cite `skill:create-issue` or `skill:issue-workflow` (`skill:write-spec`,
  `skill:commit-message`, `skill:audit-instructions`): a citation of a skill left out reads as absent.
- Gating the spec tools of the binary when `specs` is off: `check-specs`, `spec-coverage` and the traced ids that
  `delivered-quality` counts stay callable, and nothing left on calls them; rule `skill-routing-write-spec` of
  `rule-breaches` already applies only where `skill:write-spec` is synced.
- Rewording `skill:update-docs` and `skill:create-issue`, whose spec lines apply to a capability that has a spec: a
  project that turns `specs` off and keeps specs of its own traces them its own way.
- Turning off the issue claims with `workspace-lock`: `issue-claim` stays a step of `skill:issue-workflow`, since it
  keeps `next-issue` from offering an issue a session of another worktree works and refuses to start an issue on
  uncommitted changes, at one call per issue; with the lock off, a claim no longer guards a worktree (WSL-10 has no
  lock to hand over).
- Rewording `skill:issue-workflow` and its details, which tell the agent to stop when the worktree lock refuses its
  commands: with the feature off nothing refuses them.
