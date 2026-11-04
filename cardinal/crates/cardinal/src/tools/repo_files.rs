//! The files of a working tree as git lists them, shared by the scripts that scan a repository (`#[path]` include,
//! ADR 0006): tracked and untracked, without the ignored ones. A worktree nested in the scanned one (ADR 0008) is
//! never listed, whether git ignores its directory or sees an embedded repository there, so no scanner has to know
//! where the worktrees are.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The existing regular files under `dir` and the `paths` given (all of `dir` when empty), from
/// `git ls-files -co --exclude-standard`; `Ok(None)` when `dir` is not in a git work tree, `Err` with git's message
/// when git fails otherwise (a corrupt index, a refused `safe.directory`, no git), which a scanner must not take for
/// "no repository". An embedded repository that git does not ignore (a nested worktree or clone) is listed as its
/// `dir/` alone, and a submodule as its directory: both are dropped, with their content, as are the deleted tracked
/// files and the symbolic links, whose target may be in a nested worktree. A file in a merge conflict, listed once
/// per stage, comes once.
pub fn repo_files(dir: &Path, paths: &[&str]) -> Result<Option<Vec<PathBuf>>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "-z", "--"])
        .args(paths)
        // the message read below is git's English one, whatever the user's locale
        .env("LC_ALL", "C")
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git ls-files could not run in {}: {e}", dir.display()))?;
    if !out.status.success() {
        let message = String::from_utf8_lossy(&out.stderr);
        if message.contains("not a git repository") {
            return Ok(None);
        }
        return Err(format!(
            "git ls-files failed in {}: {}",
            dir.display(),
            message.trim()
        ));
    }
    let mut files: Vec<PathBuf> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty() && !p.ends_with(b"/"))
        .map(|p| dir.join(path_of(p)))
        .filter(|p| p.symlink_metadata().is_ok_and(|m| m.is_file()))
        .collect();
    files.sort();
    files.dedup();
    Ok(Some(files))
}

/// The path git printed, byte for byte where the platform allows it, so a name that is not UTF-8 is still found.
#[cfg(unix)]
pub(super) fn path_of(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(super) fn path_of(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).as_ref())
}
