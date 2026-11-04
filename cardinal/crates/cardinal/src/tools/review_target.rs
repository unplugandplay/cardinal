//! The target `skill:issue-workflow` hands to `/code-review`: the diff of a deliverable, or of a whole issue, without
//! the issue files, the lock nor the managed copies of `.claude/cardinal.lock`.
//! Usage: `cardinal run review-target [<base>]`: the working tree against `<base>` (RVT-1), `HEAD` with no argument
//! (RVT-2), followed by the exclusions of RVT-3, every word quoted for a shell (RVT-5); exits 3 on an error or when an
//! untracked file would escape the diff, printing nothing (RVT-4, RVT-6).

use std::path::Path;
use std::process::Command;

pub fn main(args: &[String]) -> u8 {
    let base = match args {
        [] => None,
        [base] => Some(base.as_str()),
        _ => {
            eprintln!("usage: review-target [<base>]");
            return crate::NOT_A_REFUSAL;
        }
    };
    match target(base) {
        Ok(line) => {
            println!("{line}");
            0
        }
        Err(e) => {
            eprintln!("review-target: {e}");
            crate::NOT_A_REFUSAL
        }
    }
}

/// The line of the target: the revision, then `--` and one quoted pathspec per word.
fn target(base: Option<&str>) -> Result<String, String> {
    if let Some(option) = base.filter(|b| b.starts_with('-')) {
        // git would read it as one of its options (RVT-4)
        return Err(format!("`{option}` is not a revision"));
    }
    let base = base.unwrap_or("HEAD");
    let root = super::repo_root()?;
    let root = Path::new(&root);
    git(
        root,
        &["rev-parse", "--verify", "-q", &format!("{base}^{{commit}}")],
    )
    .map_err(|_| format!("`{base}` is not a revision git resolves"))?;
    let managed = crate::managed_files(root).map_err(|e| format!("{e:#}"))?;
    let lock = root.join(crate::LOCK).exists().then_some(crate::LOCK);
    // with the issue files off, `issues/` is reviewed as any other directory (FEA-39)
    let issues = (!crate::features::is_off(root, crate::features::ISSUE_FILES)).then_some("issues");
    let mut excluded: Vec<&str> = lock
        .into_iter()
        .chain(managed.iter().map(String::as_str))
        .collect();
    excluded.sort_unstable();
    let pathspecs: Vec<String> = std::iter::once(":/".to_string())
        .chain(
            issues
                .into_iter()
                .chain(excluded)
                .map(|path| format!(":(top,exclude,literal){path}")),
        )
        .collect();
    // `git diff` shows no untracked file, which the commit gate would then record as reviewed (RVT-6)
    let mut args = vec!["ls-files", "-z", "--others", "--exclude-standard", "--"];
    args.extend(pathspecs.iter().map(String::as_str));
    let untracked = git(root, &args)?;
    if !untracked.is_empty() {
        let paths: Vec<&str> = untracked.split_terminator('\0').collect();
        return Err(format!(
            "untracked files git diff would leave out of the review; mark them intent-to-add, then run it again:\n  git add -N {}",
            paths.iter().map(|p| quoted(p)).collect::<Vec<_>>().join(" ")
        ));
    }
    let words = std::iter::once(quoted(base))
        .chain(std::iter::once("--".to_string()))
        .chain(pathspecs.iter().map(|p| quoted(p)));
    Ok(words.collect::<Vec<_>>().join(" "))
}

/// The stdout of `git <args>` run in `root`, or why it failed.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `word` in single quotes, a `'` of it written `'\''` (RVT-5).
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}
