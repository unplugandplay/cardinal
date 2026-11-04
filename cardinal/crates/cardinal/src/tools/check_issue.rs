//! Validate a new issue file against the template of `skill:create-issue`, its `priority` and `depends_on` included
//! (`specs/check-issue.md`).
//! Usage: `just check-issue issues/foo.md`. Prints one line per problem, exit 1 if any.

use crate::features;
use std::path::Path;

use super::next_issue;

/// True when the section exists and has no text before the next `# ` heading.
fn section_is_empty(text: &str, heading: &str) -> bool {
    let mut lines = text.lines();
    lines.by_ref().any(|l| l.trim() == heading)
        && lines
            .take_while(|l| !l.starts_with("# "))
            .all(|l| l.trim().is_empty())
}

/// The problems of the keys that order the issue (CKI-1, CKI-2): its `priority` and its `depends_on`.
fn front_matter_problems(dir: &Path, stem: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    if next_issue::priority(text).is_none() {
        out.push(
            "no priority: set `priority: high`, `medium` or `low` in the front matter".to_string(),
        );
    }
    let Some(deps) = next_issue::depends_on(text) else {
        out.push("`depends_on` is not a list `[<stem>, <stem>...]` on one line".to_string());
        return out;
    };
    for dep in deps {
        if dep == stem {
            out.push("`depends_on` names the issue itself".to_string());
        } else if ![
            dir.join(format!("{dep}.md")),
            dir.join("done").join(format!("{dep}.md")),
        ]
        .iter()
        .any(|p| p.is_file())
        {
            out.push(format!(
                "`depends_on` names `{dep}`: no `{dep}.md` in `issues/` nor in `issues/done/`"
            ));
        }
    }
    out
}

/// The directory of the file, resolved so that a file named from its own directory has one too.
fn issue_dir(path: &Path) -> std::path::PathBuf {
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

fn problems(path: &Path, text: &str) -> Vec<String> {
    let dir = issue_dir(path);
    // a done issue always fails the checks of a new one: one problem says why (CKI-4)
    if dir.ends_with("issues/done") {
        return vec!["a done issue: check-issue checks a new issue of issues/".to_string()];
    }
    let mut out = Vec::new();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let stem = name.strip_suffix(".md").unwrap_or("");
    let snake = !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !snake || dir.file_name().is_none_or(|d| d != "issues") {
        out.push("file must be `issues/<snake_case>.md`".to_string());
    }
    if dir.join("done").join(name).exists() {
        out.push(format!(
            "`issues/done/{name}` already exists: pick another name"
        ));
    }
    let headings: Vec<&str> = text.lines().filter(|l| l.starts_with("# ")).collect();
    match headings.first() {
        None => out.push("no `# <Title>` heading".to_string()),
        Some(t) if t.trim() == "# Goal" => {
            out.push("first heading must be the title, not `# Goal`".to_string());
        }
        Some(_) => {}
    }
    for required in ["# Goal", "# Approach", "# Resources"] {
        if !headings.iter().any(|h| h.trim() == required) {
            out.push(format!("missing section `{required}`"));
        }
    }
    out.extend(front_matter_problems(&dir, stem, text));
    if headings.iter().any(|h| h.trim() == "# Tracking") {
        out.push("`# Tracking` is written by issue-workflow".to_string());
    }
    if section_is_empty(text, "# Goal") {
        out.push("`# Goal` is empty".to_string());
    }
    // `# Specs` is optional, but when present it must name something.
    if section_is_empty(text, "# Specs") {
        out.push(
            "`# Specs` is empty: name the spec and requirement ids, or drop the section"
                .to_string(),
        );
    }
    // the root of the project is the nearest ancestor holding its configuration (FEA-9)
    let root = crate::run::project_root(&dir, None);
    let attribution = !root.is_ok_and(|root| features::is_off(&root, features::NO_AI_ATTRIBUTION));
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.contains("/home/") || line.contains("~/") || line.contains("/Users/") {
            out.push(format!("line {n}: private path"));
        }
        if attribution && line.to_ascii_lowercase().contains("co-authored-by") {
            out.push(format!("line {n}: attribution line"));
        }
    }
    out
}

pub(super) fn main(args: &[String]) -> u8 {
    let Some(arg) = args.first().cloned() else {
        eprintln!("usage: cardinal run check-issue issues/<name>.md");
        std::process::exit(2);
    };
    let path = Path::new(&arg);
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("cannot read {arg}");
        std::process::exit(2);
    };
    let found = problems(path, &text);
    for p in &found {
        println!("{arg}: {p}");
    }
    std::process::exit(i32::from(!found.is_empty()));
}
