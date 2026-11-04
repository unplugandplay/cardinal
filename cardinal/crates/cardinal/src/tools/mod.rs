//! The tools of Cardinal, run by `cardinal tool <name>` (and so by `cardinal run <name>`, which picks the binary
//! of the pinned revision). Each reads its arguments, standard input and current directory, and returns its exit code.

use std::path::{Path, PathBuf};
use std::process::Command;

mod audit_sources;
mod cardinal_changes;
mod cardinal_report;
mod check_commits;
mod check_issue;
mod check_specs;
mod close_issue;
mod commit_check;
mod deliverable_size;
mod delivered_quality;
mod fixed_context;
mod format_code;
mod issue_claim;
mod language;
mod language_hint;
mod lint_gate;
mod next_issue;
mod read_hint;
mod release;
mod repo_files;
mod review_gate;
mod review_target;
mod roadmap;
mod rule_breaches;
mod session_metrics;
mod shell;
mod spec_coverage;
mod transcripts;
mod workspace_lock;
mod worktree;

pub(crate) use fixed_context::description_bytes;

/// A tool: its name on the command line, a summary, and its entry point.
pub struct Tool {
    pub name: &'static str,
    pub about: &'static str,
    pub main: fn(&[String]) -> u8,
}

/// Every tool, by name.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "audit-sources",
        about: "[projects_dir] [--messages]: the instruction sources and typed messages of every project",
        main: audit_sources::main,
    },
    Tool {
        name: "cardinal-changes",
        about: "[repo] --sessions <file>: the expected effects of the Cardinal changes, with the revisions holding each",
        main: cardinal_changes::main,
    },
    Tool {
        name: "cardinal-report",
        about: "--sessions <file> [...]: the markdown report of the readers, by group of revisions",
        main: cardinal_report::main,
    },
    Tool {
        name: "check-commits",
        about: "validate the message of every non-merge commit of <base>..<head>",
        main: check_commits::main,
    },
    Tool {
        name: "check-issue",
        about: "validate a new issue file against the create-issue template",
        main: check_issue::main,
    },
    Tool {
        name: "check-specs",
        about: "[--strict] [root]: validate specs/*.md and trace their requirements to the tests",
        main: check_specs::main,
    },
    Tool {
        name: "close-issue",
        about: "move a finished issue to issues/done/ and repoint the documents that cite it",
        main: close_issue::main,
    },
    Tool {
        name: "commit-check",
        about: "validate a commit message read from stdin against the commit convention",
        main: commit_check::main,
    },
    Tool {
        name: "deliverable-size",
        about: "[<rev> | <a>..<b>]: the changed lines of a deliverable or an issue against the size limit",
        main: deliverable_size::main,
    },
    Tool {
        name: "delivered-quality",
        about: "[repo] [--summary]: one row per done issue, its fixes, reviews and failed checks",
        main: delivered_quality::main,
    },
    Tool {
        name: "fixed-context",
        about: "[projects_dir]: the bytes every session pays, per project and Cardinal revision",
        main: fixed_context::main,
    },
    Tool {
        name: "format-code",
        about: "PostToolUse hook: format an edited file with the project's format command, else cargo fmt",
        main: format_code::main,
    },
    Tool {
        name: "issue-claim",
        about: "issues/<stem>.md: claim an issue for the Claude Code session that works it",
        main: issue_claim::main,
    },
    Tool {
        name: "language-hint",
        about: "Stop hook: send the agent back once when its last reply is not in the user_language",
        main: language_hint::main,
    },
    Tool {
        name: "lint-gate",
        about: "Stop hook: block the end of a turn while the project's lint, else clippy, reports a problem",
        main: lint_gate::main,
    },
    Tool {
        name: "next-issue",
        about: "the first ready issue no claim, branch or worktree has taken, and the blocked ones",
        main: next_issue::main,
    },
    Tool {
        name: "read-hint",
        about: "PreToolUse hook: send the agent back once before a whole Read of a file over 25,000 bytes",
        main: read_hint::main,
    },
    Tool {
        name: "release",
        about: "[<version> [--tag]]: propose the version, write its CHANGELOG.md section, then tag it locally",
        main: release::main,
    },
    Tool {
        name: "review-gate",
        about: "record | check | native: the code review gate of a commit",
        main: review_gate::main,
    },
    Tool {
        name: "review-target",
        about: "[<base>]: the target of a code review, without the issue files and the managed copies",
        main: review_target::main,
    },
    Tool {
        name: "rule-breaches",
        about: "[projects_dir] [--summary]: where the agent broke a rule a program can check",
        main: rule_breaches::main,
    },
    Tool {
        name: "session-metrics",
        about: "[projects_dir] [--archive <file>]: one row of numbers per session of every project",
        main: session_metrics::main,
    },
    Tool {
        name: "spec-coverage",
        about: "[root]: the test files that cite no requirement of an accepted spec, largest first",
        main: spec_coverage::main,
    },
    Tool {
        name: "workspace-lock",
        about: "check | start | release: the hooks that keep one Claude Code session per worktree",
        main: workspace_lock::main,
    },
    Tool {
        name: "worktree",
        about: "open <name> [--path <dir>] [--branch <branch>] | close <name|path> [--force]: the user's worktrees",
        main: worktree::main,
    },
];

/// Run the tool `args[0]` with the rest; exit 3 with the list of the tools when there is none or it is unknown (BIN-7).
#[must_use]
pub fn run(args: &[String]) -> u8 {
    let found = args
        .first()
        .and_then(|name| TOOLS.iter().find(|t| t.name == name));
    if let Some(tool) = found {
        return (tool.main)(&args[1..]);
    }
    match args.first() {
        Some(name) => eprintln!("cardinal: unknown tool `{name}`; the tools are:"),
        None => eprintln!("cardinal: name a tool; the tools are:"),
    }
    for tool in TOOLS {
        eprintln!("  {:<16} {}", tool.name, tool.about);
    }
    crate::NOT_A_REFUSAL
}

/// The fenced code block open after `line`, given the one `open` before it (its character and length): a line
/// starting, indent allowed, with three or more backticks (none after them, else it is inline code) or tildes opens
/// one; a line of only its character, at least as many, closes it.
fn fence(open: Option<(char, usize)>, line: &str) -> Option<(char, usize)> {
    let Some((c, n)) = open else {
        let line = line.trim_start();
        let c = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let rest = line.trim_start_matches(c);
        let n = line.len() - rest.len();
        return (n >= 3 && !(c == '`' && rest.contains('`'))).then_some((c, n));
    };
    let line = line.trim();
    open.filter(|_| line.len() < n || !line.chars().all(|x| x == c))
}

/// Whether each line of `lines` belongs to a fenced code block, its fence lines included.
fn fenced<'a>(lines: impl IntoIterator<Item = &'a str>) -> impl Iterator<Item = (&'a str, bool)> {
    let mut open = None;
    lines.into_iter().map(move |line| {
        let before = open;
        open = fence(open, line);
        (line, before.is_some() || open.is_some())
    })
}

/// Root of the repository holding the current directory.
fn repo_root() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("cannot run git to find the repository root: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository: cannot locate .claude/cardinal.toml".into());
    }
    let root = String::from_utf8_lossy(&out.stdout);
    Ok(root.strip_suffix('\n').unwrap_or(&root).into())
}

/// The root of the work tree holding `dir`, empty outside git.
fn toplevel(dir: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// A command a project declares for a hook, run by `sh -c`, or its default (`None`), and the file-name patterns of the
/// files it acts on, every file when empty; `root` is the work tree it was read from, empty outside git.
struct Declared {
    root: String,
    command: Option<String>,
    patterns: Vec<String>,
}

impl Declared {
    /// The string `command_key` and the list `files_key` of the `[project]` table of the `.claude/cardinal.toml` of the
    /// work tree of `dir`; the default, over `default_patterns`, when the project declares no command, or when the file
    /// is missing, does not parse or gives a key of another type (CLG-7, FMT-1). A blank command is read as empty: it
    /// turns the hook off (CLG-1, FMT-2).
    fn of(dir: &Path, command_key: &str, files_key: &str, default_patterns: &[&str]) -> Declared {
        let root = toplevel(dir);
        let default = Declared {
            root: root.clone(),
            command: None,
            patterns: default_patterns.iter().map(|&p| p.into()).collect(),
        };
        if root.is_empty() {
            return default;
        }
        let Ok(table) = std::fs::read_to_string(crate::config_of(Path::new(&root)))
            .unwrap_or_default()
            .parse::<toml::Table>()
        else {
            return default;
        };
        let project = table.get("project");
        let Some(command) = project
            .and_then(|p| p.get(command_key))
            .and_then(toml::Value::as_str)
        else {
            return default;
        };
        let patterns = match project.and_then(|p| p.get(files_key)) {
            None => Some(Vec::new()),
            Some(files) => files.as_array().and_then(|a| {
                a.iter()
                    .map(|v| v.as_str().map(String::from))
                    .collect::<Option<Vec<_>>>()
            }),
        };
        match patterns {
            Some(patterns) => Declared {
                root,
                command: Some(command.trim().into()),
                patterns,
            },
            None => default,
        }
    }

    /// Whether the command acts on the file `path`, relative to `root`: it is outside the `.claude` directories, whose
    /// files are Cardinal's copies, and its name matches one of the patterns, or there are none.
    fn acts_on(&self, path: &str) -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        if path.split('/').any(|c| c == ".claude") {
            return false;
        }
        self.patterns.is_empty()
            || self
                .patterns
                .iter()
                .any(|p| check_specs::name_matches(p, name))
    }
}

/// Strings of `key = ["a", "b"]` in the `[project]` table of the configuration (`crate::config_of`); empty when the key is absent.
fn config_list(root: &Path, key: &str) -> Result<Vec<String>, String> {
    let path = crate::config_of(root);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("invalid {}: {e}", path.display()))?;
    let list = table
        .get("project")
        .and_then(|p| p.get(key))
        .or_else(|| table.get(key));
    Ok(list
        .and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default())
}
