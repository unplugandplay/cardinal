//! Move a finished issue file into `issues/done/` (only run after the user agreed).
//! Uses `git mv` when the file is tracked, a plain rename otherwise. Refuses to overwrite.
//! Then rewrites every `issues/foo.md` mentioned in the other text files of the repo to
//! `issues/done/foo.md`, and every Markdown link that resolves to the issue from the directory of its file
//! (`../issues/foo.md` from `docs/`), so no document keeps pointing at the old path; the relative links of the moved
//! issue are rebased so they still resolve from `issues/done/`. The files are those git lists (tracked and
//! untracked, not ignored), so the issue worktrees nested under `.claude/worktrees/` keep their own files.
//! Runs in any worktree, but refuses to close an issue of another worktree named by path. Frees the claim of the
//! issue (`issue_claim.rs`): deleted on the trunk, `merge pending` on another branch until the merge brings the
//! close to the trunk. Sets `done` the milestones of the roadmap whose last open issue it closed (CLS-8). Ends on the
//! issue `next-issue` then offers.
//! Usage: `just close-issue issues/foo.md`; `just close-issue --relink` fixes the references to
//! every issue already in `issues/done/` (run it once to repair old dangling paths).

use super::{issue_claim, repo_files, roadmap};

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

const SKIPPED_DIRS: [&str; 3] = [".git", "target", "node_modules"];

/// Strings of `key = ["a", "b"]` in the `[project]` table of the configuration (`crate::config_of`); empty when absent.
fn config_list(root: &std::path::Path, key: &str) -> Vec<String> {
    let text = std::fs::read_to_string(crate::config_of(root)).unwrap_or_default();
    let table: toml::Table = text.parse().unwrap_or_default();
    let list = table
        .get("project")
        .and_then(|p| p.get(key))
        .or_else(|| table.get(key));
    list.and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_text_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "justfile")
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["md", "rs", "toml", "yaml", "yml", "txt", "json"].contains(&e))
}

/// The names of the directories whose files `close-issue` never rewrites in the repository at `root`.
fn skipped_dirs(root: &Path) -> Vec<String> {
    let mut skipped: Vec<String> = SKIPPED_DIRS.iter().map(|s| (*s).to_string()).collect();
    skipped.extend(config_list(root, "skipped_dirs"));
    skipped
}

/// Whether `skipped` names a directory of `path` (relative to the root).
fn in_skipped_dir(path: &Path, skipped: &[String]) -> bool {
    path.parent()
        .into_iter()
        .flat_map(Path::components)
        .any(|c| skipped.iter().any(|s| c.as_os_str() == s.as_str()))
}

/// Whether `close-issue` rewrites the references of the file `path` (relative to the repository at `root`): the commit
/// gate accepts a rewrite only where the tool makes one (`review_gate`, REV-20).
pub(super) fn rewrites(root: &Path, path: &Path) -> bool {
    is_text_file(path) && !in_skipped_dir(path, &skipped_dirs(root))
}

/// The text files of the work tree at `root`, as git lists them (so never those of a nested worktree, ADR 0008),
/// except under a directory named like one of `skipped`. Exits with an error when git cannot list them, since the
/// references would stay dangling.
fn text_files(root: &Path, skipped: &[String]) -> Vec<std::path::PathBuf> {
    let skipped_dir =
        |path: &Path| in_skipped_dir(path.strip_prefix(root).unwrap_or(path), skipped);
    let files = match repo_files::repo_files(root, &[]) {
        Ok(Some(files)) => files,
        Ok(None) => vec![],
        Err(e) => {
            eprintln!(
                "{e}\nreferences were not updated: fix git, then run `just close-issue --relink`"
            );
            std::process::exit(1);
        }
    };
    files
        .into_iter()
        .filter(|p| is_text_file(p) && !skipped_dir(p))
        .collect()
}

/// Insert `done/` in `issues/<stem>.md` for the given stems, only where `issues/` starts a path
/// (not `my_issues/` nor `docs/issues/`).
fn relink_text(text: &str, stems: &HashSet<String>) -> String {
    const MARK: &str = "issues/";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(MARK) {
        out.push_str(&rest[..i + MARK.len()]);
        let boundary = out[..out.len() - MARK.len()]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || "`\"'([<{=,:".contains(c));
        rest = &rest[i + MARK.len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if boundary && rest[name.len()..].starts_with(".md") && stems.contains(&name) {
            out.push_str("done/");
        }
    }
    out.push_str(rest);
    out
}

/// A path relative to the repository root, as its components.
type RepoPath = Vec<String>;

fn components(path: &Path) -> RepoPath {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// `target` (a `/`-separated relative path) resolved against the directory `dir`; `None` when it climbs above the root.
fn resolve(dir: &[String], target: &str) -> Option<RepoPath> {
    let mut parts = dir.to_vec();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part.to_string()),
        }
    }
    Some(parts)
}

/// The relative path from the directory `from` to `to`; `.` when they are the same.
fn relative(from: &[String], to: &[String]) -> String {
    let common = from.iter().zip(to).take_while(|(a, b)| a == b).count();
    let mut parts = vec![".."; from.len() - common];
    parts.extend(to[common..].iter().map(String::as_str));
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

/// The new target of a link written from `old_dir` and read from `new_dir`, when the file it designates moved
/// (`moves`) or the file holding the link did; `None` when it stays as written (URLs, absolute paths and anchors
/// alone included).
fn relink_target(
    target: &str,
    old_dir: &[String],
    new_dir: &[String],
    moves: &HashMap<RepoPath, RepoPath>,
) -> Option<String> {
    if target.starts_with(['/', '#']) || target.contains(':') {
        return None;
    }
    let (path, anchor) = target.split_at(target.find(['#', '?']).unwrap_or(target.len()));
    if path.is_empty() {
        return None;
    }
    let Some(resolved) = resolve(old_dir, path) else {
        // above the root: only the move of the file holding the link changes it, one `../` per level gone down
        let deeper = new_dir.strip_prefix(old_dir).filter(|d| !d.is_empty())?;
        return Some("../".repeat(deeper.len()) + target);
    };
    let dest = moves.get(&resolved).unwrap_or(&resolved);
    let slash = if path.ends_with('/') { "/" } else { "" };
    (dest != &resolved || old_dir != new_dir).then(|| relative(new_dir, dest) + slash + anchor)
}

/// The range of the link target starting at `start` of `line`, inside its `<...>` when it has them.
fn target_span(line: &str, start: usize) -> std::ops::Range<usize> {
    if let Some(rest) = line[start..].strip_prefix('<') {
        start + 1..start + 1 + rest.find('>').unwrap_or(0)
    } else {
        let len = line[start..]
            .find(|c: char| c.is_whitespace() || c == ')')
            .unwrap_or(line.len() - start);
        start..start + len
    }
}

/// Byte ranges of `line` inside inline code spans (a run of backticks up to the next run of the same length).
fn code_spans(line: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = vec![];
    let mut from = 0;
    while let Some(i) = line[from..].find('`') {
        let open = from + i;
        let ticks = line[open..].len() - line[open..].trim_start_matches('`').len();
        let fence = &line[open..open + ticks];
        let mut search = open + ticks;
        let close = loop {
            match line[search..].find(fence) {
                Some(j) => {
                    let at = search + j;
                    let run = line[at..].len() - line[at..].trim_start_matches('`').len();
                    if run == ticks {
                        break Some(at);
                    }
                    search = at + run;
                }
                None => break None,
            }
        };
        match close {
            Some(close) => {
                spans.push(open..close + ticks);
                from = close + ticks;
            }
            None => from = open + ticks,
        }
    }
    spans
}

/// Byte ranges of the link targets of a Markdown text: inline `[text](target)` and reference `[label]: target`,
/// outside code spans, fenced code blocks and footnotes (CLS-6).
fn link_targets(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = vec![];
    let mut offset = 0;
    let mut fence = None;
    for line in text.split_inclusive('\n') {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = &line[indent..];
        let marker = roadmap::fence_marker(line);
        fence = roadmap::next_fence(fence, line);
        if fence.is_some() || marker.is_some() {
            offset += line.len();
            continue;
        }
        let code = code_spans(line);
        let mut found = vec![];
        let label = body
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("]:"));
        if let Some((label, after)) = label
            .filter(|(label, _)| indent <= 3 && !label.contains(']') && !label.starts_with('^'))
        {
            let start = indent + 1 + label.len() + 2 + (after.len() - after.trim_start().len());
            found.push(target_span(line, start));
        }
        let mut from = 0;
        while let Some(i) = line[from..].find("](") {
            let start = from + i + 2;
            found.push(target_span(line, start));
            from = start;
        }
        spans.extend(
            found
                .into_iter()
                .filter(|s| !s.is_empty() && !code.iter().any(|c| c.contains(&s.start)))
                .map(|s| offset + s.start..offset + s.end),
        );
        offset += line.len();
    }
    spans.sort_by_key(|s| s.start);
    spans
}

/// Rewrite the Markdown link targets of `text`, written from `old_dir` and read from `new_dir` (CLS-2, CLS-4).
fn relink_links(
    text: &str,
    old_dir: &[String],
    new_dir: &[String],
    moves: &HashMap<RepoPath, RepoPath>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for span in link_targets(text) {
        if span.start < last {
            continue;
        }
        if let Some(target) = relink_target(&text[span.clone()], old_dir, new_dir, moves) {
            out.push_str(&text[last..span.start]);
            out.push_str(&target);
            last = span.end;
        }
    }
    out.push_str(&text[last..]);
    out
}

/// The text of the file `path` (components from the root) once the references to the closed issues `stems` of the
/// `issues/` directory `issues` are rewritten (CLS-1, CLS-2); `moved` when it is the issue just moved to
/// `issues/done/`, whose own links are rebased (CLS-4). The commit gate replays it (`review_gate`, REV-20).
pub(super) fn rewritten(
    path: &[String],
    text: &str,
    issues: &[String],
    stems: &HashSet<String>,
    moved: bool,
) -> String {
    if !moved && !stems.iter().any(|stem| text.contains(stem.as_str())) {
        return text.to_string();
    }
    let in_issues = |parts: &[&str]| -> RepoPath {
        let mut path = issues.to_vec();
        path.extend(parts.iter().map(|p| (*p).to_string()));
        path
    };
    let relocations: HashMap<RepoPath, RepoPath> = stems
        .iter()
        .map(|stem| {
            let name = format!("{stem}.md");
            (in_issues(&[&name]), in_issues(&["done", &name]))
        })
        .collect();
    let dir = &path[..path.len().saturating_sub(1)];
    if path
        .last()
        .is_some_and(|name| Path::new(name).extension().is_some_and(|e| e == "md"))
    {
        let old_dir = if moved { issues } else { dir };
        relink_text(&relink_links(text, old_dir, dir, &relocations), stems)
    } else {
        relink_text(text, stems)
    }
}

/// The roadmap `text` of the document `path` (components from the root) with `done` on the `State:` line of each
/// milestone (`roadmap`) that is `in issues`, whose `Issues:` list links one of the `closed` issues of
/// `issues/done/` and whose every link to the `issues` directory designates a file of `issues/done/` that `exists`
/// (CLS-8); and the names of
/// those milestones. The commit gate replays it (`review_gate`, REV-20).
pub(super) fn milestones_done(
    path: &[String],
    text: &str,
    issues: &[String],
    closed: &HashSet<String>,
    exists: impl Fn(&[String]) -> bool,
) -> (String, Vec<String>) {
    let dir = &path[..path.len().saturating_sub(1)];
    let mut done_dir = issues.to_vec();
    done_dir.push("done".to_string());
    // the stem of a file directly under `issues/done/`
    let done_stem = |p: &RepoPath| match p.strip_prefix(done_dir.as_slice()) {
        Some([name]) => name.strip_suffix(".md").map(str::to_string),
        _ => None,
    };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut set = Vec::new();
    for milestone in roadmap::milestones(&lines) {
        let list = milestone.issues(&lines);
        // the links of the list to the issues directory; a URL, an anchor or another document is no issue
        let listed: Vec<RepoPath> = link_targets(&list)
            .into_iter()
            .filter_map(|span| {
                let target = &list[span];
                if target.starts_with(['/', '#']) || target.contains(':') {
                    return None;
                }
                resolve(
                    dir,
                    &target[..target.find(['#', '?']).unwrap_or(target.len())],
                )
            })
            .filter(|p| p.starts_with(issues))
            .collect();
        let state = milestone.state_line(&lines, "in issues").filter(|_| {
            listed
                .iter()
                .any(|p| done_stem(p).is_some_and(|s| closed.contains(&s)))
                && listed.iter().all(|p| done_stem(p).is_some() && exists(p))
        });
        if let Some(at) = state {
            set.push((at, milestone.name));
        }
    }
    let mut out = String::with_capacity(text.len());
    for (i, l) in lines.iter().enumerate() {
        match (set.iter().any(|(at, _)| *at == i), l.split_once(':')) {
            (true, Some((key, value))) => {
                out.push_str(key);
                out.push_str(": done");
                out.push_str(&value[value.trim_end().len()..]);
            }
            _ => out.push_str(l),
        }
    }
    (out, set.into_iter().map(|(_, name)| name).collect())
}

/// The `roadmap` documents of the configuration of the project at `project` (components from the repository root
/// `root`), each as written in the configuration and as components from `root`; `None` for one outside the repository.
pub(super) fn roadmap_documents(root: &Path, project: &[String]) -> Vec<(String, RepoPath)> {
    let project_root = project.iter().fold(root.to_path_buf(), |p, c| p.join(c));
    config_list(&project_root, "roadmap")
        .into_iter()
        .filter_map(|document| {
            let path = resolve(project, &document)?;
            Some((document, path))
        })
        .collect()
}

/// Set `done` the milestones of the `roadmap` documents of the project of `issues` (relative to `root`) that the close
/// of `stem` completes, and print them (CLS-8); returns the documents written.
fn set_milestones_done(root: &Path, issues: &[String], stem: &str) -> Vec<PathBuf> {
    let project = &issues[..issues.len().saturating_sub(1)];
    let closed = HashSet::from([stem.to_string()]);
    let file_of = |p: &[String]| p.iter().fold(root.to_path_buf(), |f, c| f.join(c));
    let mut written = Vec::new();
    for (document, path) in roadmap_documents(root, project) {
        let file = file_of(&path);
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let exists = |p: &[String]| file_of(p).is_file();
        let (updated, names) = milestones_done(&path, &text, issues, &closed, exists);
        if names.is_empty() {
            continue;
        }
        match std::fs::write(&file, updated) {
            Ok(()) => {
                for name in &names {
                    println!("milestone {name} done in {document}");
                }
                written.push(file);
            }
            Err(e) => eprintln!(
                "could not set the milestones done in {}: {e}",
                file.display()
            ),
        }
    }
    written
}

/// Rewrite the references to the given closed issues of the `issues/` directory `issues` (relative to `root`) in
/// every text file under `root`; `moved` is the stem of the issue just moved, whose own links are rebased. Returns the
/// files written.
fn relink(
    root: &Path,
    issues: &[String],
    stems: &HashSet<String>,
    moved: Option<&str>,
) -> Vec<PathBuf> {
    let skipped = skipped_dirs(root);
    let moved = moved.map(|stem| {
        let mut path = issues.to_vec();
        path.extend(["done".to_string(), format!("{stem}.md")]);
        path
    });
    let mut written = Vec::new();
    for file in text_files(root, &skipped) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let path = components(file.strip_prefix(root).unwrap_or(&file));
        let updated = rewritten(&path, &text, issues, stems, moved.as_ref() == Some(&path));
        if updated == text {
            continue;
        }
        match std::fs::write(&file, updated) {
            Ok(()) => {
                println!("updated references in {}", file.display());
                written.push(file);
            }
            Err(e) => eprintln!("could not update {}: {e}", file.display()),
        }
    }
    written
}

/// `git --literal-pathspecs args` run in `root`, its output `None` when it fails: a file name is never a glob.
fn git_literal(root: &Path, args: &[&OsStr]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("--literal-pathspecs")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The paths of the NUL-separated output of git run in `root`, canonical (relative to the top of the work tree).
fn paths_in(root: &Path, out: &[u8]) -> HashSet<PathBuf> {
    out.split(|b| *b == 0)
        .filter(|name| !name.is_empty())
        .map(|name| canonical(&root.join(repo_files::path_of(name))))
        .collect()
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The index of the repository at `root` before a close: the files that differ from `HEAD` in the index or the work
/// tree (`None` when git cannot tell), and those already staged.
struct Before {
    changed: Option<HashSet<PathBuf>>,
    staged: HashSet<PathBuf>,
}

impl Before {
    fn read(root: &Path) -> Self {
        let diff = |args: &[&str]| {
            let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
            git_literal(root, &args).map(|out| paths_in(root, &out))
        };
        Before {
            changed: diff(&["diff", "HEAD", "--name-only", "-z"]),
            staged: diff(&["diff", "--cached", "--name-only", "-z"]).unwrap_or_default(),
        }
    }
}

/// Stage the close of the tracked issue `old` (relative to `root`), from `issue` to `moved`: the issue whole, and
/// each tracked file of `written` that `before` shows unchanged, naming the others and what was staged before
/// (CLS-9). `false` when git refuses to stage it.
fn stage_close(
    root: &Path,
    (old, issue, moved): (&str, &Path, &Path),
    written: &[PathBuf],
    before: &Before,
) -> bool {
    let root = &canonical(root);
    let moved = canonical(moved);
    let written: std::collections::BTreeSet<PathBuf> = written
        .iter()
        .map(|f| canonical(f))
        .filter(|f| *f != moved)
        .collect();
    let mut ls: Vec<&OsStr> = vec![OsStr::new("ls-files"), OsStr::new("-z"), OsStr::new("--")];
    ls.extend(written.iter().map(|f| f.as_os_str()));
    let tracked = if written.is_empty() {
        HashSet::new()
    } else {
        git_literal(root, &ls).map_or_else(HashSet::new, |out| paths_in(root, &out))
    };
    let shown = |file: &Path| {
        file.strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string()
    };
    let mut stage = vec![moved.clone()];
    for file in written.into_iter().filter(|f| tracked.contains(f)) {
        if before
            .changed
            .as_ref()
            .is_none_or(|changed| changed.contains(&file))
        {
            println!("not staged: {} (it held changes of yours)", shown(&file));
        } else {
            stage.push(file);
        }
    }
    let mut add: Vec<&OsStr> = vec![OsStr::new("add"), OsStr::new("--")];
    add.extend(stage.iter().map(|f| f.as_os_str()));
    if git_literal(root, &add).is_none() {
        eprintln!(
            "could not stage the close: stage {} and its rewrites before the commit",
            shown(&moved)
        );
        return false;
    }
    let issue = canonical(issue);
    let mut others: Vec<String> = before
        .staged
        .iter()
        .filter(|f| **f != issue)
        .map(|f| shown(f))
        .collect();
    others.sort();
    for file in others {
        println!("staged before the close: {file} (a bare `git commit` takes it too)");
    }
    println!("staged: commit the close with `git commit` (no `git add` of {old}: git mv staged its removal)");
    true
}

/// The directory `dir` relative to `root`, as components. Exits with an error when it is not under `root`, since
/// the links could not be resolved.
fn under(root: &Path, dir: &Path) -> RepoPath {
    let relative = std::fs::canonicalize(root).ok().and_then(|root| {
        let dir = std::fs::canonicalize(dir).ok()?;
        Some(components(dir.strip_prefix(root).ok()?))
    });
    relative.unwrap_or_else(|| {
        eprintln!(
            "{} is not under {}: references were not updated, run `just close-issue --relink` once fixed",
            dir.display(),
            root.display()
        );
        std::process::exit(1);
    })
}

/// Root of the git repository containing `dir`; the references are only rewritten inside it.
fn repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// The git common directory of the repository of `dir` and the top of its worktree, canonical.
fn worktree_of(dir: &Path) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let common = std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    let common = if common.is_absolute() {
        common
    } else {
        dir.join(common)
    };
    let common = std::fs::canonicalize(common).ok()?;
    Some((common, std::fs::canonicalize(repo_root(dir)?).ok()?))
}

/// Exit with an error when the issues `dir` belong to another worktree of the repository of the current directory
/// (PAR-18).
fn refuse_other_worktree(dir: &Path) {
    let (Some(here), Some((common, there))) = (worktree_of(Path::new(".")), worktree_of(dir))
    else {
        return;
    };
    if here.0 == common && here.1 != there {
        eprintln!(
            "{} is an issue of another worktree ({}): close it there",
            dir.display(),
            there.display()
        );
        std::process::exit(1);
    }
}

/// Free the claim of `stem` made in the worktree of `dir` (CLM-7): delete it on the trunk, else mark it `merge
/// pending` on the current branch; the claim of another existing worktree is left to it.
fn free_claim(dir: &Path, stem: &str) {
    let Some((old, mut claim)) = issue_claim::read(dir, stem) else {
        return;
    };
    // a claim whose worktree was removed is freed by whoever closes the issue
    let theirs = std::fs::canonicalize(&claim.worktree).ok();
    let here = repo_root(dir).and_then(|r| std::fs::canonicalize(r).ok());
    if theirs.is_some() && theirs != here {
        eprintln!(
            "the claim of {stem} belongs to another worktree ({}): it was left as is",
            claim.worktree
        );
        return;
    }
    let branch = issue_claim::current_branch(dir);
    if issue_claim::trunk(dir).is_some_and(|t| t == branch) {
        if issue_claim::delete(dir, stem, &old) {
            println!("claim of {stem} released");
        }
        return;
    }
    claim.state = "merge pending".into();
    claim.branch = branch;
    if issue_claim::write(dir, stem, &claim, Some(&old)) {
        println!(
            "claim of {stem}: merge pending ({}) until the trunk holds the close",
            claim.branch
        );
    } else {
        eprintln!("the claim of {stem} changed during the close: it was left as is");
    }
}

/// `--relink`: rewrite the references to every issue of `issues/done`.
fn relink_closed() -> u8 {
    let stems: HashSet<String> = std::fs::read_dir("issues/done")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .filter(|s| {
            s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
        .collect();
    if stems.is_empty() {
        return 0;
    }
    if let Some(root) = repo_root(Path::new(".")) {
        let issues = under(&root, Path::new("issues"));
        relink(&root, &issues, &stems, None);
    } else {
        eprintln!("not inside a git repository");
        std::process::exit(1);
    }
    0
}

pub(super) fn main(args: &[String]) -> u8 {
    let Some(arg) = args.first().cloned() else {
        eprintln!("usage: cardinal run close-issue issues/<name>.md | --relink");
        std::process::exit(2);
    };
    if arg == "--relink" {
        return relink_closed();
    }
    let issue = Path::new(&arg);
    let (Some(dir), Some(name)) = (issue.parent(), issue.file_name()) else {
        eprintln!("not an issue file: {arg}");
        std::process::exit(2);
    };
    if !issue.is_file() || dir.file_name().is_none_or(|d| d != "issues") {
        eprintln!("{arg} is not an issue file directly under an `issues/` directory");
        std::process::exit(1);
    }
    refuse_other_worktree(dir);
    let done = dir.join("done");
    let target = done.join(name);
    if target.exists() {
        eprintln!("{} already exists", target.display());
        std::process::exit(1);
    }
    std::fs::create_dir_all(&done).expect("done dir is creatable");
    // Run git in the issue's directory with paths relative to it, so the caller's directory does not matter.
    let tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(name)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    let issue_path = canonical(issue);
    let before = tracked
        .then(|| repo_root(dir).map(|root| Before::read(&root)))
        .flatten();
    let moved = if tracked {
        Command::new("git")
            .arg("mv")
            .arg(name)
            .arg(Path::new("done").join(name))
            .current_dir(dir)
            .status()
            .is_ok_and(|s| s.success())
    } else {
        std::fs::rename(issue, &target).is_ok()
    };
    if !moved {
        eprintln!("could not move {arg}");
        std::process::exit(1);
    }
    let how = if tracked {
        "git mv"
    } else {
        "plain rename, not tracked by git"
    };
    println!("moved {} -> {} ({how})", issue.display(), target.display());
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let mut staged = true;
    match repo_root(dir) {
        Some(root) => {
            let issues = under(&root, dir);
            let mut written = relink(
                &root,
                &issues,
                &HashSet::from([stem.to_string()]),
                Some(stem),
            );
            written.extend(set_milestones_done(&root, &issues, stem));
            if let Some(before) = &before {
                let old = format!("{}/{}", issues.join("/"), name.display());
                if !stage_close(&root, (&old, &issue_path, &target), &written, before) {
                    staged = false;
                }
            }
        }
        None => eprintln!("not inside a git repository: references to {stem} were not updated"),
    }
    free_claim(dir, stem);
    println!("{}", super::next_issue::next(dir));
    u8::from(!staged)
}
