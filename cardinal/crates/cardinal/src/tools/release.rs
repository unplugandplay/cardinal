//! `release [<version> [--tag]]`: a numbered release of the project from its Conventional Commits, or from its
//! `release_breaking_files` only for the commits written while it turned off `conventional-commits`
//! (`specs/release.md`). With no version, print the version proposed from the commits since the last `vX.Y.Z` tag.
//! With one, write its section of `CHANGELOG.md` and run the project's `release_version_command`, for the user to
//! reread and commit; with `--tag` too, once that commit is made, create the annotated tag `v<version>` on `HEAD`.
//! Never commits nor pushes: it prints the push command.

use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::roadmap;
use super::transcripts::commit_config;
use crate::features;

/// The header of a new `CHANGELOG.md`, the text of the seeded one.
pub(crate) const HEADER: &str = "# Changelog

The notable changes of each version, newest first ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
[Semantic Versioning](https://semver.org/)). Each section is written by `just release <version>` from the commits,
then reread before the release is committed.
";
const CHANGELOG: &str = "CHANGELOG.md";
/// The section of a release with no version tag before it.
const FIRST: &str = "First numbered release.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u64, u64, u64);

impl Version {
    /// `X.Y.Z`, numbers without a leading zero, an optional leading `v`.
    fn parse(text: &str) -> Option<Version> {
        let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
        let mut next = || {
            parts
                .next()
                .filter(|p| {
                    !p.is_empty()
                        && p.bytes().all(|b| b.is_ascii_digit())
                        && (p.len() == 1 || !p.starts_with('0'))
                })?
                .parse()
                .ok()
        };
        let version = Version(next()?, next()?, next()?);
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Where a commit goes in the changelog (REL-2, REL-4).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Breaking,
    Added,
    Fixed,
    /// A commit not breaking written while its project turned off `conventional-commits` (REL-12).
    Changed,
    Unlisted,
}

struct Commit {
    kind: Kind,
    description: String,
    /// Read with Conventional Commits, the feature `conventional-commits` on in its revision (REL-11).
    convention: bool,
}

pub fn main(args: &[String]) -> u8 {
    match run(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("release: {e}");
            1
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let root = super::repo_root()?;
    let root = Path::new(&root);
    let tag = args.iter().any(|a| a == "--tag");
    let versions: Vec<&String> = args.iter().filter(|a| *a != "--tag").collect();
    let version = match versions[..] {
        [] if !tag => {
            println!("{}", proposed(root)?.0);
            return Ok(());
        }
        [v] => Version::parse(v).ok_or_else(|| format!("`{v}` is not a version X.Y.Z"))?,
        _ => return Err("usage: release [<version> [--tag]]".into()),
    };
    if git(
        root,
        &[
            "rev-parse",
            "-q",
            "--verify",
            &format!("refs/tags/v{version}"),
        ],
    )
    .is_ok()
    {
        return Err(format!("the tag v{version} exists"));
    }
    if tag {
        tag_release(root, version)
    } else {
        prepare(root, version)
    }
}

/// Write the section of `version`, mark the milestones it delivers and run the version command (REL-3 to REL-6,
/// REL-9); on a failure, the files written are put back.
fn prepare(root: &Path, version: Version) -> Result<(), String> {
    let (proposal, last, commits) = proposed(root)?;
    let path = root.join(CHANGELOG);
    let before = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let text = before.clone().unwrap_or_else(|| HEADER.to_string());
    if section(&text, version).is_some() {
        return Err(format!("{CHANGELOG} already has a section for {version}"));
    }
    let mut roadmaps = Vec::new();
    let mut milestones = String::new();
    for rel in super::config_list(root, "roadmap").unwrap_or_default() {
        let path = root.join(rel);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let (marked, names) = mark_released(&text, version);
        if !names.is_empty() {
            for name in names {
                let _ = writeln!(milestones, "- {name}");
            }
            roadmaps.push((path, Some(text), marked));
        }
    }
    let today = chrono::Local::now().format("%Y-%m-%d");
    let mut new = format!("## [{version}] - {today}\n");
    if !milestones.is_empty() {
        let _ = write!(new, "\n### Milestones\n\n{milestones}");
    }
    if let Some(last) = last {
        if commits.is_empty() {
            return Err(format!("no commit since v{last}: nothing to release"));
        }
        if version < proposal {
            return Err(format!(
                "{version} is lower than {proposal}, the version the commits since v{last} call for"
            ));
        }
        for (kind, title) in [
            (Kind::Breaking, "Breaking"),
            (Kind::Added, "Added"),
            (Kind::Fixed, "Fixed"),
            (Kind::Changed, "Changed"),
        ] {
            let mut lines = String::new();
            for c in commits.iter().filter(|c| c.kind == kind) {
                let _ = writeln!(lines, "- {}", c.description);
            }
            if !lines.is_empty() {
                let _ = write!(new, "\n### {title}\n\n{lines}");
            }
        }
    } else {
        let _ = write!(new, "\n{FIRST}\n");
    }
    let mut writes = vec![(path, before, insert(&text, &new))];
    writes.extend(roadmaps);
    let written =
        write_all(&writes).and_then(|()| match project_str(root, "release_version_command") {
            Some(command) => Command::new("sh")
                .arg("-c")
                .arg(format!("{command} {version}"))
                .current_dir(root)
                .status()
                .is_ok_and(|s| s.success())
                .then_some(())
                .ok_or_else(|| format!("the release_version_command `{command} {version}` failed")),
            None => Ok(()),
        });
    if let Err(e) = written {
        // Leave the files as they were, so the same release can be prepared again.
        for (path, before, _) in &writes {
            let _ = match before {
                Some(before) => std::fs::write(path, before),
                None => std::fs::remove_file(path),
            };
        }
        return Err(e);
    }
    println!(
        "{CHANGELOG}: the section of {version} is written (proposed: {proposal}).\n\
         Next: reread and edit it, commit the release, then `cardinal run release {version} --tag`."
    );
    Ok(())
}

/// Write each file its new text; `(path, text before, new text)`.
fn write_all(writes: &[(PathBuf, Option<String>, String)]) -> Result<(), String> {
    for (path, _, text) in writes {
        std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Tag `HEAD` with the committed section of `version` (REL-7, REL-10).
fn tag_release(root: &Path, version: Version) -> Result<(), String> {
    if !git(root, &["status", "--porcelain"])?.is_empty() {
        return Err(
            "the work tree has uncommitted changes or untracked files: commit the release first"
                .into(),
        );
    }
    let committed = git(root, &["show", &format!("HEAD:{CHANGELOG}")]).unwrap_or_default();
    let message = section(&committed, version)
        .ok_or_else(|| format!("the {CHANGELOG} of HEAD has no section for {version}"))?;
    let name = format!("v{version}");
    // `verbatim`: the default cleanup would drop the `#` headings of the section.
    git(
        root,
        &["tag", "-a", &name, "--cleanup=verbatim", "-m", &message],
    )?;
    println!("Tagged {name}. To publish it: git push origin {name}");
    Ok(())
}

/// The proposed version, the last version tag reachable from `HEAD`, and the commits since it, none with no tag
/// (REL-1, REL-11).
fn proposed(root: &Path) -> Result<(Version, Option<Version>, Vec<Commit>), String> {
    let last = git(root, &["tag", "--merged", "HEAD", "--list", "v*"])?
        .lines()
        .filter_map(Version::parse)
        .max();
    let breaking_files = super::config_list(root, "release_breaking_files").unwrap_or_default();
    let commits = match last {
        Some(last) => commits(root, last, &breaking_files)?,
        None => Vec::new(),
    };
    if commits.iter().any(|c| !c.convention) {
        eprintln!(
            "release: {} is off: the version is proposed from release_breaking_files only",
            features::CONVENTIONAL_COMMITS
        );
        if breaking_files.is_empty() {
            eprintln!(
                "release: release_breaking_files lists no file: the commits written with {} off are a patch",
                features::CONVENTIONAL_COMMITS
            );
        }
    }
    let Some(last @ Version(major, minor, patch)) = last else {
        return Ok((Version(0, 1, 0), None, commits));
    };
    let proposal = if commits.iter().any(|c| c.kind == Kind::Breaking) {
        Version(major + 1, 0, 0)
    } else if commits.iter().any(|c| c.kind == Kind::Added) {
        Version(major, minor + 1, 0)
    } else {
        Version(major, minor, patch + 1)
    };
    Ok((proposal, Some(last), commits))
}

/// The non-merge commits since the tag of `last`, oldest first, each read with the convention of its own revision
/// (REL-11, REL-12).
fn commits(root: &Path, last: Version, breaking_files: &[String]) -> Result<Vec<Commit>, String> {
    let log = git(
        root,
        &[
            "log",
            "--reverse",
            "--no-merges",
            "--name-only",
            "--format=%x1e%H%x1f%B%x1f",
            &format!("v{last}..HEAD"),
        ],
    )?;
    Ok(log
        .split('\u{1e}')
        .filter(|r| !r.trim().is_empty())
        .map(|record| {
            let mut fields = record.splitn(3, '\u{1f}');
            let (hash, message, files) = (
                fields.next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
                fields.next().unwrap_or_default(),
            );
            // as `rule-breaches` reads it (FEA-22): a revision with no readable configuration follows the convention
            let convention = !commit_config(root, hash)
                .is_some_and(|c| features::turns_off(&c, features::CONVENTIONAL_COMMITS));
            let touches_listed = files
                .lines()
                .any(|f| breaking_files.iter().any(|b| b == f.trim()));
            classify(message, touches_listed, convention)
        })
        .collect())
}

/// The kind and description of a commit message; `touches_listed` when it changes a `release_breaking_files` file;
/// without the `convention`, only that file counts and the description is the whole subject (REL-11, REL-12).
fn classify(message: &str, touches_listed: bool, convention: bool) -> Commit {
    let subject = message.lines().next().unwrap_or("").trim();
    if !convention {
        return Commit {
            kind: if touches_listed {
                Kind::Breaking
            } else {
                Kind::Changed
            },
            description: subject.into(),
            convention,
        };
    }
    let (head, description) = subject.split_once(": ").unwrap_or(("", subject));
    let footer = message
        .lines()
        .any(|l| l.starts_with("BREAKING CHANGE:") || l.starts_with("BREAKING-CHANGE:"));
    let kind = if head.ends_with('!') || footer || touches_listed {
        Kind::Breaking
    } else {
        match head.split(['(', '!']).next().unwrap_or("") {
            "feat" => Kind::Added,
            "fix" => Kind::Fixed,
            _ => Kind::Unlisted,
        }
    };
    Commit {
        kind,
        description: description.trim().into(),
        convention,
    }
}

/// A roadmap with the line `- Released: v<version>` added after the `State:` line of each milestone (`roadmap`)
/// that is `done` and has no `Released:` line, and the names of those milestones.
fn mark_released(text: &str, version: Version) -> (String, Vec<String>) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let marked: Vec<(usize, String)> = roadmap::milestones(&lines)
        .into_iter()
        .filter(|m| {
            !m.body
                .iter()
                .any(|&i| roadmap::is_field(lines[i], "Released:"))
        })
        .filter_map(|m| Some((m.state_line(&lines, "done")?, m.name)))
        .collect();
    let mut out = String::with_capacity(text.len());
    for (i, l) in lines.iter().enumerate() {
        out.push_str(l);
        if marked.iter().any(|(at, _)| *at == i) {
            if !l.ends_with('\n') {
                out.push('\n');
            }
            let _ = writeln!(out, "- Released: v{version}");
        }
    }
    (out, marked.into_iter().map(|(_, name)| name).collect())
}

/// The section of `version` in a changelog, from its heading to the next `## ` heading, trimmed.
fn section(text: &str, version: Version) -> Option<String> {
    let heading = format!("## [{version}]");
    let start = text.lines().position(|l| l.starts_with(&heading))?;
    let lines: Vec<&str> = text
        .lines()
        .skip(start)
        .enumerate()
        .take_while(|(i, l)| *i == 0 || !l.starts_with("## "))
        .map(|(_, l)| l)
        .collect();
    Some(lines.join("\n").trim().to_string())
}

/// `text` with `new` above its first `## ` section, or at its end when it has none.
fn insert(text: &str, new: &str) -> String {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("## ") && !line.starts_with("## [Unreleased]") {
            return format!("{}{new}\n{}", &text[..offset], &text[offset..]);
        }
        offset += line.len();
    }
    let sep = if text.ends_with('\n') { "\n" } else { "\n\n" };
    format!("{text}{sep}{new}")
}

/// The value of `key` in the `[project]` table of `.claude/cardinal.toml`, if any.
fn project_value(root: &Path, key: &str) -> Option<toml::Value> {
    let text = std::fs::read_to_string(crate::config_of(root)).ok()?;
    let table: toml::Table = text.parse().ok()?;
    table.get("project")?.get(key).cloned()
}

fn project_str(root: &Path, key: &str) -> Option<String> {
    project_value(root, key)?
        .as_str()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(String::from)
}

/// The output of `git <args>` in `root`, or its error.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seeded_changelog_is_the_header() {
        let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../seed/CHANGELOG.md");
        assert_eq!(std::fs::read_to_string(seed).unwrap(), HEADER);
    }
}
