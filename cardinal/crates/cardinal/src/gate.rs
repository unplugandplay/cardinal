//! The native git `pre-commit` hook that runs the review gate of the project (`specs/sync.md`, SYN-18 to SYN-23).
//!
//! The hook is a small `sh` script that starts `cardinal run review-gate native` (the tool of the revision the project
//! pins, `specs/tool-binaries.md`), then the `pre-commit` the user already had (moved to `pre-commit.user`, never
//! overwritten). Once the source no longer manages the gate, `sync` removes it and moves that hook back (SYN-60).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::Result;

use crate::entries::Entry;
use crate::{is_executable, set_executable, write, Outcome};

/// The command of the hooks that run the review gate: the gate exists only where Cardinal manages such a hook, or the
/// review hook script of an older revision (`LEGACY_HOOK`).
const REVIEW_TOOL: &str = "cardinal run review-gate";
/// The review hook of the revisions before the tools moved into `harness`, among their managed files.
pub const LEGACY_HOOK: &str = ".claude/hooks/require_code_review.rs";
/// Name of the gate in a report.
pub const GATE: &str = "native review gate";
/// The line that tells the installed hook from one of the user, written by every revision; it keeps the old name
/// because an older `cardinal` that took the gate for the user's would move it to `pre-commit.user`, which would then
/// run itself in a loop. The second is read too: two revisions wrote it.
const MARKS: [&str; 2] = [
    "# harness: native review gate",
    "# cardinal: native review gate",
];
/// Where a `pre-commit` of the user goes, and is run from.
const CHAINED: &str = "pre-commit.user";

/// The managed hook. Git runs it from the top of the work tree, which may be of an older revision whose review gate
/// is still the `rust-script` hook. A work tree without either, a machine without the program it needs, or a launcher
/// that cannot run the tool (exit 3, BIN-14) is a warning, not a refusal, so a branch or a machine without Cardinal
/// can still commit (BIN-9).
const SCRIPT: &str = r#"#!/bin/sh
# harness: native review gate (installed by `cardinal sync`, do not edit)
legacy=.claude/hooks/require_code_review.rs
if [ -f "$legacy" ] && command -v rust-script >/dev/null 2>&1; then
    rust-script "$legacy" native || exit $?
elif [ ! -f "$legacy" ] && [ -f .claude/cardinal.toml ] && cardinal help run >/dev/null 2>&1; then
    cardinal run review-gate native
    code=$?
    if [ "$code" -eq 3 ]; then
        echo "cardinal: review gate skipped, cardinal could not run it" >&2
    elif [ "$code" -ne 0 ]; then
        exit "$code"
    fi
else
    echo "cardinal: review gate skipped, its hook or the program to run it (rust-script, or cardinal) is missing" >&2
fi
case "$0" in */*) dir="${0%/*}" ;; *) dir=. ;; esac
user="$dir/pre-commit.user"
if [ -x "$user" ]; then
    exec "$user" "$@"
fi
exit 0
"#;

/// What the hooks directory holds as `pre-commit`.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Missing,
    /// A `pre-commit` that is not ours.
    Foreign,
    /// A `pre-commit` that is not ours, already linked as `pre-commit.user` by a `sync` killed before the gate took
    /// its place: only the gate is left to install.
    Interrupted,
    /// Ours, but edited or no longer executable.
    Modified,
    UpToDate,
    /// `core.hooksPath` sends git to another directory: the gate is not installed there.
    Redirected,
}

/// What `sync` will do about the gate, decided before anything is written.
#[derive(Debug)]
pub enum Plan {
    /// Write the gate over what the hooks directory holds as `pre-commit`.
    Install { hooks: PathBuf, state: State },
    /// Remove ours, the source no longer managing the gate; `chained` when a hook of the user goes back in its place.
    Remove { hooks: PathBuf, chained: bool },
}

/// The output of `git args` run in `dir`, `None` when it fails. The repository is the project's, not the one a hook or an
/// alias that runs `sync` is in.
fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let out = git_raw(dir, args)?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_raw(dir: &Path, args: &[&str]) -> Option<Output> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()
}

/// Whether `core.hooksPath` is set for the repository of `root` (`git config` exits 1 for an unset key).
fn hooks_path_set(root: &Path) -> bool {
    // any other failure (an unreadable config) is treated as set: better no gate than one in the wrong directory
    git_raw(root, &["config", "--get", "core.hooksPath"])
        .is_none_or(|out| out.status.code() != Some(1))
}

/// The hooks directory of the git repository of `root`, `None` when `root` is not the top of one.
fn hooks_dir(root: &Path) -> Option<PathBuf> {
    let git = |args: &[&str]| git_in(root, args);
    let top = fs::canonicalize(git(&["rev-parse", "--show-toplevel"])?).ok()?;
    if top != fs::canonicalize(root).ok()? {
        return None;
    }
    // for a linked worktree this is the hooks directory of the common git directory
    Some(root.join(git(&["rev-parse", "--git-path", "hooks"])?))
}

pub fn state(hooks: &Path) -> State {
    let path = hooks.join("pre-commit");
    match fs::read_to_string(&path) {
        Err(_) if fs::symlink_metadata(&path).is_err() => State::Missing,
        // the mark is the second line, so a hook of the user that only quotes it is not taken for ours
        Ok(text)
            if text
                .lines()
                .nth(1)
                .is_some_and(|l| MARKS.iter().any(|mark| l.starts_with(mark))) =>
        {
            if text == SCRIPT && is_executable(&path) {
                State::UpToDate
            } else {
                State::Modified
            }
        }
        _ if same_file(&path, &hooks.join(CHAINED)) => State::Interrupted,
        _ => State::Foreign,
    }
}

/// Whether Cardinal manages the review gate: a managed hook of `entries` runs it, or the managed `files` hold the
/// review hook of an older revision.
#[must_use]
pub fn manages<'a>(entries: &[Entry], mut files: impl Iterator<Item = &'a String>) -> bool {
    entries.iter().any(|e| e.label().contains(REVIEW_TOOL)) || files.any(|f| f == LEGACY_HOOK)
}

/// The hooks directory of the project and what its `pre-commit` is, `None` when the project is not the top of a git
/// repository. One decision for `sync` and `check`, so they never disagree about where the gate goes.
fn inspect(root: &Path) -> Option<(PathBuf, State)> {
    let hooks = hooks_dir(root)?;
    let state = if hooks_path_set(root) {
        State::Redirected
    } else {
        state(&hooks)
    };
    Some((hooks, state))
}

/// The line `check` prints when the project should have the gate and does not (or not as managed), `None` when it is
/// there, cannot be installed (`core.hooksPath`, no repository) or is not expected. `expected` tells whether the lock
/// records a managed hook of the review gate, whether or not the settings still hold it. A conflict on
/// `pre-commit.user` is named as such: the `sync` that fixes a missing gate refuses it.
#[must_use]
pub fn problem(root: &Path, expected: bool) -> Option<String> {
    if !expected {
        return None;
    }
    let (hooks, state) = inspect(root)?;
    match state {
        // an unreadable hooks directory is left to `sync` to report
        _ if conflicts(&hooks, &state).unwrap_or(false) => Some(format!(
            "conflict: {GATE} ({})",
            merge_by_hand(&hooks.join("pre-commit"), &hooks.join(CHAINED))
        )),
        State::Missing | State::Foreign | State::Interrupted => Some(format!("missing: {GATE}")),
        State::Modified => Some(format!("modified: {GATE}")),
        State::UpToDate | State::Redirected => None,
    }
}

/// Whether `hooks` holds a `pre-commit.user` of any kind, a dangling link included.
fn has_chained(hooks: &Path) -> Result<bool> {
    match fs::symlink_metadata(hooks.join(CHAINED)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Whether installing the gate over `state` would have to move a `pre-commit` of the user onto a `pre-commit.user`
/// that exists: `plan` refuses then, and `check` says so rather than send the user to that `sync`.
fn conflicts(hooks: &Path, state: &State) -> Result<bool> {
    Ok(*state == State::Foreign && has_chained(hooks)?)
}

/// Whether `a` and `b` are links of the same file, a link compared as itself rather than what it reaches. The count of
/// links guards against a file system whose inode numbers collide, where the gate would replace a hook of the user.
#[cfg(unix)]
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (fs::symlink_metadata(a), fs::symlink_metadata(b)) {
        (Ok(a), Ok(b)) => a.nlink() >= 2 && (a.dev(), a.ino()) == (b.dev(), b.ino()),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_a: &Path, _b: &Path) -> bool {
    false
}

/// Decide what to do for the project at `root`, whose source manages a hook of the review gate when `has_review_hook`:
/// install the gate, or remove the one installed before when the source no longer manages it (SYN-60).
///
/// # Errors
/// When a `pre-commit` of the user must move to `pre-commit.user` and that file exists already.
pub fn plan(root: &Path, has_review_hook: bool) -> Result<Option<Plan>> {
    let Some((hooks, state)) = inspect(root) else {
        return Ok(None);
    };
    if !has_review_hook {
        // a gate left in place would refuse every commit, nothing recording a review any more
        if !matches!(state, State::Modified | State::UpToDate) {
            return Ok(None);
        }
        let chained = has_chained(&hooks)?;
        return Ok(Some(Plan::Remove { hooks, chained }));
    }
    if conflicts(&hooks, &state)? {
        return Err(both_exist(&hooks.join("pre-commit"), &hooks.join(CHAINED)));
    }
    Ok(Some(Plan::Install { hooks, state }))
}

impl Plan {
    /// What the report says, in order.
    #[must_use]
    pub fn outcomes(&self) -> Vec<(String, Outcome)> {
        let state = match self {
            Plan::Install { state, .. } => state,
            Plan::Remove { chained, .. } => {
                let mut out = vec![(GATE.to_string(), Outcome::Removed)];
                if *chained {
                    out.push(("pre-commit.user -> pre-commit".to_string(), Outcome::Moved));
                }
                return out;
            }
        };
        let mut out = Vec::new();
        if *state == State::Redirected {
            let why = "not installed: core.hooksPath is set, so git reads its hooks elsewhere; add `cardinal run review-gate native` to your own pre-commit, run from the top of the work tree";
            return vec![(GATE.to_string(), Outcome::Skipped(why))];
        }
        if *state == State::Foreign {
            out.push(("pre-commit -> pre-commit.user".to_string(), Outcome::Moved));
        }
        let outcome = match state {
            State::Missing | State::Foreign | State::Interrupted => Outcome::Added,
            State::Modified => Outcome::Updated,
            State::UpToDate | State::Redirected => Outcome::Unchanged,
        };
        out.push((GATE.to_string(), outcome));
        out
    }

    /// Whether the plan removes the gate.
    #[must_use]
    pub fn removes(&self) -> bool {
        matches!(self, Plan::Remove { .. })
    }

    /// Write the hook, or remove it.
    ///
    /// # Errors
    /// When a file cannot be moved, written or removed.
    pub fn apply(&self) -> Result<()> {
        match self {
            Plan::Install { hooks, state } => install(hooks, state),
            Plan::Remove { hooks, chained } => uninstall(hooks, *chained),
        }
    }
}

fn install(hooks: &Path, state: &State) -> Result<()> {
    if matches!(state, State::UpToDate | State::Redirected) {
        return Ok(());
    }
    let hook = hooks.join("pre-commit");
    // the new hook is complete before the hook of the user moves, so a failure never leaves the repository
    // with no hook at all, and it goes on any failure, so the hooks directory is left as found
    let staged = hooks.join("pre-commit.cardinal-new");
    swap(&hook, &staged, state).inspect_err(|_| {
        let _ = fs::remove_file(&staged);
    })
}

fn swap(hook: &Path, staged: &Path, state: &State) -> Result<()> {
    write(staged, SCRIPT.as_bytes())?;
    set_executable(staged, true)?;
    match state {
        // a `pre-commit` written since `plan` is the user's: never replace it
        State::Missing if link_new(staged, hook)? => {
            // the gate is in place: a staged name left behind fails nothing
            let _ = fs::remove_file(staged);
        }
        State::Foreign | State::Interrupted => {
            let chained = hook.with_file_name(CHAINED);
            // checked again here: a `pre-commit.user` replaced since `plan` makes the link refuse below
            if *state == State::Interrupted && same_file(hook, &chained) {
                fs::rename(staged, hook)?;
            } else if link_new(hook, &chained)? {
                // the user's hook is kept as `pre-commit.user`, so one rename replaces it with the gate
                fs::rename(staged, hook).inspect_err(|_| {
                    let _ = fs::remove_file(&chained);
                })?;
            } else {
                fs::rename(hook, &chained)?;
                fs::rename(staged, hook).inspect_err(|_| {
                    let _ = fs::rename(&chained, hook);
                })?;
            }
        }
        // a rename replaces a link, never writes through it
        _ => fs::rename(staged, hook)?,
    }
    Ok(())
}

/// Link `from` as `to`, a link of the directory itself rather than of what it reaches, so a link stays a link, its
/// relative target unchanged in the same directory, a dangling one included. `false` when the file system makes no
/// hard links: the caller renames instead, `to` checked absent just before.
///
/// # Errors
/// When `to` exists: a rename would replace it.
fn link_new(from: &Path, to: &Path) -> Result<bool> {
    let exists = |e: std::io::Error| match e.kind() {
        std::io::ErrorKind::AlreadyExists => Err(both_exist(from, to)),
        _ if fs::symlink_metadata(to).is_ok() => Err(both_exist(from, to)),
        _ => Ok(false),
    };
    fs::hard_link(from, to).map_or_else(exists, |()| Ok(true))
}

fn both_exist(hook: &Path, chained: &Path) -> anyhow::Error {
    anyhow::anyhow!(merge_by_hand(hook, chained))
}

/// What `sync` refuses and `check` reports when a hook of the user would move onto a `pre-commit.user` that exists.
fn merge_by_hand(hook: &Path, chained: &Path) -> String {
    let (hook, chained) = (hook.display(), chained.display());
    format!(
        "{hook} and {chained} both exist: merge them by hand into {chained} (replace it if it is a dangling link), remove {hook}, then run `cardinal sync`, which never overwrites a hook of yours"
    )
}

/// Remove the gate and put the hook of the user it chained back in its place, a link moved as a link.
fn uninstall(hooks: &Path, chained: bool) -> Result<()> {
    let hook = hooks.join("pre-commit");
    if chained {
        // one rename replaces the gate, so the repository is never left without the user's hook
        fs::rename(hooks.join(CHAINED), &hook)?;
    } else {
        fs::remove_file(&hook)?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    // spec: SYN-19 (internal: a `pre-commit.user` created between `plan` and `apply` is a race the binary cannot stage)
    #[test]
    fn the_move_never_replaces_a_chained_hook_and_leaves_the_hooks_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path();
        fs::write(hooks.join("pre-commit"), "user").unwrap();
        fs::write(hooks.join(CHAINED), "kept").unwrap();
        let err = install(hooks, &State::Foreign).unwrap_err().to_string();
        assert!(err.contains("merge them by hand"), "{err}");
        let mut names: Vec<_> = fs::read_dir(hooks)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, ["pre-commit", CHAINED]);
        assert_eq!(
            fs::read_to_string(hooks.join("pre-commit")).unwrap(),
            "user"
        );
        assert_eq!(fs::read_to_string(hooks.join(CHAINED)).unwrap(), "kept");
    }

    // spec: SYN-19 (internal: a `pre-commit` created between `plan` and `apply` is a race the binary cannot stage)
    #[test]
    fn the_install_never_replaces_a_hook_written_since_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path();
        fs::write(hooks.join("pre-commit"), "user").unwrap();
        assert!(install(hooks, &State::Missing).is_err());
        let names: Vec<_> = fs::read_dir(hooks)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["pre-commit"]);
        assert_eq!(
            fs::read_to_string(hooks.join("pre-commit")).unwrap(),
            "user"
        );
    }
}
