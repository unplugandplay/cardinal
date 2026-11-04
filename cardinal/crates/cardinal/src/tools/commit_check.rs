//! `commit-check`: validate a commit message read from stdin against the commit convention
//! (`.claude/skills/commit-message/conventional-commits.md`).
//! Prints each violation and exits 1; prints nothing and exits 0 when the message is valid.
//! Exits 2 with an `error:` line on stderr when `.claude/cardinal.toml` cannot be read from the repository root.
//! Usage: `printf '%s\n' "$msg" | cardinal run commit-check`, or `just check-commit` (checks the message of HEAD).

use std::io::Read;

use super::cardinal_changes::{expectation, METRICS, RULE_METRIC, WASTE_METRIC};
use super::{config_list, repo_root};
use crate::features;

pub(super) const TYPES: [&str; 9] = [
    "feat", "fix", "refactor", "docs", "test", "perf", "build", "ci", "chore",
];
pub(super) const BUILT_IN_SCOPES: [&str; 2] = ["docs", "cardinal"];
const ATTRIBUTION: [&str; 3] = ["co-authored-by", "generated with", "noreply@anthropic.com"];

/// The built-in scopes plus the project's `commit_scopes`, read once from the repository root; `None` when the project
/// turns off the feature `conventional-commits`, its format then being the project's (FEA-21).
pub(super) fn scopes() -> Result<Option<Vec<String>>, String> {
    let root = repo_root()?;
    if features::is_off(&root, features::CONVENTIONAL_COMMITS) {
        return Ok(None);
    }
    let mut all: Vec<String> = BUILT_IN_SCOPES.iter().map(|s| (*s).to_string()).collect();
    all.extend(config_list(&root, "commit_scopes")?);
    Ok(Some(all))
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FFFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF)
}

fn check_subject(subject: &str, scopes: &[String], errors: &mut Vec<String>) {
    if subject.chars().count() > 72 {
        errors.push(format!(
            "subject is {} chars, the maximum is 72",
            subject.chars().count()
        ));
    }
    let Some((head, description)) = subject.split_once(": ") else {
        errors
            .push("format must be `<type>(<scope>): <description>` (colon then one space)".into());
        return;
    };
    let (ty, scope) = match head.split_once('(') {
        Some((ty, rest)) => {
            if let Some(scope) = rest.strip_suffix(')') {
                (ty, Some(scope))
            } else {
                errors.push("format: unclosed scope parenthesis".into());
                return;
            }
        }
        None => (head, None),
    };
    if !TYPES.contains(&ty) {
        errors.push(format!("unknown type `{ty}`, use one of {TYPES:?}"));
    }
    if let Some(scope) = scope.filter(|s| !scopes.iter().any(|k| k == s)) {
        errors.push(format!(
            "unknown scope `{scope}`, use one of {scopes:?} (or omit it)"
        ));
    }
    if description.chars().next().is_some_and(char::is_uppercase) {
        errors.push("description must start in lowercase".into());
    }
    if description.ends_with('.') {
        errors.push("description must not end with a period".into());
    }
}

/// Whether the project of the repository root refuses an AI or tool attribution: true unless it turns off the feature
/// `no-ai-attribution` (FEA-8).
pub(super) fn attribution_banned() -> bool {
    repo_root().map_or(true, |root| {
        !features::is_off(&root, features::NO_AI_ATTRIBUTION)
    })
}

/// The violations of the convention by the message of `lines`, line numbers counted from 1 (FEA-21).
fn convention_violations(lines: &[&str], scopes: &[String], errors: &mut Vec<String>) {
    check_subject(lines.first().copied().unwrap_or(""), scopes, errors);
    if lines.get(1).is_some_and(|l| !l.is_empty()) {
        errors.push("a blank line must separate the subject from the body".into());
    }
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        if i > 0 && line.chars().count() > 72 {
            errors.push(format!(
                "line {n} is {} chars, wrap at 72",
                line.chars().count()
            ));
        }
        if i > 0 && (line.starts_with('#') || line.starts_with("```")) {
            errors.push(format!("line {n}: no markdown headings or code fences"));
        }
        if line.chars().any(is_emoji) {
            errors.push(format!("line {n}: no emoji"));
        }
    }
}

/// The violations of `message`, empty when it is valid; an attribution is one only when `attribution` is banned, and
/// the convention is checked only when `scopes` is known (the feature `conventional-commits` on).
pub(super) fn violations(
    message: &str,
    scopes: Option<&[String]>,
    attribution: bool,
) -> Vec<String> {
    let lines: Vec<&str> = message.lines().collect();
    let mut errors = Vec::new();
    if let Some(scopes) = scopes {
        convention_violations(&lines, scopes, &mut errors);
    }
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let lower = line.to_lowercase();
        if attribution && ATTRIBUTION.iter().any(|a| lower.contains(a)) {
            errors.push(format!(
                "line {n}: no Co-Authored-By trailer or AI/tool attribution allowed"
            ));
        }
        if expectation(line).is_some_and(|e| e.is_err()) {
            errors.push(format!(
                "line {n}: write `Expected-effect: <metric> <up|down>`, the metric one of {METRICS:?}, `{RULE_METRIC}<rule>` or `{WASTE_METRIC}<rule>`"
            ));
        }
    }
    errors
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut message = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut message) {
        eprintln!("error: cannot read the message on stdin: {e}");
        return 2;
    }
    let scopes = match scopes() {
        Ok(scopes) => scopes,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let errors = violations(&message, scopes.as_deref(), attribution_banned());
    for e in &errors {
        println!("{e}");
    }
    u8::from(!errors.is_empty())
}
