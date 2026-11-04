---
id: BIN
status: accepted
adrs: [0002, 0010]
---

# Prebuilt tools of Cardinal

## Intent

The hooks, checks and transcript readers of Cardinal run in every consumer, Rust or not. They ship as the
`cardinal` binary of the pinned revision, fetched prebuilt, so a project needs only `cardinal` and `just`, and runs the
tools of the revision its managed files come from. The binary was named `harness` before: the releases and the builds of the commits
from before the rename keep that name (BIN-3, BIN-5).

## Requirements

- **BIN-1** (deterministic) `cardinal run <tool> [args]` must run the tool of the revision selected by the configuration nearest the working directory (`.claude/cardinal.toml`) (else nearest `CLAUDE_PROJECT_DIR`, when the agent left the project), with the arguments, standard input and working directory it was given, its exit code, standard output and error passed through unchanged.
- **BIN-2** (deterministic) With a git source, `run` must execute the binary cached for the pinned commit and the current platform (`<cache>/cardinal/bin/<commit>/<target>/cardinal`, `<cache>` being `XDG_CACHE_HOME`, else `~/.cache`) without starting any download or build when that binary is present.
- **BIN-3** (deterministic) When that binary is missing, `run`, `sync` and `update` must download with `curl` the asset `cardinal-<target>` of the release `cardinal-<commit>` of a GitHub source and its digest `cardinal-<target>.sha256`, else, when that release has no such asset (a commit from before the rename), the asset `harness-<target>` of the release `harness-<commit>` and its digest `harness-<target>.sha256`, and install the binary only when its SHA-256 matches the digest.
- **BIN-4** (deterministic) The binary must appear in the cache by a rename of a complete file, so that a tool run concurrently never executes a partly written binary.
- **BIN-5** (deterministic) When the download fails (no release, no asset for the platform, a source not on GitHub, no network) and `cargo` is on the path, the fetch must build the binary with `cargo install --git <source> --rev <commit> --locked` naming no crate (a scp-like `[user@]host:path` source written `ssh://[user@]host/path`, the form cargo reads) into a temporary root and install as BIN-4 says the `cardinal` it built, else its `harness` (a commit from before the rename).
- **BIN-6** (deterministic) With a local directory source, `run` must build the `cardinal` crate of that directory with `cargo` and run the result, so the tools of the repository that develops them are those of its working tree.
- **BIN-7** (deterministic) `run` with no tool or an unknown tool must exit 3, as BIN-14, and print the names of the tools on stderr, so that a hook command naming a tool the pinned binary does not know (mistyped, or from a newer revision) never blocks the agent.
- **BIN-8** (deterministic) The hook commands of the source's `entries/.claude/settings.json`, the recipes of the `.just` files of `files/`, the commands the skills of `files/` give and the git `pre-commit` gate `sync` installs must call the tools through `cardinal run`, never through `harness run` or `rust-script`, except the gate in a work tree that still holds the review hook script of an older revision (SYN-18).
- **BIN-9** (deterministic) The `pre-commit` gate must run the review gate through `cardinal` when it is on the path, and print a warning on stderr and let the commit through when it is not, as it does when the review hook is missing.
- **BIN-10** (deterministic) A downloaded asset whose SHA-256 differs from its digest must fail the fetch, naming the commit, and leave no binary in the cache.
- **BIN-11** (deterministic) When the download fails and `cargo` is not on the path or its build fails, the fetch must fail with a message naming the commit, the platform and `cardinal update`, leaving no binary in the cache.
- **BIN-12** (deterministic) With a local directory source whose build fails, `run` must run the last binary built from that source, with a one-line warning on stderr, and exit 3 (BIN-14) with the build's errors only when there is none, so that a working tree that does not compile does not block every hook.
- **BIN-13** (deterministic) `sync` and `update` must print a fetch that fails as a warning on stderr, without failing because of it, since `run` fetches again; under `--dry-run` they fetch nothing.
- **BIN-14** (deterministic) When `run` cannot get the binary of the tool (no configuration, a fetch or a build that fails), it must exit 3 with the reason on stderr, a code Claude Code does not read as a refusal (it blocks on 2), so that a hook that cannot run never blocks the agent.
- **BIN-15** (deterministic) `check` must fail with one line per hook command of the project's `.claude/settings.json` whose first word is `cardinal` and whose second word is a subcommand the `cardinal` on the path refuses (`cardinal help <subcommand>` exits non-zero), the line naming the command and the install of the project's `cardinal` (BIN-20), and look at nothing when no `cardinal` is on the path, as BIN-9.
- **BIN-16** (deterministic) `update` must write nothing and fail when a hook command of the managed entries of the target revision names a subcommand the `cardinal` on the path refuses (as BIN-15 judges it), naming each command and, once, the install of the target (BIN-20), except under `--dry-run`, which prints the same lines as warnings without failing.
- **BIN-17** (deterministic) When the `cardinal` launcher cannot parse its arguments (an unknown subcommand, an unknown flag, a missing argument), it must print the parser's message on stderr and exit 3, as BIN-14, so that a hook command the installed launcher cannot parse never blocks the agent, whatever path brought it to the project; `--help` and `--version` keep exit 0. A tool the pinned binary does not know is BIN-7's.
- **BIN-18** (deterministic) withdrawn: the install of the source no longer puts a `harness` alias next to `cardinal`; every consumer runs `cardinal`, and a project whose hooks still call `harness` moves them with one `update`.
- **BIN-19** (deterministic) `cardinal upgrade <source> <commit>` must get the binary of the full commit hash `<commit>` of the git source `<source>` as `run` does (BIN-2, BIN-3, BIN-5, BIN-10), with no configuration needed, then replace the first `cardinal` of the `PATH` (a symlink replaced, not its target; the running `cardinal` when the `PATH` has none) with it by the rename of a complete file of its directory (as BIN-4) and print the path it replaced.
- **BIN-20** (deterministic) The install of a `cardinal` that the errors and warnings of `check`, `sync`, `update` and `init` name (BIN-15, BIN-16, SYN-40, SYN-42) must be `cardinal upgrade <source> <commit>` (BIN-19) for a git source at a full commit hash, `cargo install --force --git <source> --rev <rev> --locked` for a git source at another revision (none when it has none), and `cargo install --force --path <source>/crates/cardinal --locked` for a local source: `--force` reinstalls the same version.
- **BIN-21** (deterministic) `cardinal upgrade` with a `<commit>` that is not a full hash, a `<source>` that is not a git source, or a fetch that fails (BIN-11) must exit non-zero with the reason on stderr and leave the `cardinal` it would replace unchanged.

## Scenarios

- (BIN-2) Given a project pinned to commit C whose binary is cached, when a hook runs `cardinal run review-gate check`
  with no network, then the cached binary runs and its exit code is the hook's.
- (BIN-10) Given a published digest that does not match the downloaded asset, when `sync` fetches it, then `sync` fails
  naming the commit and the cache holds no binary for C.
- (BIN-4) Given two hooks that start at once with no cached binary, when both fetch, then each runs a complete binary.
- (BIN-5) Given a commit pinned before its release was published (`cardinal update` right after a push), when `cargo`
  is on the path, then the binary is built from the source instead.
- (BIN-14) Given an agent that ran `cd /tmp`, when a hook runs `cardinal run review-gate check`, then the tool of the
  project of `CLAUDE_PROJECT_DIR` runs; with no project at all, the hook exits 3 and the command goes on.
- (BIN-12) Given this repository with a syntax error in the crate, when a hook runs, then the tools of the last
  successful build run and stderr says the build failed.
- (BIN-6) Given this repository (`source = "."`) with a change to a tool in its working tree, when a hook runs it,
  then the changed tool runs.
- (BIN-15) Given a branch whose settings gained the hook `cardinal newcmd`, with an installed `cardinal` that has no
  `newcmd`, when `cardinal check` of the branch's code runs in its worktree before the merge, then it fails naming the
  hook; once the branch's `cardinal` is installed, it passes.
- (BIN-16) Given an installed `cardinal` that has no `newcmd` and a target revision whose hooks call `cardinal newcmd`,
  when `update` runs, then nothing is written and the error names `cardinal upgrade <source> <commit>`; after
  that install, the same `update` succeeds.
- (BIN-3) Given a project pinned to a commit from before the rename, when `cardinal run` fetches it, then it installs
  the asset of the release `harness-<commit>`.
- (BIN-17) Given an installed `cardinal` with no `newcmd` and a hook `cardinal newcmd` written in
  `settings.local.json`, when the hook runs, then it exits 3 with clap's message and the agent's command goes on.

## Non-goals

- Windows: the hooks and recipes run under `sh`.
- Running the tools of another revision than the pinned one, or without a configuration.
- Signing the binaries: the digest proves the asset is the one published with the release, not who published it.
- The release workflow itself: it runs on the source's CI and is checked by its own run, not by a test.
- Checking the words after the subcommand (the tool of `cardinal run`): the pinned binary runs them (BIN-1), not the
  installed one.
- Hook commands that do not start with `cardinal`, such as the `harness` hooks of a project synced before the rename
  (they fail as a missing command, which never blocks the agent, until `update` moves them to `cardinal`), and an
  installed `cardinal` older than BIN-15 and BIN-16: it cannot refuse what it does not check.
