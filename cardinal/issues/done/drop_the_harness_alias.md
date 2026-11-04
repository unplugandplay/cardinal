# Drop the harness alias

# Goal
The binary was renamed `cardinal` with a `harness` alias and fallbacks so the consumers synced before the rename kept
running. Once every consumer runs `cardinal`, drop what only served the transition, so the code and the release name
one binary.

# Findings
* The crate installs a second bin, `harness` (`crates/cardinal/src/bin/harness.rs`, BIN-18).
* `release.yml` publishes each commit twice, as `cardinal-<commit>` and `harness-<commit>`;
  `install_cardinal.sh` links `harness` to the installed `cardinal`.
* The git gate runs `cardinal`, else `harness` (BIN-9); `check` and `update` judge the `harness` hooks and count a
  missing `cardinal` as refused when `harness` is there (BIN-15, BIN-16); `gate::REVIEW_TOOLS` recognises the review
  hook under both names (SYN-22, SYN-23).
* The installed `pre-commit` gate still opens with the mark line `# harness: native review gate` (`gate::MARK`),
  seen in the gate of both consumers after their migration on 2026-10-29.

# Approach
1. Measure first: list the consumers whose lock still records a `harness run` hook or whose machine has no
   `cardinal`; drop nothing while one is left.
2. Remove the `harness` bin, the `harness-<commit>` publication and link, and the `harness` fallbacks of the gate,
   `check`, `update` and `REVIEW_TOOLS`, with their tests, and the recipe `check-harness` of `cardinal.just`
   (renamed `check-cardinal`).
   Write the gate's mark line `# cardinal: native review gate`, still recognising the old one as the managed gate.
3. Keep reading the `harness-<commit>` releases of the commits from before the rename (BIN-3) and the `harness run`
   refusals of old transcripts (EVA-5): old pins and old sessions stay readable.

# Specs
* `specs/tool-binaries.md`: BIN-18 withdrawn, BIN-9, BIN-15, BIN-16, BIN-17 amended; `specs/sync.md`: SYN-18,
  SYN-22, SYN-23 amended.

# Resources
* `issues/done/rename_the_binary_to_cardinal.md` (the transition and its decisions)
* `crates/cardinal/src/gate.rs`, `launcher.rs`, `src/bin/harness.rs`, `.github/workflows/release.yml`,
  `files/.github/install_cardinal.sh`

# Hints
* The cache moved to `<cache>/cardinal`: the old `<cache>/harness` (clones, binaries) can be deleted then.
* A gate already installed calls `harness` when `cardinal` is missing: `sync` reinstalls it with the new script.

# Tracking

Plan: validated

## Understanding
Every consumer now runs `cardinal` (measured 2026-10-29: both locks are `cardinal.lock`, all hooks `cardinal run`,
migrations committed). Drop the `harness` bin, its release and CI link, and the `harness` fallbacks of the gate,
`check`, `update` and the review hook detection. Out of scope: the `harness-<commit>` releases and builds of old
commits (BIN-3, BIN-5), the old transcripts and commits (EVA-5, BRE-9), the old file names (next issue).

## Plan
1. [x] Drop the `harness` binary: remove `src/bin/harness.rs` and its `[[bin]]`, the `harness-<commit>` publication
   of `release.yml`, the `harness` links of `install_cardinal.sh` (keeping its fallback to the `harness` release and
   build of an old commit), the BIN-18 test; verified by `just check` and the install script tests; makes true BIN-18
   (withdrawn). ~80 lines.
2. [x] Drop the `harness` fallbacks: the gate script runs `cardinal` only and writes the mark
   `# cardinal: native review gate` (the old mark still recognised), `launcher::LAUNCHERS` and the "missing
   `cardinal` but `harness` there" refusal go, `gate::REVIEW_TOOLS` keeps `cardinal run review-gate` only, the
   recipe `check-harness` leaves `files/cardinal.just`; tests amended, docs (`docs/usage.md`, `docs/architecture.md`,
   README, CLAUDE.md) updated; verified by the gate, launcher and sync CLI tests; makes true BIN-9, BIN-15, BIN-16,
   BIN-17, SYN-18, SYN-22, SYN-23. ~250 lines.
3. [x] Fixes of the `high` review: the gate keeps writing its mark `# harness: native review gate` and also reads
   `# cardinal: native review gate`, the docs say `update` (not
   `sync`) moves the `harness` hooks, the `install` recipe no longer names the alias; verified by the gate CLI tests;
   makes true SYN-18, SYN-19. ~35 lines.

## Decisions
- Mode: autonomous
- Base commit: 300d92b57383b6a51237a38d947c9b0116e09ab9
- No new ADR: the drop is the end of step 2 of ADR 0012.
- The gate mark stays `# harness: native review gate` (point 7 of the findings reverted): an older `cardinal` reads
  only that mark, would take a `# cardinal:` gate for the user's, move it to `pre-commit.user`, and that copy would
  exec itself in a loop. The `# cardinal:` mark that `fd59f8b` and `9db214f` wrote is read too: the `sync` of this
  repository had already moved such a gate to `pre-commit.user` (removed by hand, identical to the gate of `9db214f`).
- Machine cleanup (old cache, old binary): the commands are given to the user at the end, not run (answer 1 a).

## Deliverable 1
- done: no `harness` bin, `release.yml` publishes `cardinal-<commit>` only, `install_cardinal.sh` links no `harness`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (BIN-18 withdrawn) |
| docs | updated (architecture, usage, README, CLAUDE.md) |
| code review | `low`: no findings |

- red: none (a removal: the BIN-18 half of the BIN-17 test is dropped, no new test)
- Code review: no findings
- not verified: `release.yml` runs on the push to the main branch only.
- decisions needed: none

## Deliverable 2
- done: the gate runs `cardinal` only and writes the mark `# cardinal: native review gate` (the old one still ours),
  `check` and `update` judge the `cardinal` hooks only, the review hook is `cardinal run review-gate` only, no
  `check-harness` recipe; both specs `accepted`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (both `accepted`) |
| docs | updated (usage, architecture) |
| code review | `low`: no findings |

- red: BIN-9, BIN-15, SYN-18, SYN-19, SYN-20, SYN-22, SYN-23: the gate ran `harness` when only it was installed (exit 2
  instead of 0), `check` refused 3 hooks with only `harness` installed instead of none, the gate was written with the
  `# harness:` mark, a `harness run review-gate` hook got a gate (`added: native review gate`).
- Code review: no findings
- not verified: none
- decisions needed: none

## Deliverable 3
- done: the `high` review of `300d92b..9db214f` (9 findings) settled.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | updated (usage, BIN-18, the `install` recipe) |
| code review | `low`: 1 finding, fixed |

- red: SYN-18, SYN-19: the gate was written with the mark `# cardinal: native review gate`, which an older `cardinal` takes
  for the user's hook; then a gate of that mark was moved to `pre-commit.user` (`added`, expected `updated`).
- Fixed, `high` findings: the gate mark (1), `sync` instead of `update` in the docs (5), the `install` recipe comment (7).
- Not fixed, `high` findings: the `harness` hooks unjudged (2) and a `harness run review-gate` hook given no gate (3), by
  design (BIN-15, SYN-22, no consumer left on them); `check-harness` removal (4), no consumer justfile depends on it
  (checked); the old-commit fallbacks of `install_cardinal.sh` (6), reachable for a pin between `e82841f` and `afe9e0f`,
  kept as Approach 3 says; a stale `pre-commit.harness-new` (8), only left by an interrupted older write, harmless;
  `modified: native review gate` before the next `sync` (9), what any change of the gate script does already.
- Code review: 1 findings: a gate of the `# cardinal:` mark taken for the user's (fixed: both marks read).
- not verified: none
- decisions needed: none
