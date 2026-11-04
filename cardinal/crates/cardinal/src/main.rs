//! `cardinal`: copy the shared agent harness into a project and tell when a managed file drifted.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cardinal::{
    budget_warnings, check, init, retention_warnings, run, sync, tools, update, upgrade, Outcome,
    Report, SyncOptions, NOT_A_REFUSAL,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Project root (default: the current directory).
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create `.claude/cardinal.toml` pointing at a source (a directory, or a git URL with `--rev`).
    Init {
        source: String,
        /// Branch, tag or commit of a git source.
        #[arg(long)]
        rev: Option<String>,
    },
    /// Copy the managed files of the source into the project and seed the missing project-owned ones.
    Sync {
        /// Overwrite managed files modified locally.
        #[arg(long)]
        force: bool,
        /// Print what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Move the project to a revision of its git source (default: the head of its default branch), then sync; refuses a revision whose hooks run a subcommand the installed `cardinal` does not know.
    Update {
        /// Branch, tag or commit; `rev` stores the resolved commit.
        #[arg(long)]
        rev: Option<String>,
        /// Overwrite managed files modified locally.
        #[arg(long)]
        force: bool,
        /// Print the target revision and what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Fail when a managed file was modified or deleted since the last sync, the git review gate is missing or edited (not looked at when `CI` is set), or a hook runs a subcommand the `cardinal` on the path refuses; warn when the settings lack a deny rule of the review waiver, run a missing hook or leave the attribution on, or Claude Code keeps the transcripts less than a year.
    Check,
    /// Run a tool of Cardinal (hook, check, transcript reader) with the `cardinal` binary of the pinned revision; with no tool, list them.
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        tool: Vec<OsString>,
    },
    /// Replace this `cardinal` with the one of a commit of a git source: its release asset, checked against its SHA-256, else built with `cargo`.
    Upgrade {
        /// The git source, as `source` of `.claude/cardinal.toml` names it.
        source: String,
        /// The full commit hash, as `rev` of `.claude/cardinal.toml` pins it.
        commit: String,
    },
    /// Run a tool of this binary (what `run` executes once it picked the binary).
    #[command(hide = true)]
    Tool {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        tool: Vec<String>,
    },
}

/// Replace this process by the tool of the pinned revision, so its input, output and exit code are the tool's (BIN-1).
fn run(root: &Path, tool: &[OsString]) -> ExitCode {
    let project = std::env::var_os("CLAUDE_PROJECT_DIR").map(PathBuf::from);
    let found = run::command(
        root,
        project.as_deref(),
        None,
        &run::Programs::default(),
        tool,
    );
    let mut cmd = match found {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("cardinal run: {e:#}");
            return ExitCode::from(NOT_A_REFUSAL);
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = cmd.exec();
        eprintln!("cardinal run: cannot run the cardinal binary: {e}");
        ExitCode::from(NOT_A_REFUSAL)
    }
    #[cfg(not(unix))]
    match cmd.status() {
        Ok(status) => ExitCode::from(status.code().map_or(NOT_A_REFUSAL, |c| c as u8)),
        Err(e) => {
            eprintln!("cardinal run: cannot run the cardinal binary: {e}");
            ExitCode::from(NOT_A_REFUSAL)
        }
    }
}

/// Print what changed, and the warnings on the settings to stderr; true when nothing is in conflict.
fn print_changes(report: &Report) -> bool {
    for (path, outcome) in &report.entries {
        if *outcome != Outcome::Unchanged {
            println!("{outcome}: {path}");
        }
    }
    for warning in &report.warnings {
        eprintln!("{warning}");
    }
    report.conflicts() == 0
}

/// The skill `update` names as its next step (SYN-61): the agent otherwise writes the commit message inline.
const COMMIT_SKILL: &str = ".claude/skills/commit-message/SKILL.md";

#[must_use]
pub fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // `--help` and `--version` print to stdout and exit 0; a real parse error goes to stderr.
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(if e.use_stderr() { NOT_A_REFUSAL } else { 0 });
        }
    };
    let warns_of_retention = !matches!(cli.command, Command::Init { .. } | Command::Upgrade { .. });
    let result = match cli.command {
        Command::Init { source, rev } => init(&cli.root, &source, rev.as_deref()).map(|()| true),
        Command::Sync { force, dry_run } => {
            sync(&cli.root, &SyncOptions { force, dry_run }).map(|report| print_changes(&report))
        }
        Command::Update {
            rev,
            force,
            dry_run,
        } => update(&cli.root, rev.as_deref(), &SyncOptions { force, dry_run }).map(|done| {
            println!(
                "rev: {} -> {}",
                done.previous.as_deref().unwrap_or("(none)"),
                done.rev
            );
            let clean = print_changes(&done.sync);
            if clean && dry_run {
                println!("dry run: nothing written");
            } else if !clean {
                println!("conflicts: nothing written, `rev` unchanged (fix them or use --force)");
            }
            if done.changed && cli.root.join(COMMIT_SKILL).is_file() {
                println!("next: commit the update with skill:commit-message");
            }
            clean
        }),
        Command::Run { tool } => return run(&cli.root, &tool),
        Command::Tool { tool } => return ExitCode::from(tools::run(&tool)),
        Command::Upgrade { source, commit } => upgrade(&cli.root, &source, &commit).map(|exe| {
            println!("upgraded: {}", exe.display());
            true
        }),
        Command::Check => check(&cli.root).map(|checked| {
            for problem in &checked.problems {
                println!("{problem}");
            }
            for warning in &checked.warnings {
                eprintln!("{warning}");
            }
            checked.problems.is_empty()
        }),
    };
    if warns_of_retention && result.is_ok() {
        for warning in retention_warnings(&cli.root)
            .into_iter()
            .chain(budget_warnings(&cli.root))
        {
            eprintln!("{warning}");
        }
    }
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(NOT_A_REFUSAL)
        }
    }
}
