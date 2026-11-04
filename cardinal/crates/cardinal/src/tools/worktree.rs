//! Open and remove the user's git worktrees, each serving issues in turn (ADR 0023).
//! `open <name> [--path <dir>] [--branch <branch>]` creates the worktree `<main worktree>-<name>`, or `<dir>`
//! (relative to the current directory), on a new branch `<name>` (or `<branch>`) cut from the branch of the main
//! worktree, or on that branch as it is when it exists and no worktree has it checked out.
//! `close <name|path> [--force]` removes a worktree and the branch it has checked out once that
//! branch is merged into the branch of the main worktree (ancestor, or every commit has a patch-equivalent there,
//! or a merge into it would change no file, which covers a rebase and a squash of any number of commits); `--force` also
//! accepts a branch whose work is not on it, after the user agreed, and still refuses a worktree with uncommitted
//! changes or holding another worktree, and the main worktree. A name designates `<main worktree>-<name>`, then a
//! worktree the former `start-issue` left under `<main worktree>/.claude/worktrees/`, then the worktree of the branch
//! `<name>`, then that branch checked out nowhere. `open` runs from any worktree; `close` too, except from inside the
//! worktree it would remove (it refuses and names the main worktree, `--force` or not).
//! Usage: `just open-worktree foo`, `just close-worktree foo` (`just close-worktree foo --force`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn git_ok(dir: &Path, args: &[&str]) -> bool {
    git(dir, args).is_ok()
}

/// The main worktree and the branch it has checked out (`None` when detached).
fn main_worktree(cwd: &Path) -> Result<(PathBuf, Option<String>), String> {
    let list = git(cwd, &["worktree", "list", "--porcelain"])
        .map_err(|_| "not inside a git repository".to_string())?;
    let record = list.split("\n\n").next().unwrap_or_default();
    if record.lines().any(|l| l == "bare") {
        return Err("a bare repository has no main worktree".to_string());
    }
    let path = record
        .lines()
        .find_map(|l| l.strip_prefix("worktree "))
        .ok_or("no worktree found")?;
    let branch = record
        .lines()
        .find_map(|l| l.strip_prefix("branch refs/heads/"))
        .map(str::to_string);
    Ok((PathBuf::from(path), branch))
}

/// A worktree of `git worktree list`: its path, the branch it has checked out (`None` when detached) and its HEAD.
struct Worktree {
    path: PathBuf,
    branch: Option<String>,
    head: Option<String>,
}

/// Every worktree, the main one first.
fn worktrees(cwd: &Path) -> Result<Vec<Worktree>, String> {
    let list = git(cwd, &["worktree", "list", "--porcelain"])?;
    Ok(list
        .split("\n\n")
        .filter_map(|record| {
            let field = |key: &str| record.lines().find_map(|l| l.strip_prefix(key));
            Some(Worktree {
                path: PathBuf::from(field("worktree ")?),
                branch: field("branch refs/heads/").map(str::to_string),
                head: field("HEAD ").map(str::to_string),
            })
        })
        .collect())
}

/// The path of the worktree that has `branch` checked out.
fn worktree_of(all: &[Worktree], branch: &str) -> Option<PathBuf> {
    all.iter()
        .find(|w| w.branch.as_deref() == Some(branch))
        .map(|w| w.path.clone())
}

/// Where the former `start-issue` put the worktree of an issue (ADR 0008), still accepted by `close`.
fn nested(main: &Path, stem: &str) -> PathBuf {
    main.join(WORKTREES).join(stem)
}

/// The default place of a worktree: next to the main one (ADR 0003, ADR 0023).
fn sibling(main: &Path, stem: &str) -> Option<PathBuf> {
    let name = main.file_name()?.to_string_lossy();
    Some(main.with_file_name(format!("{name}-{stem}")))
}

const WORKTREES: &str = ".claude/worktrees";

/// The worktrees nested in `dir`, such as an agent's: `git status` in `dir` does not see them, being ignored.
fn worktrees_inside<'a>(all: &'a [Worktree], dir: &Path) -> Option<&'a Path> {
    all.iter()
        .map(|w| w.path.as_path())
        .find(|w| *w != dir && w.starts_with(dir))
}

/// A name for a path and a branch: ASCII letters, digits, `.`, `_` and `-`, not opening on `-` or `.` (PAR-2).
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['-', '.'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// `path` with its `.` and `..` folded, as the user reads it: the directory does not exist yet.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn open(
    cwd: &Path,
    name: &str,
    path: Option<&str>,
    branch: Option<&str>,
) -> Result<String, String> {
    if !valid_name(name) {
        return Err(format!(
            "{name} is not a worktree name: ASCII letters, digits, `.`, `_` and `-`, not opening on `-` or `.`"
        ));
    }
    let branch = branch.unwrap_or(name);
    if branch.starts_with('-') || !git_ok(cwd, &["check-ref-format", "--branch", branch]) {
        return Err(format!("{branch} is not a valid branch name"));
    }
    let (main, trunk) = main_worktree(cwd)?;
    let dir = match path {
        Some(p) => lexical(
            &std::path::absolute(cwd.join(p)).map_err(|e| format!("cannot resolve {p}: {e}"))?,
        ),
        None => sibling(&main, name).ok_or("the main worktree has no directory name")?,
    };
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    // git would list a worktree nested in the main one there: keep them apart (ADR 0023)
    let canonical_main = std::fs::canonicalize(&main).unwrap_or_else(|_| main.clone());
    if dir.starts_with(&main) || dir.starts_with(&canonical_main) {
        return Err(format!(
            "{} is inside the main worktree {}: open it outside, such as next to it",
            dir.display(),
            main.display()
        ));
    }
    let target = dir.to_string_lossy();
    let exists = git_ok(
        &main,
        &[
            "rev-parse",
            "--verify",
            "-q",
            &format!("refs/heads/{branch}"),
        ],
    );
    if exists {
        if let Some(at) = worktree_of(&worktrees(&main)?, branch) {
            return Err(format!(
                "the branch {branch} is checked out in {}: open that worktree, or name another branch",
                at.display()
            ));
        }
        git(&main, &["worktree", "add", "-q", &target, branch])?;
    } else {
        let base = trunk
            .ok_or("the main worktree has no branch checked out to cut the new branch from")?;
        git(
            &main,
            &["worktree", "add", "-q", "-b", branch, &target, &base],
        )?;
    }
    // git ignores the personal configuration, so the checkout lacks it (PAR-17)
    let personal = main.join(crate::PERSONAL_CONFIG);
    if personal.is_file() {
        let copy = dir.join(crate::PERSONAL_CONFIG);
        std::fs::create_dir_all(copy.parent().unwrap())
            .and_then(|()| std::fs::copy(&personal, &copy))
            .map_err(|e| format!("cannot copy {}: {e}", personal.display()))?;
    }
    Ok(dir.display().to_string())
}

/// What `close` acts on: a worktree (none for a branch checked out nowhere), its branch (none when detached) and the
/// HEAD of the worktree, whose work must be on trunk when it has no branch.
struct Target {
    dir: Option<PathBuf>,
    branch: Option<String>,
    head: Option<String>,
}

impl Target {
    fn of(w: &Worktree) -> Self {
        Target {
            dir: Some(w.path.clone()),
            branch: w.branch.clone(),
            head: w.head.clone(),
        }
    }
}

/// What `arg` designates (PAR-19): a path, or a name, in this order the worktree at `<main>-<name>`, the one at
/// `<main>/.claude/worktrees/<name>`, the worktree that has the branch `<name>` checked out, a branch checked out nowhere.
fn designate(cwd: &Path, main: &Path, all: &[Worktree], arg: &str) -> Result<Target, String> {
    let at = |dir: &Path| {
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        all.iter()
            .find(|w| w.path == dir || std::fs::canonicalize(&w.path).is_ok_and(|w| w == dir))
    };
    let none = || format!("{arg} designates no worktree nor branch: nothing was removed");
    if arg.contains('/') || arg.starts_with('.') {
        let path = lexical(&std::path::absolute(cwd.join(arg)).map_err(|e| e.to_string())?);
        return at(&path).map(Target::of).ok_or_else(none);
    }
    if !valid_name(arg) {
        return Err(none());
    }
    let is_branch = git_ok(
        main,
        &["rev-parse", "--verify", "-q", &format!("refs/heads/{arg}")],
    );
    let places = [sibling(main, arg), Some(nested(main, arg))];
    if let Some(found) = places.iter().flatten().find_map(|dir| at(dir)) {
        // `Stale:` names a branch: a worktree reused on another branch keeps its name, not that branch
        if is_branch && found.branch.as_deref() != Some(arg) {
            return Err(format!("{arg} designates both the worktree {} (on another branch) and the branch {arg}: pass the path of the worktree to remove it, or run `git branch -D {arg}` for the branch; nothing was removed", found.path.display()));
        }
        return Ok(Target::of(found));
    }
    if let Some(found) = all.iter().find(|w| w.branch.as_deref() == Some(arg)) {
        return Ok(Target::of(found));
    }
    if is_branch {
        return Ok(Target {
            dir: None,
            branch: Some(arg.to_string()),
            head: None,
        });
    }
    Err(none())
}

/// Whether `head` is merged into `trunk`: its tip is an ancestor of it, or every commit has a patch-equivalent there,
/// or a merge into it would change no file.
fn merged(main: &Path, head: &str, trunk: &str) -> bool {
    let trunk = format!("refs/heads/{trunk}");
    let trunk = trunk.as_str();
    git_ok(main, &["merge-base", "--is-ancestor", head, trunk])
        // A rebase leaves every commit with a patch-equivalent (`-`) on trunk.
        || git(main, &["cherry", trunk, head])
            .is_ok_and(|c| !c.lines().any(|l| l.starts_with('+')))
        // A squash: merging the branch into trunk would change no file, so the merge tree is trunk's tree
        // (keep in sync with `has_own_commits` of `next_issue.rs`; git before 2.38 lacks `--write-tree` and counts as not merged).
        || {
            let tree = git(main, &["merge-tree", "--write-tree", trunk, head])
                .ok()
                .and_then(|t| t.lines().next().map(str::to_string));
            tree.is_some() && tree == git(main, &["rev-parse", &format!("{trunk}^{{tree}}")]).ok()
        }
}

fn close(cwd: &Path, arg: &str, force: bool) -> Result<String, String> {
    let (main, trunk) = main_worktree(cwd)?;
    let all = worktrees(&main)?;
    let target = designate(cwd, &main, &all, arg)?;
    if let Some(dir) = &target.dir {
        if std::fs::canonicalize(dir).ok() == std::fs::canonicalize(&main).ok() {
            return Err(format!(
                "{arg} designates the main worktree {}: it is never removed",
                main.display()
            ));
        }
        // Removing the directory the caller stands in breaks its shell: refuse, whatever `--force` says.
        let inside = std::fs::canonicalize(cwd)
            .ok()
            .zip(std::fs::canonicalize(dir).ok())
            .is_some_and(|(here, target)| here.starts_with(target));
        if inside {
            return Err(format!("the current directory is inside {}: run close-worktree from the main worktree {}; nothing was removed", dir.display(), main.display()));
        }
    }
    // the branch, or the HEAD of a detached worktree, even one whose directory is gone
    let head = match &target.branch {
        Some(branch) => format!("refs/heads/{branch}"),
        None => target.head.clone().ok_or("the worktree has no HEAD")?,
    };
    let trunk =
        trunk.ok_or("the main worktree has no branch checked out to compare the work with")?;
    if !force && !merged(&main, &head, &trunk) {
        let what = target.branch.as_ref().map_or_else(
            || format!("the detached HEAD {head}"),
            |b| format!("the branch {b}"),
        );
        return Err(format!("{what} is not merged into {trunk}: merge it first, or pass --force to drop it (it deletes work that is not on {trunk}; squash detection needs git 2.38); nothing was removed"));
    }
    match &target.dir {
        Some(dir) if !dir.exists() => {
            git(&main, &["worktree", "prune"])?;
        }
        Some(dir) => {
            if !git(dir, &["status", "--porcelain"])?.is_empty() {
                return Err(format!(
                    "{} has uncommitted changes: nothing was removed",
                    dir.display()
                ));
            }
            if let Some(inner) = worktrees_inside(&all, dir) {
                return Err(format!(
                    "{} holds the worktree {}: remove that one first; nothing was removed",
                    dir.display(),
                    inner.display()
                ));
            }
            git(&main, &["worktree", "remove", &dir.to_string_lossy()])?;
        }
        None => {}
    }
    let mut removed = Vec::new();
    if let Some(dir) = &target.dir {
        removed.push(format!("the worktree {}", dir.display()));
    }
    if let Some(branch) = &target.branch {
        // `-D`: the merge was checked above, or forced; `-d` would also weigh an upstream, after the worktree is gone
        git(&main, &["branch", "-D", branch])?;
        removed.push(format!("the branch {branch}"));
    }
    Ok(format!("removed {}", removed.join(" and ")))
}

const USAGE: &str =
    "usage: cardinal run worktree open <name> [--path <dir>] [--branch <branch>] | close <name|path> [--force]";

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    let result = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["open", rest @ ..] if !rest.is_empty() => {
            let (mut path, mut branch, mut name) = (None, None, None);
            let mut it = rest.iter();
            let mut bad = false;
            while let Some(a) = it.next() {
                match *a {
                    "--path" => {
                        path = it.next().copied();
                        bad |= path.is_none();
                    }
                    "--branch" => {
                        branch = it.next().copied();
                        bad |= branch.is_none();
                    }
                    n if name.is_none() && !n.starts_with("--") => name = Some(n),
                    _ => bad = true,
                }
            }
            match name {
                Some(n) if !bad => open(Path::new("."), n, path, branch),
                _ => Err(USAGE.to_string()),
            }
        }
        ["close", target] => close(Path::new("."), target, false),
        ["close", target, "--force"] | ["close", "--force", target] => {
            close(Path::new("."), target, true)
        }
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
