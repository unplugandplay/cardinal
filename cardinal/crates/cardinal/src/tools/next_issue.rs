//! Print the next issue to work on and the order of the others (ADR 0021): the ready issues (every `depends_on` of
//! their front matter in `issues/done/`) by the order of their milestone in the `roadmap` documents, then `priority`,
//! then age, then the blocked ones with what blocks them. Reports (stderr) a missing priority, an unknown dependency,
//! a cycle and an unknown milestone.
//! An issue is taken, and skipped, when a git worktree has a branch named after its stem checked out, or when such
//! a branch holds work that is not on the branch of the main worktree (patch-equivalent commits and a squash of any
//! number of commits count as on it: merging the branch would change no file). A branch with neither (left over after a merge, empty) is reported `Stale:` and takes nothing. The mark
//! comes from git alone, so every worktree sees it.
//! An issue is also taken by its claim (`issue_claim.rs`, ADR 0022): `working`, listed with its worktree, or `merge
//! pending`, listed with its branch until the trunk holds the closed issue, when the claim is dropped.
//! Usage: `just next-issue` (from the repo root, or pass the `issues` directory as argument).

use std::path::Path;
use std::process::Command;

use super::issue_claim::{self, Claim};

fn title(dir: &Path, stem: &str) -> String {
    std::fs::read_to_string(dir.join(format!("{stem}.md")))
        .ok()
        .and_then(|t| {
            let body = split_front_matter(&t).map_or(0, |(_, _, body)| body);
            t[body..]
                .lines()
                .find_map(|l| l.strip_prefix("# ").map(str::to_string))
        })
        .unwrap_or_else(|| "(no file)".to_string())
}

fn git_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `branch` holds work that `trunk` lacks; true when git cannot tell, so unmerged work is never ignored.
/// No work when every commit has a patch-equivalent on `trunk` (`git cherry`), or when merging the branch into `trunk`
/// would change no file (a squash of any number of commits): the merge tree is then the tree of `trunk`. Keep in sync
/// with `merged` of `worktree.rs`.
fn has_own_commits(dir: &Path, trunk: &str, branch: &str) -> bool {
    let branch = format!("refs/heads/{branch}");
    let equivalent = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["cherry", trunk, &branch])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| {
            !String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.starts_with('+'))
        });
    if equivalent {
        return false;
    }
    let merged = git_lines(dir, &["merge-tree", "--write-tree", trunk, &branch])
        .first()
        .cloned();
    let trunk_tree = git_lines(dir, &["rev-parse", &format!("{trunk}^{{tree}}")])
        .first()
        .cloned();
    merged.is_none() || merged != trunk_tree
}

/// A local branch: the path of the worktree that has it checked out, if any, and whether it takes its issue.
struct Branch {
    name: String,
    worktree: Option<String>,
    taken: bool,
}

/// The branches of the repository that holds `dir`, and the trunk. The branch of the main worktree is left out: it is
/// the trunk, not an issue. Empty outside a git repository. Only the branches named after one of `stems` are examined
/// for commits.
fn branches(dir: &Path, stems: &[&str]) -> (Vec<Branch>, String) {
    let mut out: Vec<(String, Option<String>)> = git_lines(
        dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .into_iter()
    .map(|b| (b, None))
    .collect();
    let (mut path, mut first) = (String::new(), true);
    let mut trunk = String::new();
    for line in git_lines(dir, &["worktree", "list", "--porcelain"]) {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            if first {
                out.retain(|(name, _)| name != b);
                trunk = b.to_string();
            } else if let Some(entry) = out.iter_mut().find(|(name, _)| name == b) {
                entry.1 = Some(path.clone());
            }
            first = false;
        } else if let Some(h) = line
            .strip_prefix("HEAD ")
            .filter(|_| first && trunk.is_empty())
        {
            trunk = h.to_string();
        } else if line.is_empty() {
            first = false;
        }
    }
    let branches = out
        .into_iter()
        .map(|(name, worktree)| {
            let taken = worktree.is_some()
                || trunk.is_empty()
                || !stems.contains(&name.as_str())
                || has_own_commits(dir, &trunk, &name);
            Branch {
                name,
                worktree,
                taken,
            }
        })
        .collect();
    (branches, trunk)
}

/// The stems of the issue files of `dir`, sorted.
fn issue_stems(dir: &Path) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md"))
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    stems.sort();
    stems
}

/// The lines of the front matter of `text` (between a first line `---` and the next `---`, line endings kept) and
/// the offsets of its closing line and of the body; `None` when the file opens no closed front matter.
fn split_front_matter(text: &str) -> Option<(Vec<&str>, usize, usize)> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let (mut at, mut matter) = (first.len(), Vec::new());
    for l in lines {
        if l.trim_end() == "---" {
            return Some((matter, at, at + l.len()));
        }
        at += l.len();
        matter.push(l);
    }
    None
}

/// `value` without the single (a doubled `'` inside stands for one) or double quotes around it.
fn unquote(value: &str) -> String {
    let inner = |q: char| {
        (value.len() >= 2 && value.starts_with(q) && value.ends_with(q))
            .then(|| &value[1..value.len() - 1])
    };
    if let Some(v) = inner('\'') {
        v.replace("''", "'")
    } else {
        inner('"').unwrap_or(value).to_string()
    }
}

/// The value of `key` in the front matter of `text`, unquoted.
fn front_matter(text: &str, key: &str) -> Option<String> {
    let (matter, _, _) = split_front_matter(text)?;
    matter
        .iter()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(':'))
        .map(|v| unquote(v.trim()))
}

/// The place of a `priority` in the order: `high` first; `None` for a missing or unknown one (NXT-2).
pub(super) fn priority(text: &str) -> Option<u8> {
    match front_matter(text, "priority")?.as_str() {
        "high" => Some(0),
        "medium" => Some(1),
        "low" => Some(2),
        _ => None,
    }
}

/// The stems of the `depends_on` of the front matter of `text`, `[<stem>, ...]` on one line, each once (NXT-7): empty
/// when the key is absent, `None` when its value is not such a list (NXT-8).
pub(super) fn depends_on(text: &str) -> Option<Vec<String>> {
    let Some(value) = front_matter(text, "depends_on") else {
        return Some(Vec::new());
    };
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut stems: Vec<String> = Vec::new();
    for stem in inner.split(',') {
        let stem = unquote(stem.trim());
        if stem.is_empty() || stem.contains([' ', '/', '[']) {
            return None;
        }
        if !stems.contains(&stem) {
            stems.push(stem);
        }
    }
    Some(stems)
}

/// The milestone named by the first non-empty line of the `# Milestone` section of `text` (NXT-10).
fn milestone(text: &str) -> Option<String> {
    let mut lines = text.lines();
    // the heading of the template of `skill:create-issue` carries a note: `# Milestone (optional; ...)`
    lines.by_ref().find(|l| {
        let l = l.trim_end();
        l == "# Milestone" || l.starts_with("# Milestone ")
    })?;
    lines
        .take_while(|l| !l.starts_with("# "))
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// The milestones of the `roadmap` documents of the configuration of `root`, in order; `None` when no document
/// can be read (NXT-14).
fn milestones(root: &Path) -> Option<Vec<String>> {
    let texts: Vec<String> = super::config_list(root, "roadmap")
        .unwrap_or_default()
        .into_iter()
        .filter_map(|rel| std::fs::read_to_string(root.join(rel)).ok())
        .collect();
    (!texts.is_empty()).then(|| {
        texts
            .iter()
            .flat_map(|t| {
                let lines: Vec<&str> = t.split_inclusive('\n').collect();
                super::roadmap::milestones(&lines)
            })
            .map(|m| m.name)
            .collect()
    })
}

/// The stems of the issue files `git` added under `dir`, oldest first: the place of each in the tie-breaker (NXT-1).
fn ages(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in git_lines(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            "--reverse",
            "--no-renames",
            "--diff-filter=A",
            "--relative",
            "--format=",
            "--name-only",
            "--",
            ".",
        ],
    ) {
        if let Some(stem) = line.strip_suffix(".md").filter(|s| !s.contains('/')) {
            if !out.iter().any(|s| s == stem) {
                out.push(stem.to_string());
            }
        }
    }
    out
}

/// The groups of open issues whose dependencies (`deps`, to open issues only) form a cycle, each group once, its
/// stems sorted (NXT-9).
fn cycles(deps: &[(String, Vec<String>)]) -> Vec<Vec<String>> {
    let reach = |from: &str| {
        let mut seen: Vec<&str> = Vec::new();
        let mut todo: Vec<&str> = vec![from];
        while let Some(s) = todo.pop() {
            for d in deps.iter().filter(|(x, _)| x == s).flat_map(|(_, d)| d) {
                if !seen.contains(&d.as_str()) {
                    seen.push(d);
                    todo.push(d);
                }
            }
        }
        seen.into_iter().map(str::to_string).collect::<Vec<_>>()
    };
    let reached: Vec<(String, Vec<String>)> =
        deps.iter().map(|(s, _)| (s.clone(), reach(s))).collect();
    let mut groups: Vec<Vec<String>> = Vec::new();
    for (stem, from) in &reached {
        if !from.contains(stem) || groups.iter().any(|g| g.contains(stem)) {
            continue;
        }
        let mut group: Vec<String> = reached
            .iter()
            .filter(|(other, back)| from.contains(other) && back.contains(stem))
            .map(|(other, _)| other.clone())
            .collect();
        group.sort();
        groups.push(group);
    }
    groups.sort();
    groups
}

/// An open issue: its `why` and what blocks it (empty when it is ready, NXT-7).
struct Entry {
    stem: String,
    why: String,
    blockers: Vec<String>,
}

/// The order of the open issues of `dir` (NXT-1, NXT-7, NXT-10), each problem reported on stderr (NXT-2, NXT-8, NXT-9,
/// NXT-13); the flag is set when one was.
fn order(dir: &Path) -> (Vec<Entry>, bool) {
    let stems = issue_stems(dir);
    let done = issue_stems(&dir.join("done"));
    let root = dir.parent().unwrap_or(Path::new(""));
    let roadmap = milestones(root);
    let ages = ages(dir);
    let mut drift = false;
    let mut report = |line: String| {
        eprintln!("{line}");
        drift = true;
    };
    let mut keyed = Vec::new();
    let mut open_deps = Vec::new();
    for stem in &stems {
        let text = std::fs::read_to_string(dir.join(format!("{stem}.md"))).unwrap_or_default();
        let why = front_matter(&text, "why").unwrap_or_default();
        let rank = priority(&text).unwrap_or_else(|| {
            report(format!("issue without a priority: {stem}"));
            3
        });
        let place = match (milestone(&text), &roadmap) {
            (Some(name), Some(names)) => {
                names.iter().position(|n| *n == name).unwrap_or_else(|| {
                    report(format!("unknown milestone of {stem}: {name}"));
                    usize::MAX
                })
            }
            _ => usize::MAX,
        };
        let blockers = match depends_on(&text) {
            None => {
                report(format!("invalid depends_on: {stem}"));
                vec!["an invalid depends_on".to_string()]
            }
            Some(deps) => {
                for dep in deps
                    .iter()
                    .filter(|d| !stems.contains(d) && !done.contains(d))
                {
                    report(format!("unknown dependency of {stem}: {dep}"));
                }
                open_deps.push((
                    stem.clone(),
                    deps.iter().filter(|d| stems.contains(d)).cloned().collect(),
                ));
                deps.into_iter().filter(|d| !done.contains(d)).collect()
            }
        };
        let age = ages.iter().position(|s| s == stem).unwrap_or(usize::MAX);
        keyed.push((
            (!blockers.is_empty(), place, rank, age, stem.clone()),
            Entry {
                stem: stem.clone(),
                why,
                blockers,
            },
        ));
    }
    for group in cycles(&open_deps) {
        report(format!("dependency cycle: {}", group.join(", ")));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    (keyed.into_iter().map(|(_, e)| e).collect(), drift)
}

/// The open issues of a directory in order, and the branches and claims that may take them.
struct Queue {
    entries: Vec<Entry>,
    drift: bool,
    branches: Vec<Branch>,
    claims: Vec<(String, Claim)>,
}

/// The claims of the open issues `stems` (CLM-5, CLM-6); a `merge pending` one whose issue the trunk holds in
/// `issues/done/` is deleted instead, whether or not the issue is still open here.
fn claims(dir: &Path, stems: &[&str], trunk: &str) -> Vec<(String, Claim)> {
    let mut out = Vec::new();
    for (stem, oid, claim) in issue_claim::all(dir) {
        let merged = claim.state == "merge pending"
            && !trunk.is_empty()
            && !git_lines(
                dir,
                &["cat-file", "-t", &format!("{trunk}:./done/{stem}.md")],
            )
            .is_empty();
        if merged {
            issue_claim::delete(dir, &stem, &oid);
        } else if stems.contains(&stem.as_str()) {
            out.push((stem, claim));
        }
    }
    out
}

impl Queue {
    /// The order of the issues (NXT-1); the problems are reported on stderr.
    fn read(dir: &Path) -> Self {
        let (entries, drift) = order(dir);
        let stems: Vec<&str> = entries.iter().map(|e| e.stem.as_str()).collect();
        let (branches, trunk) = branches(dir, &stems);
        let claims = claims(dir, &stems, &trunk);
        Queue {
            entries,
            drift,
            branches,
            claims,
        }
    }

    /// The branch that takes the issue `stem`, if any (PAR-4).
    fn taken_by(&self, stem: &str) -> Option<&Branch> {
        self.branches.iter().find(|b| b.name == stem && b.taken)
    }

    /// The claim of the issue `stem`, if any (CLM-5, CLM-6).
    fn claim(&self, stem: &str) -> Option<&Claim> {
        self.claims.iter().find(|(s, _)| s == stem).map(|(_, c)| c)
    }

    /// Whether a branch or a claim keeps the issue `stem` from being offered.
    fn taken(&self, stem: &str) -> bool {
        self.taken_by(stem).is_some() || self.claim(stem).is_some()
    }

    /// The ready issues no branch takes, with their position in the order: `(index, entry)` (PAR-4, NXT-7).
    fn free(&self) -> Vec<(usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.blockers.is_empty() && !self.taken(&e.stem))
            .collect()
    }

    /// What `next-issue` offers first: `Next: issues/<stem>.md` and the title of the first free ready issue, else the
    /// line saying there is none (NXT-12).
    fn offer(&self, dir: &Path, free: &[(usize, &Entry)]) -> String {
        match free.first() {
            None if self.entries.is_empty() => "No open issue.".to_string(),
            None if self.entries.iter().all(|e| self.taken(&e.stem)) => {
                "Every open issue is taken.".to_string()
            }
            None => "No ready issue: every free one waits on another.".to_string(),
            Some((_, e)) => format!("Next: issues/{}.md\n  {}", e.stem, title(dir, &e.stem)),
        }
    }
}

/// The issue `next-issue` offers first in `dir` (CLS-7: `close-issue` prints it after a close).
pub(super) fn next(dir: &Path) -> String {
    let queue = Queue::read(dir);
    queue.offer(dir, &queue.free())
}

pub(super) fn main(args: &[String]) -> u8 {
    let dir = args.first().map_or("issues", String::as_str);
    let dir = Path::new(dir);
    if !dir.is_dir() {
        eprintln!("no {} directory", dir.display());
        std::process::exit(2);
    }
    let queue = Queue::read(dir);
    let free = queue.free();
    println!("{}", queue.offer(dir, &free));
    if let Some((_, first)) = free.first() {
        println!("  why: {}", first.why);
        for (i, e) in free.iter().skip(1) {
            println!("{:>3}. {}", i + 1, e.stem);
        }
    }
    for e in queue.entries.iter().filter(|e| !e.blockers.is_empty()) {
        println!("Blocked: {} (by {})", e.stem, e.blockers.join(", "));
    }
    let taken = |stem: &str| queue.taken_by(stem).map(|b| b.worktree.as_deref());
    let stale = |stem: &str| queue.branches.iter().any(|b| b.name == stem && !b.taken);
    for e in &queue.entries {
        match queue.claim(&e.stem) {
            Some(c) if c.state == "merge pending" => {
                println!("Merge pending: {} ({})", e.stem, c.branch);
                continue;
            }
            Some(c) => {
                let ended = if c.alive() { "" } else { ", session ended" };
                println!("Taken: {} (claimed in {}{ended})", e.stem, c.worktree);
                continue;
            }
            None => {}
        }
        if let Some(path) = taken(&e.stem) {
            println!(
                "Taken: {}{}",
                e.stem,
                path.map_or(String::new(), |p| format!(" (worktree {p})"))
            );
        }
    }
    for e in &queue.entries {
        if stale(&e.stem) {
            println!("Stale: {0} (branch with no worktree and nothing off the main branch; run `just close-worktree {0}`)", e.stem);
        }
    }
    std::process::exit(i32::from(queue.drift));
}
