//! The features a project turns off (`specs/features.md`, ADR 0025): the source groups managed files and settings
//! entries into named features in `features.toml`, the project lists the ones it turns off in the `disabled` list of
//! the `[cardinal]` table of its configuration, and `sync` leaves them out as if the source did not manage them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::entries::Entry;
use crate::{config_of, CONFIG, TABLE};

/// The features of a source, relative to its root.
pub const SOURCE: &str = "features.toml";
/// The key of the `[cardinal]` table that lists the features the project turns off.
const DISABLED: &str = "disabled";
/// The feature whose checks refuse an AI or tool attribution in commits and issues.
pub const NO_AI_ATTRIBUTION: &str = "no-ai-attribution";
/// The feature whose checks hold the commit messages to Conventional Commits.
pub const CONVENTIONAL_COMMITS: &str = "conventional-commits";
/// The feature whose hooks and native git hook refuse a commit its code review did not see.
pub const REVIEW_GATE: &str = "review-gate";
/// The feature whose skills and recipes keep the issues as files under `issues/`.
pub const ISSUE_FILES: &str = "issue-files";
/// The feature whose skills measure the harness from the transcripts.
pub const HARNESS_MEASURE: &str = "harness-measure";

/// What a feature is made of: paths under `files/`, and entries written as `sync` prints them (SYN-34); and the
/// features it needs (FEA-17).
#[derive(Debug, Default)]
struct Feature {
    files: Vec<String>,
    entries: Vec<String>,
    needs: Vec<String>,
}

/// What `sync` leaves out: the managed files and the managed entries of the features the project turns off.
#[derive(Debug, Default)]
pub struct Left {
    pub files: BTreeSet<String>,
    pub entries: Vec<Entry>,
}

/// The features `source` declares, by name; none when it has no `features.toml` (FEA-1). An error when the file is not
/// in the shape of FEA-1 and FEA-17 (FEA-7).
fn of_source(source: &Path) -> Result<BTreeMap<String, Feature>> {
    let path = source.join(SOURCE);
    let text = match fs::read_to_string(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        other => other.with_context(|| format!("reading {}", path.display()))?,
    };
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let mut features = BTreeMap::new();
    for (name, value) in table {
        let Some(feature) = value.as_table() else {
            bail!("{}: {name} is not a table", path.display());
        };
        let list = |key: &str| -> Result<Vec<String>> {
            match feature.get(key) {
                None => Ok(Vec::new()),
                Some(value) => {
                    strings(value).with_context(|| format!("{}: {name}.{key}", path.display()))
                }
            }
        };
        let (files, entries, needs) = (list("files")?, list("entries")?, list("needs")?);
        if let Some(key) = feature
            .keys()
            .find(|k| !matches!(k.as_str(), "files" | "entries" | "needs"))
        {
            bail!(
                "{}: {name}.{key} is not `files`, `entries` nor `needs`",
                path.display()
            );
        }
        features.insert(
            name,
            Feature {
                files,
                entries,
                needs,
            },
        );
    }
    Ok(features)
}

/// The strings of a TOML list, or an error when it is not a list of strings.
fn strings(value: &toml::Value) -> Result<Vec<String>> {
    let Some(list) = value.as_array() else {
        bail!("not a list of strings");
    };
    list.iter()
        .map(|v| v.as_str().map(str::to_string))
        .collect::<Option<_>>()
        .context("not a list of strings")
}

/// The features the project at `root` turns off, in the order of its configuration; none without the key (FEA-2). An
/// error when the configuration cannot be read or the key is not a list of strings (FEA-7).
pub fn disabled(root: &Path) -> Result<Vec<String>> {
    let path = config_of(root);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: toml::Table = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    match config.get(TABLE).and_then(|t| t.get(DISABLED)) {
        None => Ok(Vec::new()),
        Some(value) => strings(value).with_context(|| format!("{CONFIG}: [{TABLE}] {DISABLED}")),
    }
}

/// Whether the configuration `config` turns off the feature `name`.
pub fn turns_off(config: &toml::Table, name: &str) -> bool {
    config
        .get(TABLE)
        .and_then(|t| t.get(DISABLED))
        .and_then(toml::Value::as_array)
        .is_some_and(|names| names.iter().any(|n| n.as_str() == Some(name)))
}

/// Whether the project at `root` turns off the feature `name`; false when its configuration cannot be read, so a check
/// stays on rather than silently off.
#[must_use]
pub fn is_off(root: &Path, name: &str) -> bool {
    fs::read_to_string(config_of(root))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .is_some_and(|config| turns_off(&config, name))
}

/// What `sync` leaves out of `files` and `entries`, the managed content of `source`, for the project at `root`. An
/// error when the project names a feature the source does not declare (FEA-5), or a feature names a file or an entry
/// the source does not manage (FEA-6), needs a feature the source does not declare (FEA-17), or when the project turns
/// off a feature that one it leaves on needs (FEA-18).
pub fn left_out(root: &Path, source: &Path, files: &[String], entries: &[Entry]) -> Result<Left> {
    let features = of_source(source)?;
    for (name, feature) in &features {
        if let Some(file) = feature.files.iter().find(|f| !files.contains(f)) {
            bail!("{SOURCE}: the feature {name} names the file {file}, absent from files/");
        }
        if let Some(entry) = feature
            .entries
            .iter()
            .find(|e| !entries.iter().any(|m| m.to_string() == **e))
        {
            bail!(
                "{SOURCE}: the feature {name} names the entry {entry}, absent from {}",
                crate::entries::FRAGMENT
            );
        }
        if let Some(needed) = feature.needs.iter().find(|n| !features.contains_key(*n)) {
            bail!("{SOURCE}: the feature {name} needs the feature {needed}, which the source does not declare");
        }
    }
    let disabled = disabled(root)?;
    let mut left = Left::default();
    for name in &disabled {
        let Some(feature) = features.get(name) else {
            let names: Vec<&str> = features.keys().map(String::as_str).collect();
            bail!(
                "{CONFIG}: {DISABLED} names the unknown feature {name}; the source declares {}",
                if names.is_empty() {
                    "none".to_string()
                } else {
                    names.join(", ")
                }
            );
        };
        left.files.extend(feature.files.iter().cloned());
        left.entries.extend(
            entries
                .iter()
                .filter(|e| feature.entries.contains(&e.to_string()))
                .cloned(),
        );
    }
    // Only the features left on are judged, so a cycle in `needs` never blocks turning all of it off.
    for (name, feature) in features.iter().filter(|(n, _)| !disabled.contains(n)) {
        if let Some(needed) = feature.needs.iter().find(|n| disabled.contains(n)) {
            bail!("{CONFIG}: {DISABLED} turns off {needed}, which {name} needs");
        }
    }
    Ok(left)
}
