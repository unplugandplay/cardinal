//! The changed lines of a deliverable, or of a whole issue, against the size limit of `skill:issue-workflow`.
//! Usage: `cardinal run deliverable-size [<rev> | <a>..<b>]`: no argument counts the working tree and its untracked
//! files against `HEAD` (SIZ-1), `<rev>` against that revision (SIZ-2), a range its commits alone (SIZ-3). Prints
//! `<lines>\t<path>` per file, largest first, then the total against the limit; exits 0 within it, 1 over it, 3 on an
//! error (SIZ-9, SIZ-10). git does all the counting: the untracked files are marked intent-to-add in a copy of the
//! index, so `git diff` pairs a plain `mv` as a rename and reads their binary attributes (SIZ-5, SIZ-7, SIZ-11).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The lock files, left out with `issues/` (SIZ-4), which counts when the issue files are off (FEA-37).
const LOCK_FILES: &[&str] = &[
    "*.lock",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "go.sum",
];

pub fn main(args: &[String]) -> u8 {
    let target = match args {
        [] => None,
        [target] => Some(target.as_str()),
        _ => {
            eprintln!("usage: deliverable-size [<rev> | <a>..<b>]");
            return crate::NOT_A_REFUSAL;
        }
    };
    match report(target) {
        Ok((report, over)) => {
            print!("{report}");
            u8::from(over)
        }
        Err(e) => {
            eprintln!("deliverable-size: {e}");
            crate::NOT_A_REFUSAL
        }
    }
}

/// The printed report and whether its total exceeds the limit.
fn report(target: Option<&str>) -> Result<(String, bool), String> {
    if let Some(option) = target.filter(|t| t.starts_with('-')) {
        // git would read it as one of its options (SIZ-10)
        return Err(format!("`{option}` is not a revision or a range"));
    }
    let root = super::repo_root()?;
    let root = Path::new(&root);
    let (key, default) = match target {
        None => ("deliverable_size_limit", 1000),
        Some(_) => ("issue_size_limit", 3000),
    };
    let limit = limit(root, key, default)?;
    let numstat = match target {
        Some(range) if range.contains("..") => git(root, &diff_args(range), None, None)?,
        Some(rev) => working_tree_diff(root, rev)?,
        None if git(root, &["rev-parse", "--verify", "-q", "HEAD"], None, None).is_ok() => {
            working_tree_diff(root, "HEAD")?
        }
        None => {
            // no commit yet: everything counts as added (SIZ-1)
            let empty = git(
                root,
                &["hash-object", "-t", "tree", "--stdin"],
                None,
                Some(b""),
            )?;
            working_tree_diff(root, String::from_utf8_lossy(&empty).trim())?
        }
    };
    // with the issue files off, `issues/` counts as any other directory (FEA-37)
    let issue_files = !crate::features::is_off(root, crate::features::ISSUE_FILES);
    let mut files: Vec<(u64, String)> = counts(&numstat)
        .into_iter()
        .filter(|(path, n)| *n > 0 && !left_out(path, issue_files))
        .map(|(path, n)| (n, path))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let total: u64 = files.iter().map(|(n, _)| n).sum();
    let over = total > limit;
    let verdict = if over { "over" } else { "within" };
    let mut out = String::new();
    for (n, path) in &files {
        let _ = writeln!(out, "{n}\t{path}");
    }
    let _ = writeln!(out, "{total} changed lines, limit {limit}: {verdict}");
    Ok((out, over))
}

/// The positive integer `key` of the `[project]` table of the configuration, `default` when the file or the key is
/// absent (SIZ-8, SIZ-10).
fn limit(root: &Path, key: &str, default: u64) -> Result<u64, String> {
    let path = crate::config_of(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(default),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("invalid {}: {e}", path.display()))?;
    match table.get("project").and_then(|p| p.get(key)) {
        None => Ok(default),
        Some(value) => value
            .as_integer()
            .and_then(|n| u64::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("{key} of {} is not a positive integer", path.display())),
    }
}

/// `git diff --numstat` of `target`, renames detected only when exact, so a renamed and edited file counts its whole
/// old and new content (SIZ-5, SIZ-6).
fn diff_args(target: &str) -> [&str; 7] {
    [
        "diff",
        "--numstat",
        "-z",
        "-M100%",
        "--no-ext-diff",
        target,
        "--",
    ]
}

/// The diff of the working tree against `rev`, the untracked files included as new ones: they are marked
/// intent-to-add in a copy of the index, removed after, so the index of the repository is untouched (SIZ-11).
fn working_tree_diff(root: &Path, rev: &str) -> Result<Vec<u8>, String> {
    let index = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        None,
        None,
    )?;
    let index = PathBuf::from(String::from_utf8_lossy(&index).trim());
    let copy = TempIndex(index.with_extension(format!("cardinal-size-{}", std::process::id())));
    if index.exists() {
        std::fs::copy(&index, &copy.0)
            .map_err(|e| format!("cannot copy {}: {e}", index.display()))?;
    }
    let untracked: Vec<u8> = git(
        root,
        &["ls-files", "-o", "--exclude-standard", "-z"],
        None,
        None,
    )?
    .split(|b| *b == 0)
    // a nested repository is listed as its directory: never counted, as `repo_files` drops it
    .filter(|p| !p.is_empty() && !p.ends_with(b"/"))
    .flat_map(|p| p.iter().copied().chain([0]))
    .collect();
    if !untracked.is_empty() {
        git(
            root,
            &[
                "--literal-pathspecs",
                "add",
                "--intent-to-add",
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ],
            Some(&copy.0),
            Some(&untracked),
        )?;
    }
    git(root, &diff_args(rev), Some(&copy.0), None)
}

/// A copy of the index, deleted when dropped.
struct TempIndex(PathBuf);

impl Drop for TempIndex {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The added plus removed lines of each path of a `git diff --numstat -z`; a binary file, `-` for both, counts 0
/// (SIZ-7).
fn counts(numstat: &[u8]) -> BTreeMap<String, u64> {
    let text = String::from_utf8_lossy(numstat);
    let mut counts = BTreeMap::new();
    let mut fields = text.split('\0');
    while let Some(record) = fields.next() {
        let mut parts = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let path = if path.is_empty() {
            // an exact rename: its old path, then its new one
            fields.next();
            fields.next().unwrap_or_default()
        } else {
            path
        };
        let n = added.parse::<u64>().unwrap_or(0) + removed.parse::<u64>().unwrap_or(0);
        *counts.entry(path.to_string()).or_default() += n;
    }
    counts
}

/// Whether `path` is left out of the count: a lock file, or a file under `issues/` when `issue_files` is on.
fn left_out(path: &str, issue_files: bool) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    issue_files && path.starts_with("issues/")
        || LOCK_FILES
            .iter()
            .any(|p| super::check_specs::name_matches(p, name))
}

/// The stdout of `git <args>` run in `root`, with `index` as its index and `stdin` as its input, or git's message.
fn git(
    root: &Path,
    args: &[&str],
    index: Option<&Path>,
    stdin: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    use std::io::Write as _;
    use std::process::Stdio;
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(root)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(input)
            .map_err(|e| format!("cannot write to git: {e}"))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}
