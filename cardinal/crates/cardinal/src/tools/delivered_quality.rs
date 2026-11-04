//! Measure what happened to the work of each done issue of a repository (`specs/delivered-quality.md`): one
//! tab-separated row per file of `issues/done/`, with the commits of the issue, the fixes inside it, and the fixes
//! of the lines it wrote and the reverts of its files on the default branch after its close. Signals, not verdicts: a fix after a feature may
//! come from a new need. The sessions that made its commits give the failed checks and the typed messages of its
//! span, the tokens of its messages, and the lines it changed in the specs and in the docs. With `--summary`, the columns are summed per Cardinal revision and setup (the main model, alone or advised). No text of a commit, an issue or a message
//! is printed.
//! Usage: `cardinal run delivered-quality [repo] [--projects <dir>] [--window-days <n>] [--summary]` (default the
//! current directory, the `projects/` of the configuration directory (EVA-15) and 30 days).

use super::check_specs::{
    citation, citation_list, config_names, is_req_id, markers, name_matches, spec_patterns,
};
use super::transcripts;

use chrono::{DateTime, Duration, FixedOffset};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use transcripts::{
    default_branch, for_each_line, git, is_validation, projects, projects_root, repository,
    revision, sessions, sorted_entries, subagents, typed_text, Place,
};

pub(super) const SUMMARY_HEADER: &str = "revision\tsetup\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\ttraced\ttraced_without_red\tspec_lines\tdoc_lines\tsessions\tfailed_checks\tvalidations\tother_messages\tinput_tokens\tcache_read_tokens\toutput_tokens\ttoken_issues";
const HEADER: &str = "issue\trevision\tsetup\tclosed\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\ttraced\ttraced_without_red\tspec_lines\tdoc_lines\tsessions\tfailed_checks\tvalidations\tother_messages\tinput_tokens\tcache_read_tokens\toutput_tokens";

/// The paths whose later fixes say nothing of the issue's work: the issue files, the docs and the lock of the sync.
const UNRELATED_PREFIXES: &[&str] = &["issues/", "docs/"];
const UNRELATED_FILES: &[&str] = &super::transcripts::LOCKS;

/// A non-merge commit of the default branch.
struct Commit {
    hash: String,
    date: DateTime<FixedOffset>,
    subject: String,
    body: String,
    /// The paths it changed, renames read as a deletion and an addition.
    paths: Vec<String>,
    /// The paths it added.
    added: Vec<String>,
}

impl Commit {
    /// The Conventional Commits type of its subject (feature `conventional-commits`), empty when it has none.
    fn kind(&self) -> &str {
        let head = self.subject.split(':').next().unwrap_or_default();
        let kind = head
            .split('(')
            .next()
            .unwrap_or_default()
            .trim_end_matches('!');
        if self.subject.contains(':')
            && !kind.is_empty()
            && kind.chars().all(|c| c.is_ascii_lowercase())
        {
            kind
        } else {
            ""
        }
    }

    fn touches(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p == path)
    }
}

/// The non-merge commits of the default branch of `repo`, oldest first.
fn commits(repo: &Path) -> Vec<Commit> {
    let Some(branch) = default_branch(repo) else {
        return Vec::new();
    };
    let Some(log) = git(
        repo,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            "--no-merges",
            "--no-renames",
            "--reverse",
            "--name-status",
            "--format=%x1e%H%x1f%aI%x1f%B%x1f",
            branch,
        ],
    ) else {
        return Vec::new();
    };
    log.split('\x1e')
        .filter_map(|record| {
            let mut fields = record.splitn(4, '\x1f');
            let hash = fields.next()?.trim().to_string();
            let date = DateTime::parse_from_rfc3339(fields.next()?).ok()?;
            let message = fields.next()?;
            let status = fields.next().unwrap_or_default();
            let mut paths = Vec::new();
            let mut added = Vec::new();
            for line in status.lines().filter(|l| !l.trim().is_empty()) {
                let mut parts = line.split('\t');
                let code = parts.next().unwrap_or_default();
                if let Some(path) = parts.next() {
                    if code == "A" {
                        added.push(path.to_string());
                    }
                    paths.push(path.to_string());
                }
            }
            Some(Commit {
                hash,
                date,
                subject: message.lines().next().unwrap_or_default().to_string(),
                body: message.to_string(),
                paths,
                added,
            })
        })
        .collect()
}

/// The indexes in `log` of the commits of the issue `stem` (QUA-2), oldest first; empty when it was never closed on
/// the default branch.
fn issue_commits(repo: &Path, log: &[Commit], stem: &str) -> Vec<usize> {
    let open = format!("issues/{stem}.md");
    let done = format!("issues/done/{stem}.md");
    let Some(close) = log.iter().rposition(|c| c.added.contains(&done)) else {
        return Vec::new();
    };
    let candidates: Vec<usize> = (0..=close)
        .filter(|&i| log[i].touches(&open) || log[i].touches(&done))
        .collect();
    // The oldest version of the file holding the heading outside a fence (QUA-12) started the work: the version the
    // commit wrote, under `issues/done/` once it moved there.
    let tracked = |c: &Commit| {
        let path = if c.touches(&done) { &done } else { &open };
        git(repo, &["cat-file", "blob", &format!("{}:{path}", c.hash)])
            .is_some_and(|text| matches!(section(&text, "# Tracking", |_| true), Section::Read(_)))
    };
    let start = candidates
        .iter()
        .copied()
        .find(|&i| tracked(&log[i]))
        .unwrap_or(close);
    let closes_another = |c: &Commit| {
        c.added
            .iter()
            .any(|p| p.starts_with("issues/done/") && *p != done)
    };
    candidates
        .into_iter()
        .filter(|&i| i >= start && (i == close || !closes_another(&log[i])))
        .collect()
}

/// A section of an issue file as QUA-12 reads it.
enum Section<'a> {
    /// The lines after its heading.
    Read(Vec<&'a str>),
    /// Its heading stands only inside a fenced code block.
    Unread,
    /// The file does not hold its heading.
    Absent,
}

/// The lines of `text` after the line `heading` (trailing spaces ignored), up to the next line that `ends` accepts,
/// both outside a fenced code block (QUA-12).
fn section<'a>(text: &'a str, heading: &str, ends: impl Fn(&str) -> bool) -> Section<'a> {
    let mut quoted = false;
    let mut lines = super::fenced(text.lines()).skip_while(|&(line, fenced)| {
        let found = line.trim_end() == heading;
        quoted |= found && fenced;
        fenced || !found
    });
    if lines.next().is_none() {
        drop(lines);
        return if quoted {
            Section::Unread
        } else {
            Section::Absent
        };
    }
    Section::Read(
        lines
            .take_while(|&(line, fenced)| fenced || !ends(line))
            .map(|(line, _)| line)
            .collect(),
    )
}

/// The section `heading`, empty when the file does not hold its heading; `None` when it is unread (QUA-12).
fn readable<'a>(text: &'a str, heading: &str, ends: impl Fn(&str) -> bool) -> Option<Vec<&'a str>> {
    match section(text, heading, ends) {
        Section::Read(lines) => Some(lines),
        Section::Absent => Some(Vec::new()),
        Section::Unread => None,
    }
}

/// The number of plan lines of an issue file (QUA-3): `<n>. [x]` or `<n>. [ ]` at the start of a line of `## Plan`,
/// else of `# Tracking` before its first sub-heading (the older issues); `None` when neither heading stands outside a
/// fence and one stands inside (QUA-12).
fn deliverables(text: &str) -> Option<usize> {
    let is_item = |line: &&str| {
        let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.len() < line.len() && (rest.starts_with(". [x]") || rest.starts_with(". [ ]"))
    };
    let ends = |l: &str| l.starts_with('#');
    let plan = match (
        section(text, "## Plan", ends),
        section(text, "# Tracking", ends),
    ) {
        (Section::Read(lines), _) | (_, Section::Read(lines)) => lines,
        (Section::Absent, Section::Absent) => Vec::new(),
        _ => return None,
    };
    Some(plan.into_iter().filter(is_item).count())
}

/// The reviews recorded in the `# Tracking` section of an issue file and the sum of their findings (QUA-6): a line
/// holding `Code review` is one review, unless it is a row of the `check | result` table, its findings the `<n>` of its first `<n> finding(s)` (0 for `no findings`),
/// since what follows may count the fixed ones; `None` for the sum when a line gives no count, and for both when the
/// section is unread (QUA-12).
fn reviews(text: &str) -> (Option<usize>, Option<usize>) {
    let Some(tracking) = readable(text, "# Tracking", |l| l.starts_with("# ")) else {
        return (None, None);
    };
    let lines: Vec<String> = tracking
        .iter()
        .filter(|l| !l.trim_start().starts_with('|'))
        .map(|l| l.to_lowercase())
        .filter(|l| l.contains("code review"))
        .collect();
    let counts = lines.iter().map(|line| {
        let words: Vec<&str> = line
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        words
            .windows(2)
            .filter(|w| w[1] == "finding" || w[1] == "findings")
            .find_map(|w| {
                if w[0] == "no" {
                    Some(0)
                } else {
                    w[0].parse().ok()
                }
            })
    });
    let sum = counts
        .collect::<Option<Vec<usize>>>()
        .map(|c| c.iter().sum());
    (Some(lines.len()), sum)
}

/// The requirement ids of a `// spec: A-1, B-2` list, read as `check-specs` reads them.
fn cited(list: &str) -> impl Iterator<Item = String> + '_ {
    citation(list)
        .0
        .filter(|w| is_req_id(w))
        .map(str::to_string)
}

/// The requirement ids the commits `own` cite on the citation lines (a `spec:` citation of `check-specs` for the
/// file's extension, QUA-13) they add to the files of the issue, less, in each commit, those cited on the citation
/// lines it removes, renames detected (QUA-11): a moved or extended citation traces only its new ids. `None` when git
/// cannot show the diffs.
fn traced(repo: &Path, own: &[&Commit]) -> Option<BTreeSet<String>> {
    let mut args = vec![
        "-c",
        "core.quotePath=false",
        "show",
        "-M",
        "-U0",
        "--no-color",
        "--format=%x1e",
        "-G spec:",
    ];
    args.extend(own.iter().map(|c| c.hash.as_str()));
    args.extend(["--", "."]);
    let excluded: Vec<String> = UNRELATED_PREFIXES
        .iter()
        .chain(UNRELATED_FILES)
        .map(|p| format!(":(exclude){}", p.trim_end_matches('/')))
        .collect();
    args.extend(excluded.iter().map(String::as_str));
    let diffs = git(repo, &args)?;
    let patterns = spec_patterns(repo).0;
    // the markers of a path of a diff header: none when it is no test file (QUA-13)
    let markers_of = |header: &str| -> &'static [&'static str] {
        // git ends a name holding a space with a tab, and quotes one holding a special character
        let path = header.trim_end_matches('\t');
        let path = path
            .strip_prefix('"')
            .and_then(|p| p.strip_suffix('"'))
            .unwrap_or(path);
        let path = Path::new(path.split_once('/').map_or(path, |(_, p)| p));
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if patterns.iter().any(|p| name_matches(p, name)) {
            markers(path)
        } else {
            &[]
        }
    };
    let mut traced = BTreeSet::new();
    for diff in diffs.split('\x1e') {
        let (mut added, mut removed) = (BTreeSet::new(), BTreeSet::new());
        // the old and new paths of the file the hunks below belong to, read from its header
        let (mut old, mut new, mut header): (&[&str], &[&str], bool) = (&[], &[], false);
        for line in diff.lines() {
            if line.starts_with("diff --git ") {
                header = true;
            } else if header && line.starts_with("@@") {
                header = false;
            } else if header {
                if let Some(path) = line.strip_prefix("--- ") {
                    old = markers_of(path);
                } else if let Some(path) = line.strip_prefix("+++ ") {
                    new = markers_of(path);
                }
            } else if let Some(text) = line.strip_prefix('+') {
                added.extend(citation_list(new, text).into_iter().flat_map(cited));
            } else if let Some(text) = line.strip_prefix('-') {
                removed.extend(citation_list(old, text).into_iter().flat_map(cited));
            }
        }
        traced.extend(added.difference(&removed).cloned());
    }
    Some(traced)
}

fn is_markdown(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// The files whose lines count as docs (QUA-14), read once from `.claude/cardinal.toml`: the `root_docs`
/// (`README.md` when none), the `architecture` document, and the `roadmap` documents, which never count.
struct DocFiles {
    root_docs: Vec<String>,
    architecture: Option<String>,
    roadmap: Vec<String>,
}

impl DocFiles {
    fn read(repo: &Path) -> Self {
        let table: toml::Table = std::fs::read_to_string(crate::config_of(repo))
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or_default();
        let project = table.get("project");
        let architecture = project
            .and_then(|p| p.get("architecture"))
            .and_then(toml::Value::as_str)
            .map(str::to_string);
        let roadmap = project
            .and_then(|p| p.get("roadmap"))
            .and_then(toml::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        DocFiles {
            root_docs: config_names(repo, "root_docs", "README.md").0,
            architecture,
            roadmap,
        }
    }

    fn is_doc(&self, path: &str) -> bool {
        let markdown = is_markdown(path);
        !self.roadmap.iter().any(|r| r == path)
            && ((markdown && path.starts_with("docs/") && !path.starts_with("docs/adr/"))
                || self.root_docs.iter().any(|r| r == path)
                || self.architecture.as_deref() == Some(path))
    }
}

/// The added and removed lines the commits `own` change in the specs and in the docs (QUA-14): the `.md` files of
/// `specs/` but its README and template; the doc files of `docs`. Renames are detected, a binary file counts 0.
/// `None` when git cannot show the diffs.
fn doc_spec_lines(repo: &Path, own: &[&Commit], docs: &DocFiles) -> Option<(usize, usize)> {
    let mut args = vec!["show", "-M", "--numstat", "-z", "--format=%x1e"];
    args.extend(own.iter().map(|c| c.hash.as_str()));
    let stats = git(repo, &args)?;
    let (mut spec_lines, mut doc_lines) = (0, 0);
    // `-z`: `<added>\t<removed>\t<path>\0`, or for a rename `<added>\t<removed>\t\0<old>\0<new>\0`
    let mut fields = stats.split('\0');
    while let Some(field) = fields.next() {
        let mut parts = field.trim_start_matches(['\x1e', '\n']).splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let path = if path.is_empty() {
            fields.next();
            fields.next().unwrap_or_default()
        } else {
            path
        };
        let count = added.parse().unwrap_or(0) + removed.parse().unwrap_or(0);
        let spec = path.starts_with("specs/")
            && is_markdown(path)
            && !matches!(path, "specs/README.md" | "specs/TEMPLATE.md");
        if spec {
            spec_lines += count;
        } else if docs.is_doc(path) {
            doc_lines += count;
        }
    }
    Some((spec_lines, doc_lines))
}

/// The requirement ids recorded as seen failing in an issue file (QUA-11): the list that opens each `red:` item of
/// its `# Tracking` section, an item running over its indented continuation lines, the list ending at the first
/// word that is not an id (`red: ABC-3, ABC-4: <failure>` names ABC-3 and ABC-4, `red: none (ABC-3)` none); `None`
/// when the section is unread (QUA-12).
fn red_ids(text: &str) -> Option<BTreeSet<String>> {
    let tracking = readable(text, "# Tracking", |l| l.starts_with("# "))?;
    let unmarked = |line: &str| {
        let line = line.trim();
        let line = line.trim_start_matches(['-', '*', '+']);
        let numbered = line.trim_start_matches(|c: char| c.is_ascii_digit());
        let line = match numbered.strip_prefix('.') {
            Some(rest) if numbered.len() < line.len() => rest,
            _ => line,
        };
        line.trim_start().to_string()
    };
    let mut ids = BTreeSet::new();
    for (i, line) in tracking.iter().enumerate() {
        let Some(head) = unmarked(line).strip_prefix("red:").map(str::to_string) else {
            continue;
        };
        let continued = tracking[i + 1..].iter().take_while(|l| {
            l.starts_with(char::is_whitespace) && !l.trim().is_empty() && unmarked(l) == l.trim()
        });
        let item: Vec<String> = std::iter::once(head)
            .chain(continued.map(|l| l.trim().to_string()))
            .collect();
        for word in item
            .join(" ")
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|w| !w.is_empty())
        {
            let id = word.strip_suffix(':').unwrap_or(word);
            if !is_req_id(id) {
                break;
            }
            ids.insert(id.to_string());
            if id.len() < word.len() {
                break;
            }
        }
    }
    Some(ids)
}

/// Whether `commit` reverts the work of the issue (QUA-4): a `revert` type or a `Revert "` subject changing one of
/// its `files`, or a `This reverts commit <hash>` citing one of its `own` commits.
fn reverts(commit: &Commit, files: &BTreeSet<&str>, own: &[&Commit]) -> bool {
    let typed = commit.kind() == "revert" || commit.subject.starts_with("Revert \"");
    let cited = commit
        .body
        .split("This reverts commit ")
        .skip(1)
        .any(|rest| {
            let hash: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
            hash.len() >= 7 && own.iter().any(|c| c.hash.starts_with(&hash))
        });
    cited || typed && commit.paths.iter().any(|p| files.contains(p.as_str()))
}

/// Whether `path` says nothing of an issue's work (QUA-4): an issue file, a doc or the lock of the sync.
fn unrelated(path: &str) -> bool {
    UNRELATED_PREFIXES.iter().any(|d| path.starts_with(d)) || UNRELATED_FILES.contains(&path)
}

/// The commits that last wrote, at the first parent of `fix`, the lines it removes or changes and, above a hunk that
/// only adds lines, the line right above it (QUA-4), in the paths `unrelated` leaves; renames followed, lines moved or
/// copied from another file of the same commit included. Empty for a root commit.
fn blamed(repo: &Path, fix: &str) -> HashSet<String> {
    let parent = format!("{fix}^");
    // `-z` gives the paths unquoted, whatever the prefixes and the quoting of the diff headers
    let Some(status) = git(repo, &["diff", "-M", "--name-status", "-z", &parent, fix]) else {
        return HashSet::new();
    };
    let mut fields = status.split('\0').filter(|f| !f.is_empty());
    let mut writers = HashSet::new();
    while let Some(code) = fields.next() {
        let paired = code.starts_with('R') || code.starts_with('C');
        let (Some(old), new) = (fields.next(), if paired { fields.next() } else { None }) else {
            break;
        };
        // an added file or the source of a copy holds no line the fix removes
        if code.starts_with('A') || code.starts_with('C') || unrelated(old) {
            continue;
        }
        let mut args = vec![
            "diff",
            "-M",
            "-U0",
            "--no-color",
            "--no-ext-diff",
            &parent,
            fix,
            "--",
            old,
        ];
        args.extend(new);
        let Some(diff) = git(repo, &args) else {
            continue;
        };
        // a hunk header is the only line of a diff that starts with `@@`: every line of a hunk starts with ` `, `+` or `-`
        let ranges: Vec<String> = diff
            .lines()
            .filter_map(|l| l.strip_prefix("@@ -")?.split(' ').next())
            .filter_map(|old| {
                let (start, count) = old.split_once(',').unwrap_or((old, "1"));
                // a hunk that only adds lines sits below its `start`, 0 at the top of the file
                let (start, count) = (
                    start.parse::<usize>().ok()?,
                    count.parse::<usize>().ok()?.max(1),
                );
                (start > 0).then(|| format!("{start},{}", start + count - 1))
            })
            .collect();
        if ranges.is_empty() {
            continue;
        }
        let mut args = vec!["blame", "--porcelain", "-M", "-C"];
        for range in &ranges {
            args.extend(["-L", range]);
        }
        args.extend([parent.as_str(), "--", old]);
        // the user's `blame.ignoreRevsFile` makes blame fail in a repository without that file, or skip a commit
        let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(&args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
        else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        // the header line of each blamed group opens with the hash of its commit (40 hex digits, 64 with SHA-256)
        writers.extend(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| {
                    let hash = l.split(' ').next()?;
                    (hash.len() >= 40 && hash.chars().all(|c| c.is_ascii_hexdigit()))
                        .then(|| hash.to_string())
                }),
        );
    }
    writers
}

/// What a session tells of the issue it worked on, at a timestamp.
enum Event {
    Validation,
    Other,
    FailedCheck,
    /// The `message.id`, input sum (REP-5), cache read and output tokens of an assistant message (QUA-15).
    Tokens(String, [u64; 3]),
}

/// The tokens of an assistant message line, once per `message.id` (EVA-2, QUA-15): the top-level usage, which adds
/// up the executor iterations of an advised message and holds none of the advisor's tokens.
fn tokens(line: &Value, seen: &mut HashSet<String>) -> Option<Event> {
    let message = &line["message"];
    let usage = &message["usage"];
    let id = message["id"]
        .as_str()
        .filter(|_| line["type"] == "assistant" && usage.is_object())?;
    if !seen.insert(id.to_string()) {
        return None;
    }
    let count = |key: &str| usage[key].as_u64().unwrap_or(0);
    let cache_read = count("cache_read_input_tokens");
    let input = count("input_tokens") + cache_read + count("cache_creation_input_tokens");
    Some(Event::Tokens(
        id.to_string(),
        [input, cache_read, count("output_tokens")],
    ))
}

/// The timestamp of a transcript line.
fn timestamp(line: &Value) -> Option<DateTime<FixedOffset>> {
    line["timestamp"]
        .as_str()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
}

/// A session of the repository: its span, sub-agents included, its branches other than the default ones, its setup, and
/// the events of its own transcript.
struct Session {
    branches: Vec<String>,
    setup: Option<String>,
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
    events: Vec<(DateTime<FixedOffset>, Event)>,
}

/// A check of the project: `just` followed by a word starting with `check` or `test` (QUA-8).
fn is_check(command: &str) -> bool {
    let words: Vec<&str> = command
        .split(|c: char| c.is_whitespace() || ";&|()".contains(c))
        .filter(|w| !w.is_empty())
        .collect();
    words
        .windows(2)
        .any(|w| w[0] == "just" && (w[1].starts_with("check") || w[1].starts_with("test")))
}

/// The model Claude Code names on the line it writes for a failed request: no model worked on it.
pub(super) const SYNTHETIC: &str = "<synthetic>";

/// The setup of a session whose main model is `main` (REP-15): alone, or advised by `advisor` when that is another
/// model (a model never advises itself). The report groups the sessions under this same name.
pub(super) fn setup_name(main: &str, advisor: Option<&str>) -> String {
    match advisor {
        Some(advisor) if advisor != main => format!("{main} advised by {advisor}"),
        _ => format!("{main} solo"),
    }
}

/// The first main model and advisor model of a session's assistant lines, in order of first appearance (QUA-16).
#[derive(Default)]
struct Models {
    main: Option<String>,
    advisor: Option<String>,
}

impl Models {
    fn observe(&mut self, line: &Value) {
        if line["type"] != "assistant" {
            return;
        }
        let message = &line["message"];
        if self.main.is_none() && message["id"].is_string() && message["usage"].is_object() {
            self.main = message["model"]
                .as_str()
                .filter(|m| *m != SYNTHETIC)
                .map(String::from);
        }
        if self.advisor.is_none() {
            self.advisor = line["advisorModel"].as_str().map(String::from);
        }
    }

    /// The setup as REP-15 names it, none without a main model.
    fn setup(self) -> Option<String> {
        Some(setup_name(&self.main?, self.advisor.as_deref()))
    }
}

/// The sessions of the projects directory `root` whose repository is `repo` (QUA-7); `None` when `root` cannot be read.
fn read_sessions(root: &Path, repo: &Path) -> Option<Vec<Session>> {
    std::fs::read_dir(root).ok()?;
    let repo = std::fs::canonicalize(repo).ok();
    let mut repositories: HashMap<String, Option<PathBuf>> = HashMap::new();
    let mut found = Vec::new();
    for path in projects(root).iter().flat_map(|p| sessions(p)) {
        let mut place = Place::default();
        let mut events = Vec::new();
        let mut checks: HashMap<String, bool> = HashMap::new();
        let mut messages = HashSet::new();
        let mut models = Models::default();
        let read = for_each_line(&path, |line| {
            place.observe(line, true);
            models.observe(line);
            let Some(at) = timestamp(line) else {
                return;
            };
            events.extend(tokens(line, &mut messages).map(|t| (at, t)));
            if let Some(text) = typed_text(line) {
                events.push((
                    at,
                    if is_validation(&text) {
                        Event::Validation
                    } else {
                        Event::Other
                    },
                ));
            }
            for block in line["message"]["content"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                let id = block["id"]
                    .as_str()
                    .or(block["tool_use_id"].as_str())
                    .unwrap_or_default()
                    .to_string();
                if block["type"] == "tool_use" && block["name"] == "Bash" {
                    checks.insert(
                        id,
                        is_check(block["input"]["command"].as_str().unwrap_or_default()),
                    );
                } else if block["type"] == "tool_result"
                    && block["is_error"] == Value::Bool(true)
                    && checks.get(&id) == Some(&true)
                {
                    events.push((at, Event::FailedCheck));
                }
            }
        });
        if let Err(e) = read {
            eprintln!("cannot read {}: {e}", path.display());
        }
        for sub in subagents(&path) {
            let _ = for_each_line(&sub, |line| {
                place.observe(line, false);
                models.observe(line);
                // only an assistant line can carry tokens: the others skip the parse of their timestamp
                let at = (line["type"] == "assistant")
                    .then(|| timestamp(line))
                    .flatten();
                if let Some(at) = at {
                    events.extend(tokens(line, &mut messages).map(|t| (at, t)));
                }
            });
        }
        let (Some(cwd), Some((start, _)), Some(end)) =
            (place.cwd.clone(), place.start.clone(), place.end)
        else {
            continue;
        };
        let resolved = repositories
            .entry(cwd.clone())
            .or_insert_with(|| repository(&cwd, &place.issues));
        if repo.is_some() && *resolved == repo {
            found.push(Session {
                branches: place.issues,
                setup: models.setup(),
                start,
                end,
                events,
            });
        }
    }
    Some(found)
}

/// The transcript columns of an issue (QUA-7, QUA-8, QUA-15): its sessions, then the failed checks, validations and
/// other typed messages of their spans that belong to it, and the input, cache read and output tokens of their messages;
/// with the setup of its sessions, `mixed` when they differ (QUA-16).
fn session_cells(
    log: &[Commit],
    own: &[usize],
    stem: &str,
    sessions: &[Session],
) -> ([u64; 7], Option<String>) {
    let on_branch = |s: &Session| s.branches.iter().any(|b| b == stem);
    let mut by_session: BTreeMap<usize, Vec<DateTime<FixedOffset>>> = BTreeMap::new();
    for &i in own {
        let at = log[i].date;
        let holding: Vec<(usize, &Session)> = sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.start <= at && at <= s.end)
            .collect();
        let preferred = holding
            .iter()
            .filter(|(_, s)| on_branch(s))
            .max_by_key(|(_, s)| s.start);
        if let Some((k, _)) = preferred.or_else(|| holding.iter().max_by_key(|(_, s)| s.start)) {
            by_session.entry(*k).or_default().push(at);
        }
    }
    let setups: BTreeSet<&str> = by_session
        .keys()
        .filter_map(|&k| sessions[k].setup.as_deref())
        .collect();
    let setup = match setups.len() {
        0 => None,
        1 => setups.first().map(|s| (*s).to_string()),
        _ => Some("mixed".to_string()),
    };
    let mut cells = [by_session.len() as u64, 0, 0, 0, 0, 0, 0];
    // a resumed session copies the messages of the one it resumes, ids included: each counts once for the issue
    let mut counted: HashSet<&str> = HashSet::new();
    for (k, dates) in &by_session {
        let session = &sessions[*k];
        let (Some(&first), Some(&last)) = (dates.iter().min(), dates.iter().max()) else {
            continue;
        };
        let other_work = log
            .iter()
            .enumerate()
            .filter(|(i, c)| !own.contains(i) && c.date >= session.start && c.date < first)
            .map(|(_, c)| c.date)
            .max();
        let after = other_work.filter(|_| !on_branch(session));
        let inside = |at: &DateTime<FixedOffset>| {
            after.map_or(*at >= session.start, |a| *at > a) && *at <= last
        };
        for (_, event) in session.events.iter().filter(|(at, _)| inside(at)) {
            match event {
                Event::FailedCheck => cells[1] += 1,
                Event::Validation => cells[2] += 1,
                Event::Other => cells[3] += 1,
                Event::Tokens(id, t) => {
                    if counted.insert(id) {
                        for (cell, n) in cells[4..].iter_mut().zip(t) {
                            *cell += n;
                        }
                    }
                }
            }
        }
    }
    (cells, setup)
}

struct Row {
    issue: String,
    /// The author date of its oldest commit, which orders the revisions of the summary.
    first: DateTime<FixedOffset>,
    closed: DateTime<FixedOffset>,
    cells: Vec<String>,
}

/// A count, empty when unknown.
fn cell(value: Option<impl ToString>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

fn issue_row(
    repo: &Path,
    log: &[Commit],
    sessions: Option<&[Session]>,
    path: &Path,
    window: Duration,
    docs: &DocFiles,
    blames: &mut HashMap<String, HashSet<String>>,
) -> Option<Row> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let own = issue_commits(repo, log, &stem);
    let (&first, &close) = (own.first()?, own.last()?);
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mine: Vec<&Commit> = own.iter().map(|&i| &log[i]).collect();
    let files: BTreeSet<&str> = mine
        .iter()
        .flat_map(|c| c.paths.iter().map(String::as_str))
        .filter(|p| !unrelated(p))
        .collect();
    let end = log[close].date + window;
    let later: Vec<&Commit> = log[close + 1..].iter().filter(|c| c.date <= end).collect();
    let fixes_after = later
        .iter()
        .filter(|c| c.kind() == "fix" && c.paths.iter().any(|p| !unrelated(p)))
        .filter(|c| {
            let writers = blames
                .entry(c.hash.clone())
                .or_insert_with(|| blamed(repo, &c.hash));
            mine.iter().any(|m| writers.contains(&m.hash))
        })
        .count();
    let reverts_after = later.iter().filter(|c| reverts(c, &files, &mine)).count();
    let fixes_in = mine
        .iter()
        .filter(|c| matches!(c.kind(), "fix" | "revert"))
        .count();
    let (review_count, findings) = reviews(&text);
    let traced = traced(repo, &mine);
    let without_red = traced
        .as_ref()
        .zip(red_ids(&text))
        .map(|(t, red)| t.difference(&red).count());
    let lines = doc_spec_lines(repo, &mine, docs);
    let (transcript, setup) = sessions.map(|s| session_cells(log, &own, &stem, s)).unzip();
    let mut cells = vec![
        revision(repo, &(log[first].date - Duration::seconds(1)).to_rfc3339()),
        setup.flatten().unwrap_or_default(),
        log[close].date.to_rfc3339(),
        cell(deliverables(&text)),
        own.len().to_string(),
        fixes_in.to_string(),
        fixes_after.to_string(),
        reverts_after.to_string(),
        cell(review_count),
        cell(findings),
        cell(traced.map(|t| t.len())),
        cell(without_red),
        cell(lines.map(|l| l.0)),
        cell(lines.map(|l| l.1)),
    ];
    // the tokens of an issue no session holds are unknown, not 0 (QUA-15)
    let known = |k: usize| k < TOKENS || transcript.is_some_and(|t| t[0] > 0);
    cells.extend((0..7).map(|k| cell(transcript.filter(|_| known(k)).map(|t| t[k]))));
    Some(Row {
        issue: stem,
        first: log[first].date,
        closed: log[close].date,
        cells,
    })
}

/// The index, in the transcript cells of `session_cells`, of the first token column (QUA-15).
const TOKENS: usize = 4;

/// Per revision and setup: its count of issues, the sum of each numeric column and the count of issues of known tokens.
type SetupSums = (usize, Vec<Option<u64>>, u64);

/// The rows summed per revision and setup (QUA-9, QUA-17), in the order the revisions were first used, then by setup:
/// the count of issues, then the sum of each numeric column, empty when one of its values is, but the token columns,
/// summed over the issues whose tokens are known and followed by the count of those issues.
fn summary(rows: &[Row]) -> Vec<String> {
    let mut first_use: HashMap<&str, DateTime<FixedOffset>> = HashMap::new();
    let mut by_pair: BTreeMap<(&str, &str), SetupSums> = BTreeMap::new();
    for row in rows {
        let values: Vec<Option<u64>> = row.cells[3..].iter().map(|v| v.parse().ok()).collect();
        let tokens = values.len() - 3;
        let first = first_use.entry(&row.cells[0]).or_insert(row.first);
        *first = (*first).min(row.first);
        let (count, sums, known) = by_pair
            .entry((&row.cells[0], &row.cells[1]))
            .or_insert_with(|| (0, vec![Some(0); values.len()], 0));
        *count += 1;
        *known += u64::from(values[tokens].is_some());
        for (k, (sum, value)) in sums.iter_mut().zip(values).enumerate() {
            *sum = if k >= tokens {
                sum.map(|a| a + value.unwrap_or(0))
            } else {
                sum.zip(value).map(|(a, b)| a + b)
            };
        }
    }
    let mut ordered: Vec<_> = by_pair.into_iter().collect();
    ordered.sort_by_key(|((revision, setup), _)| (first_use[revision], *revision, *setup));
    ordered
        .into_iter()
        .map(|((revision, setup), (count, sums, known))| {
            let sums: Vec<String> = sums
                .iter()
                .map(|s| s.map(|n| n.to_string()).unwrap_or_default())
                .collect();
            format!("{revision}\t{setup}\t{count}\t{}\t{known}", sums.join("\t"))
        })
        .collect()
}

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    let mut repo = PathBuf::from(".");
    let mut window_days = 30;
    let mut projects_dir = None;
    let mut summed = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--window-days" => {
                if let Some(n) = rest
                    .next()
                    .and_then(|n| n.parse::<i64>().ok())
                    .filter(|n| (0..=3650).contains(n))
                {
                    window_days = n;
                } else {
                    eprintln!("--window-days needs a whole number of days from 0 to 3650");
                    std::process::exit(2);
                }
            }
            "--projects" => {
                if let Some(dir) = rest.next() {
                    projects_dir = Some(dir.clone());
                } else {
                    eprintln!("--projects needs a directory");
                    std::process::exit(2);
                }
            }
            "--summary" => summed = true,
            other if other.starts_with("--") => {
                eprintln!("unknown option {other}");
                std::process::exit(2);
            }
            other => repo = PathBuf::from(other),
        }
    }
    let log = commits(&repo);
    let sessions = projects_root(projects_dir.as_ref())
        .ok()
        .and_then(|root| read_sessions(&root, &repo));
    let docs = DocFiles::read(&repo);
    let mut blames = HashMap::new();
    let mut rows: Vec<Row> = sorted_entries(&repo.join("issues/done"))
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| {
            issue_row(
                &repo,
                &log,
                sessions.as_deref(),
                p,
                Duration::days(window_days),
                &docs,
                &mut blames,
            )
        })
        .collect();
    rows.sort_by(|a, b| a.closed.cmp(&b.closed).then_with(|| a.issue.cmp(&b.issue)));
    if summed {
        println!("{SUMMARY_HEADER}");
        for line in summary(&rows) {
            println!("{line}");
        }
        return 0;
    }
    println!("{HEADER}");
    for row in rows {
        println!("{}\t{}", row.issue, row.cells.join("\t"));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    fn commit(date: &str) -> Commit {
        Commit {
            hash: String::new(),
            date: at(date),
            subject: String::new(),
            body: String::new(),
            paths: Vec::new(),
            added: Vec::new(),
        }
    }

    fn session(start: &str, end: &str, events: &[(&str, &str, u64)]) -> Session {
        Session {
            branches: vec!["x".to_string()],
            setup: None,
            start: at(start),
            end: at(end),
            events: events
                .iter()
                .map(|&(ts, id, n)| (at(ts), Event::Tokens(id.to_string(), [n, 0, n])))
                .collect(),
        }
    }

    // a resumed session needs two transcripts sharing a message and a commit in each: shorter built from its structs
    // spec: QUA-15 (internal: the resumed session is built from its structs)
    #[test]
    fn a_message_copied_into_a_resumed_session_counts_once() {
        let log = [
            commit("2026-01-01T02:00:00Z"),
            commit("2026-01-02T02:00:00Z"),
        ];
        let first = session(
            "2026-01-01T00:00:00Z",
            "2026-01-01T03:00:00Z",
            &[("2026-01-01T01:00:00Z", "m1", 10)],
        );
        let resumed = session(
            "2026-01-01T00:00:00Z",
            "2026-01-02T03:00:00Z",
            &[
                ("2026-01-01T01:00:00Z", "m1", 10),
                ("2026-01-02T01:00:00Z", "m2", 5),
            ],
        );
        // the copy starts the resumed session as early as the first: the first starting a second later holds the
        // first commit, the resumed one the second
        let first = Session {
            start: at("2026-01-01T00:00:01Z"),
            ..first
        };
        let (cells, _) = session_cells(&log, &[0, 1], "x", &[first, resumed]);
        assert_eq!(cells[0], 2);
        assert_eq!(cells[4..], [15, 0, 15]);
    }
}
