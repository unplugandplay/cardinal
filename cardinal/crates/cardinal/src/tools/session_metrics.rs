//! Print one tab-separated row of numbers per session of the Claude Code transcripts (`specs/evaluation.md`): cost,
//! user interventions, and what may explain a difference (project, issue, Cardinal revision, model, version). The
//! sub-agent transcripts count with their session; the Cardinal revision comes from the git history of the
//! project's `.claude/cardinal.toml`. No text of a message is printed, so the rows can be archived.
//! With `--archive <file>`, also merge the rows into that file, so the history outlives the transcripts.
//! Usage: `cardinal run session-metrics [projects_dir] [--archive <file>]`
//! (default the `projects/` of the configuration directory, EVA-15).

use super::transcripts;

use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use transcripts::{
    for_each_line, is_validation, project_name, projects, projects_root, push_distinct, repository,
    result_text, revision, sessions, subagents, typed_text, Place,
};

pub(super) const HEADER: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\tmain_context_peak\tadvisor_models\tadvisor_calls\tadvisor_input_tokens\tadvisor_output_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";

/// The columns added since the first archives, newest last (EVA-17, EVA-20): an archive written before them is
/// upgraded, its rows given empty cells for the columns its header lacks.
const ADDED_COLUMNS: &[&[&str]] = &[
    &["main_context_peak"],
    &[
        "advisor_models",
        "advisor_calls",
        "advisor_input_tokens",
        "advisor_output_tokens",
    ],
];

/// The line types seen in the transcripts; any other is counted in `unknown_lines`, since the format moves.
const KNOWN_TYPES: &[&str] = &[
    "user",
    "assistant",
    "attachment",
    "system",
    "summary",
    "ai-title",
    "custom-title",
    "last-prompt",
    "queue-operation",
    "api-request",
    "api-request-blob",
    "api-request-shape",
    "atis-latch",
    "mode",
    "cost-state",
    "file-history-snapshot",
    "tools-used",
    "relocated",
    "worktree-state",
    "permission-mode",
];

/// The lines of the conversation: the only ones that must carry a `timestamp`.
const CONVERSATION_TYPES: &[&str] = &["user", "assistant", "attachment", "system"];

#[derive(Default)]
struct Session {
    place: Place,
    models: Vec<String>,
    advisors: Vec<String>,
    advisor_calls: u64,
    advisor_input: u64,
    advisor_output: u64,
    versions: Vec<String>,
    messages: HashSet<String>,
    turns: u64,
    tool_calls: u64,
    input: u64,
    output: u64,
    thinking: u64,
    cache_read: u64,
    cache_creation: u64,
    main_peak: u64,
    hooks: BTreeMap<String, u64>,
    skills: BTreeMap<String, u64>,
    validations: u64,
    others: u64,
    unreadable: u64,
    unknown: u64,
    missing: u64,
}

/// A refusal of a hook: `PreToolUse:<tool> hook error:`, the tool name holding no space.
fn is_refusal(text: &str) -> bool {
    text.strip_prefix("PreToolUse:")
        .and_then(|rest| rest.split_once(" hook error:"))
        .is_some_and(|(tool, _)| !tool.is_empty() && !tool.contains(char::is_whitespace))
}

/// The hook a refusal names: the tool of the first `cardinal run <tool>` (`harness run <tool>` before the rename) its
/// text cites, under the name of the script
/// it was (`-` written `_`, `review-gate` named `require_code_review`) so the counts compare across revisions; else
/// the stem of the first `.rs` file name it cites; `unknown` when none.
fn hook_name(text: &str) -> String {
    let name_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let cited = ["cardinal run ", "harness run "]
        .iter()
        .filter_map(|launcher| text.find(launcher).map(|at| at + launcher.len()))
        .min();
    if let Some(rest) = cited.map(|at| &text[at..]) {
        let tool: String = rest.chars().take_while(|c| name_char(*c)).collect();
        if !tool.is_empty() {
            return match tool.as_str() {
                "review-gate" => "require_code_review".to_string(),
                tool => tool.replace('-', "_"),
            };
        }
    }
    for (end, _) in text.match_indices(".rs") {
        let ends_name = !text[end + 3..].starts_with(name_char);
        let stem: String = text[..end]
            .chars()
            .rev()
            .take_while(|c| name_char(*c))
            .collect();
        if ends_name && !stem.is_empty() {
            return stem.chars().rev().collect();
        }
    }
    "unknown".into()
}

impl Session {
    fn read(&mut self, path: &Path, main: bool) {
        match for_each_line(path, |line| self.line(line, main)) {
            Ok(unreadable) => self.unreadable += unreadable as u64,
            Err(e) => eprintln!("cannot read {}: {e}", path.display()),
        }
    }

    fn line(&mut self, line: &Value, main: bool) {
        let kind = line["type"].as_str().unwrap_or_default();
        if !KNOWN_TYPES.contains(&kind) {
            self.unknown += 1;
        }
        if !self.place.observe(line, main) && CONVERSATION_TYPES.contains(&kind) {
            self.missing += 1;
        }
        push_distinct(&mut self.versions, line["version"].as_str());
        match kind {
            // a sub-agent written in the main transcript (older versions) is not the main context
            "assistant" => self.assistant(
                line,
                &line["message"],
                main && line["isSidechain"].as_bool() != Some(true),
            ),
            "user" => self.user(line, main),
            _ => {}
        }
    }

    fn assistant(&mut self, line: &Value, message: &Value, main: bool) {
        push_distinct(&mut self.advisors, line["advisorModel"].as_str());
        let blocks = message["content"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        self.advisor_calls += blocks
            .iter()
            .filter(|b| b["type"] == "server_tool_use" && b["name"] == "advisor")
            .count() as u64;
        for block in blocks.iter().filter(|b| b["type"] == "tool_use") {
            self.tool_calls += 1;
            if block["name"] == "Skill" {
                *self
                    .skills
                    .entry(
                        block["input"]["skill"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string(),
                    )
                    .or_default() += 1;
            }
        }
        let usage = &message["usage"];
        let Some(id) = message["id"].as_str().filter(|_| usage.is_object()) else {
            self.missing += 1;
            return;
        };
        if !self.messages.insert(id.to_string()) {
            return;
        }
        self.turns += 1;
        push_distinct(&mut self.models, message["model"].as_str());
        let tokens = |key: &str| usage[key].as_u64().unwrap_or(0);
        self.input += tokens("input_tokens");
        self.output += tokens("output_tokens");
        self.cache_read += tokens("cache_read_input_tokens");
        self.cache_creation += tokens("cache_creation_input_tokens");
        self.thinking += usage["output_tokens_details"]["thinking_tokens"]
            .as_u64()
            .unwrap_or(0);
        let iterations = usage["iterations"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if main {
            // An advised message's top-level usage adds up its executor iterations: its context is the largest one.
            let advised = iterations.iter().any(|i| i["type"] == "advisor_message");
            let context = iterations
                .iter()
                .filter(|i| advised && i["type"] == "message")
                .map(context_tokens)
                .max()
                .unwrap_or_else(|| context_tokens(usage));
            self.main_peak = self.main_peak.max(context);
        }
        for advice in iterations.iter().filter(|i| i["type"] == "advisor_message") {
            self.advisor_input += context_tokens(advice);
            self.advisor_output += advice["output_tokens"].as_u64().unwrap_or(0);
        }
    }

    fn user(&mut self, line: &Value, main: bool) {
        match typed_text(line).filter(|_| main) {
            Some(text) if is_validation(&text) => self.validations += 1,
            Some(_) => self.others += 1,
            None => {}
        }
        let blocks = line["message"]["content"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        for block in blocks
            .iter()
            .filter(|b| b["type"] == "tool_result" && b["is_error"].as_bool() == Some(true))
        {
            let text = result_text(block);
            if is_refusal(&text) {
                *self.hooks.entry(hook_name(&text)).or_default() += 1;
            }
        }
    }

    fn row(&self, id: &str, project: &str, revision: &str) -> String {
        let pairs = |map: &BTreeMap<String, u64>| {
            map.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(";")
        };
        let (start, duration) = match (&self.place.start, self.place.end) {
            (Some((s, text)), Some(e)) => (text.clone(), (e - *s).num_seconds().to_string()),
            _ => (String::new(), String::new()),
        };
        // no advisor set is not 0 calls (EVA-19)
        let advised = |n: u64| {
            if self.advisors.is_empty() {
                String::new()
            } else {
                n.to_string()
            }
        };
        [
            id.to_string(),
            project.to_string(),
            self.place.issues.join(","),
            revision.to_string(),
            self.models.join(","),
            self.versions.join(","),
            start,
            duration,
            self.turns.to_string(),
            self.tool_calls.to_string(),
            self.input.to_string(),
            self.output.to_string(),
            self.thinking.to_string(),
            self.cache_read.to_string(),
            self.cache_creation.to_string(),
            self.main_peak.to_string(),
            self.advisors.join(","),
            advised(self.advisor_calls),
            advised(self.advisor_input),
            advised(self.advisor_output),
            pairs(&self.hooks),
            pairs(&self.skills),
            self.validations.to_string(),
            self.others.to_string(),
            self.unreadable.to_string(),
            self.unknown.to_string(),
            self.missing.to_string(),
        ]
        .join("\t")
    }
}

/// The input and cache tokens of a usage or of one of its iterations: the size of the context it read.
fn context_tokens(usage: &Value) -> u64 {
    [
        "input_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
    ]
    .iter()
    .map(|key| usage[key].as_u64().unwrap_or(0))
    .sum()
}

/// The new row of an archived session, keeping the archived project and revision when the new one lost its revision
/// (the repository is gone or unreadable since).
fn keep_resolved(old: &str, new: &str) -> String {
    let (old, mut new): (Vec<&str>, Vec<&str>) =
        (old.split('\t').collect(), new.split('\t').collect());
    if new.get(3).is_some_and(|r| r.is_empty()) && old.get(3).is_some_and(|r| !r.is_empty()) {
        new[1] = old[1];
        new[3] = old[3];
    }
    new.join("\t")
}

/// The headers of the archives written before the columns of `ADDED_COLUMNS`, each with the columns it lacks.
fn older_headers() -> Vec<(String, Vec<&'static str>)> {
    (0..ADDED_COLUMNS.len())
        .map(|since| {
            let lacked: Vec<&str> = ADDED_COLUMNS[since..].concat();
            let header = HEADER
                .split('\t')
                .filter(|c| !lacked.contains(c))
                .collect::<Vec<_>>()
                .join("\t");
            (header, lacked)
        })
        .collect()
}

/// An archived row written before the `lacked` columns, given empty cells for them (EVA-17, EVA-20).
fn with_empty(row: &str, lacked: &[&str]) -> String {
    let mut cells = row.split('\t');
    HEADER
        .split('\t')
        .map(|c| {
            if lacked.contains(&c) {
                ""
            } else {
                cells.next().unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join("\t")
}

/// Merges `rows` into the archive at `path`: a session already there is replaced, one absent from `rows` is kept, and
/// the file stays one header then rows sorted by start. An archive written before a column of `ADDED_COLUMNS` is
/// upgraded; one with another header is left unchanged.
fn archive(path: &Path, rows: &[String]) -> Result<(), String> {
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let mut lines = existing.lines();
    let lacked = match lines.next() {
        Some(header) if header != HEADER => older_headers()
            .into_iter()
            .find(|(older, _)| older == header)
            .map(|(_, lacked)| lacked)
            .ok_or_else(|| {
                format!(
                    "{} has other columns than this script: archive elsewhere",
                    path.display()
                )
            })?,
        _ => Vec::new(),
    };
    let session = |row: &str| row.split('\t').next().unwrap_or_default().to_string();
    let start = |row: &str| row.split('\t').nth(6).unwrap_or_default().to_string();
    let mut merged: BTreeMap<String, String> = lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            let row = if lacked.is_empty() {
                l.to_string()
            } else {
                with_empty(l, &lacked)
            };
            (session(&row), row)
        })
        .collect();
    for row in rows {
        let new = match merged.get(&session(row)) {
            Some(old) => keep_resolved(old, row),
            None => row.clone(),
        };
        merged.insert(session(row), new);
    }
    let mut merged: Vec<String> = merged.into_values().collect();
    merged.sort_by_key(|r| start(r));
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let partial = path.with_extension("tsv.partial");
    let text = std::iter::once(HEADER.to_string())
        .chain(merged)
        .map(|l| l + "\n")
        .collect::<String>();
    std::fs::write(&partial, text)
        .and_then(|()| std::fs::rename(&partial, path))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

pub(super) fn main(args: &[String]) -> u8 {
    const USAGE: &str = "usage: cardinal run session-metrics [projects_dir] [--archive <file>]";
    let mut args = args.iter().cloned();
    let (mut dir, mut archive_path) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--archive" => {
                if let Some(file) = args.next() {
                    archive_path = Some(PathBuf::from(file));
                } else {
                    eprintln!("--archive needs a file\n{USAGE}");
                    std::process::exit(2);
                }
            }
            flag if flag.starts_with('-') => {
                eprintln!("unknown option {flag}\n{USAGE}");
                std::process::exit(2);
            }
            _ => dir = Some(arg),
        }
    }
    let root = match projects_root(dir.as_ref()) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut repositories: HashMap<String, Option<PathBuf>> = HashMap::new();
    let mut rows = Vec::new();
    for project in projects(&root) {
        for path in sessions(&project) {
            let mut session = Session::default();
            session.read(&path, true);
            for sub in subagents(&path) {
                session.read(&sub, false);
            }
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            let cwd = session.place.cwd.clone().unwrap_or_default();
            let repo = repositories
                .entry(cwd.clone())
                .or_insert_with(|| repository(&cwd, &session.place.issues));
            let start = session.place.start_text();
            let revision = repo
                .as_deref()
                .filter(|_| !start.is_empty())
                .map(|r| revision(r, start))
                .unwrap_or_default();
            rows.push(session.row(&id, &project_name(&cwd, repo.as_deref()), &revision));
        }
    }
    println!("{HEADER}");
    for row in &rows {
        println!("{row}");
    }
    if let Err(e) = archive_path.map_or(Ok(()), |path| archive(&path, &rows)) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    0
}
