//! List the sources of the instruction audit: per project of the transcripts directory, the sessions, the
//! messages typed by the user, the memory files and the instruction files of each working directory and of its
//! ancestors up to the repository root (AUD-1: the memory files a session loads at launch, `CLAUDE.local.md` and
//! its imports, `AGENTS.md`, `.claude/AGENTS.md` and the scoped rules), after the user-level instruction files of the
//! configuration directory (AUD-2). With
//! `--messages`, print the typed messages as `project<TAB>session<TAB>text` (newlines escaped); subagent
//! transcripts are skipped. Prints private content: never commit the output.
//! Usage: `cardinal run audit-sources [projects_dir] [--messages]`
//! (default the `projects/` of the configuration directory, EVA-15).

use super::transcripts;
use crate::memory::{instruction_files, WorkTree, PROJECT, USER};

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use transcripts::{for_each_line, projects, projects_root, sessions, sorted_entries, typed_text};

/// One line per message: tab, carriage return and backslash are escaped along with the newline.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// The instruction files of `cwd` and of its ancestors up to the root of its repository (AUD-1), sorted: a session
/// loads those above the repository too, but the audit walks no directory above it. The ancestors are read in the
/// tree of the repository, so that an import may climb to any of its directories. The user-level files, when the
/// configuration directory `config` is the `.claude/` of one of these directories, are left out: AUD-2 lists them.
fn instructions(cwd: &Path, config: Option<&Path>) -> BTreeSet<PathBuf> {
    // the working directory alone outside a repository
    let root = cwd
        .ancestors()
        .find(|d| d.join(".git").exists())
        .unwrap_or(cwd);
    let tree = WorkTree(root);
    cwd.ancestors()
        .take_while(|d| d.starts_with(root))
        .flat_map(|dir| {
            let rel = dir
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let user = (config == fs::canonicalize(dir.join(".claude")).ok().as_deref())
                .then(|| Path::new(&rel).join(".claude"));
            instruction_files(&tree, &rel, &PROJECT)
                .into_iter()
                .filter(move |f| !user.as_ref().is_some_and(|u| Path::new(f).starts_with(u)))
        })
        .map(|f| root.join(f))
        .collect()
}

/// The user-level instruction files (AUD-2): `CLAUDE.md`, the rules and their imports in the configuration directory
/// `config`.
fn user_instructions(config: Option<&Path>) -> BTreeSet<PathBuf> {
    let Some(config) = config else {
        return BTreeSet::new();
    };
    instruction_files(&WorkTree(config), "", &USER)
        .into_iter()
        .map(|f| config.join(f))
        .collect()
}

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    let print_messages = args.iter().any(|a| a == "--messages");
    if let Some(flag) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--messages")
    {
        eprintln!(
            "unknown option {flag}\nusage: cardinal run audit-sources [projects_dir] [--messages]"
        );
        std::process::exit(2);
    }
    let root = match projects_root(args.iter().find(|a| !a.starts_with('-'))) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    // canonical, so that the paths printed are absolute and compare with the working directories
    let config = crate::config_dir().and_then(|dir| fs::canonicalize(dir).ok());
    if !print_messages {
        for file in user_instructions(config.as_deref()) {
            println!("instructions: {}", file.display());
        }
    }
    for project in projects(&root) {
        let name = project.file_name().unwrap().to_string_lossy().into_owned();
        let (mut session_count, mut messages, mut cwds) = (0, 0, Vec::<String>::new());
        for session in sessions(&project) {
            session_count += 1;
            let stem = session.file_stem().unwrap().to_string_lossy().into_owned();
            let read = for_each_line(&session, |line| {
                if let Some(c) = line["cwd"]
                    .as_str()
                    .filter(|c| !cwds.iter().any(|k| k == c))
                {
                    cwds.push(c.to_string());
                }
                if let Some(t) = typed_text(line) {
                    messages += 1;
                    if print_messages {
                        println!("{name}\t{stem}\t{}", escape(&t));
                    }
                }
            });
            let skipped = match read {
                Ok(skipped) => skipped,
                Err(e) => {
                    eprintln!("cannot read {}: {e}", session.display());
                    continue;
                }
            };
            if skipped > 0 {
                eprintln!("{}: {skipped} unreadable lines skipped", session.display());
            }
        }
        if print_messages {
            continue;
        }
        println!("{name}: {session_count} sessions, {messages} messages");
        let is_note = |p: &PathBuf| {
            p.extension().is_some_and(|e| e == "md")
                && p.file_name().is_some_and(|n| n != "MEMORY.md")
        };
        let memory = sorted_entries(&project.join("memory"))
            .iter()
            .filter(|p| is_note(p))
            .count();
        if memory > 0 {
            println!("  memory: {memory} files");
        }
        for cwd in &cwds {
            for file in instructions(Path::new(cwd), config.as_deref()) {
                println!("  instructions: {}", file.display());
            }
        }
    }
    0
}
