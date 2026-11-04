---
id: PAR
status: accepted
adrs: [0003, 0008, 0009, 0011, 0018, 0022, 0023]
---

# Parallel issue isolation

## Intent

Two agents drive two different issues at once, each in its own git worktree and branch, so that neither one's
uncommitted files, index or build artifacts leak into the other's work. A worktree is the user's: it serves several
issues in turn and keeps its build. Merging back stays a
deliberate step run from the main branch.

## Requirements

- **PAR-1** (deterministic) `open-worktree <name>` must create a worktree at `<main worktree>-<name>`, next to the main worktree, or at the path given by `--path` (relative to the current directory), on a new branch `<name>`, or the one given by `--branch`, cut from the branch of the main worktree, and print its path; an existing branch checked out in no worktree is checked out as it is.
- **PAR-2** (deterministic) `open-worktree` must fail, changing nothing, when the name is not made of ASCII letters, digits, `.`, `_` and `-` or starts with `-` or `.`, when a new branch is to be cut and the main worktree is detached, when the branch is already checked out in a worktree, when the path already exists, or when the path is inside the main worktree (git would list it there).
- **PAR-3** (deterministic) An issue must count as taken when a worktree has a branch named after its stem checked out, or when such a branch exists with at least one commit that is not on the main branch (a branch whose content is already on the main branch counts as merged, see PAR-6, and takes nothing); a branch named like the main branch must never take an issue; the mark must be derived from git alone, so it is seen from every worktree and needs no file to clean up.
- **PAR-4** (deterministic) `next-issue` must skip the taken issues, print the first free issue of the order among the ready ones (NXT-1, NXT-7), and list the taken ones with the worktree path when there is one; when every entry is taken it must say so. A branch named after an open issue that takes no issue (no worktree, nothing off the main branch) must be listed on a `Stale:` line that names `close-worktree`, so that nothing hides an issue without a visible reason.
- **PAR-5** (deterministic) `next-issue` must keep its current output when no issue is taken, and must work outside a git repository.
- **PAR-6** (deterministic) `close-worktree` must remove the worktree it designates (PAR-19) and delete the branch that worktree has checked out only when that branch is merged into the main branch, that is its tip is an ancestor of it or every commit of the branch has a patch-equivalent on it (`git cherry`), or merging it into the main branch would change no file (a squash of any number of commits, found by comparing trees), and must otherwise fail, keeping both, with a message that names `--force`; a detached worktree has no branch to delete.
- **PAR-10** (deterministic) `close-worktree <target> --force` must remove the worktree and delete its branch even when the branch is not detected as merged (its content is not all on the main branch), yet still refuse a worktree with uncommitted changes; without `--force`, work that is not on the main branch is never deleted.
- **PAR-11** (deterministic) `close-worktree`, with or without `--force`, must fail, removing nothing, when its working directory is the worktree to remove or inside it, with a message that names the main worktree to run it from.
- **PAR-7** (deterministic) `close-worktree` must refuse, removing nothing, a worktree with uncommitted changes, a worktree holding another worktree (one nested in it, which `git status` there does not show), and the main worktree.
- **PAR-8** (deterministic) withdrawn: an issue is closed on the branch that worked it (PAR-18), its claim then waits for the merge (CLM-7).
- **PAR-9** (deterministic) The hooks that use the git directory (the code review gate and its marker and waiver) must resolve it per worktree, so a review recorded in one worktree is not seen by another.
- **PAR-13** (deterministic) withdrawn: worktrees no longer open under `.claude/worktrees/` (ADR 0023); the `info/exclude` line an earlier `start-issue` added is left as it is.
- **PAR-14** (deterministic) `close-issue` (with or without `--relink`) and the spec traceability check must take the files they scan in a git repository from git (the tracked and the untracked files, without the ignored ones), so that they never read or rewrite a file of a worktree nested in the one they scan, whether or not git ignores its directory, while an untracked file of the scanned worktree and a directory named `worktrees` anywhere are still scanned.
- **PAR-16** (deterministic) Outside a git repository, the spec traceability check must scan every `.rs` file of the root it is given that TRC-9 and TRC-10 let it read, those of `.claude/worktrees/` included.
- **PAR-15** (deterministic) `close-worktree <name>` must also remove a worktree an earlier `start-issue` left at `<main worktree>/.claude/worktrees/<name>`, under the same conditions as PAR-6, PAR-7, PAR-10 and PAR-11.
- **PAR-12** (deterministic) `cardinal run format-code` must run `cargo fmt` from the directory of the edited file, so that cargo formats the project holding that file (the main worktree, an issue worktree, a crate in a subdirectory) and not the one of `CLAUDE_PROJECT_DIR`.
- **PAR-18** (deterministic) `close-issue` (with or without `--relink`) must run in any worktree yet refuse to close an issue whose `issues/` directory is in a worktree other than the one it runs in, whatever directory it runs from, changing nothing.
- **PAR-17** (deterministic) `open-worktree` must copy the `.claude/cardinal.local.toml` of the main worktree, when it has one, to the same path of the new worktree, so a person's preferences (ADR 0018) follow them into each worktree.
- **PAR-19** (deterministic) `close-worktree` must act only on what its argument designates: a path, or a name designating, in this order, the worktree at `<main worktree>-<name>`, the one at `<main worktree>/.claude/worktrees/<name>`, the worktree that has the branch `<name>` checked out, then a branch `<name>` checked out nowhere (deleted under PAR-6 and PAR-10); when it designates none of them, or a worktree on another branch while a branch `<name>` also exists (a worktree reused after the branch of `Stale:`), it fails, removing nothing.

## Scenarios

- (PAR-3) Given a branch `foo` with a commit that is not on the main branch and no worktree, when `next-issue` runs, then `foo` is skipped and reported as taken without a path.
- (PAR-4) Given the issues `a`, `b` in that order with `a` taken, when `next-issue` runs, then it prints `b` as next and reports `a` as taken.
- (PAR-6) Given a branch merged by fast-forward or by rebase, when `close-worktree` runs, then it counts as merged since its tip is an ancestor of the main branch.
- (PAR-6) Given a branch merged by a rebase or by a squash of one commit, when `close-worktree` runs, then it counts as merged since every commit has a patch-equivalent on the main branch.
- (PAR-6) Given a branch squash-merged from several commits, when `close-worktree` runs, then it counts as merged since merging it would change nothing on the main branch.
- (PAR-3) Given a branch squash-merged from several commits with no worktree, when `next-issue` runs, then it is not taken and is reported as `Stale:`.
- (PAR-10) Given a branch with a commit whose content is not on the main branch, when `close-worktree` runs without `--force`, then it fails and keeps both; with `--force` it removes both.
- (PAR-6) Given a squashed branch whose content was later changed on the main branch, when `close-worktree` runs, then it counts as not merged (the safe side).
- (PAR-11) Given the shell in a subdirectory of the issue worktree, when `close-worktree` runs, then it fails and the worktree and the branch are kept.
- (PAR-3) Given a branch `foo` with no worktree and no commit off the main branch, when `next-issue` runs, then `foo` is not taken and is reported as `Stale:`.
- (PAR-18) Given the shell in the main worktree, when `close-issue ../repo-foo/issues/foo.md` names the issue of the linked worktree `repo-foo`, then it fails and the issues of `repo-foo` are untouched.
- (PAR-18) Given the shell in the linked worktree `repo-foo`, when `close-issue ../repo/issues/foo.md` names the issue of the main worktree, then it fails and the issues of the main worktree are untouched.
- (PAR-1) Given the main worktree `repo` on `trunk`, when `open-worktree feat` runs, then `../repo-feat` is created on a new branch `feat` cut from `trunk`.
- (PAR-1) Given a merged branch `feat` checked out nowhere, when `open-worktree feat` runs, then `../repo-feat` checks out `feat` as it is.
- (PAR-14) Given an issue worktree under `.claude/worktrees/` whose documents cite `issues/foo.md`, when `close-issue issues/foo.md` runs in the main worktree, then the files of the issue worktree are untouched.
- (PAR-14) Given an issue worktree under `.claude/worktrees/` in a clone whose `info/exclude` lacks the line of PAR-13, when the spec traceability check runs in the main worktree, then it reads none of the files of the issue worktree.
- (PAR-15) Given a worktree `.claude/worktrees/foo` made by an earlier `start-issue` and a merged branch `foo`, when `close-worktree foo` runs, then the worktree and the branch are removed.
- (PAR-19) Given a worktree `../repo-a` reused on a branch `b`, when `close-worktree a` runs and `b` is merged, then the worktree and `b` are removed.

- (PAR-12) Given a session opened in the main worktree that entered the issue worktree, when the agent edits a Rust file there, then `cargo fmt` runs in the issue worktree and the main worktree is untouched.

## Non-goals

- Pushing, opening pull requests or merging: the merge back is the user's decision.
- More than one agent on the same issue, and sharing uncommitted files between worktrees.
- Sharing `target/` between worktrees: each has its own.
