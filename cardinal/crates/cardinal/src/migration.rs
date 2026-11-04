//! The migrations a source declares in `migrations/<n>.toml` (SYN-52): the managed names the format `<n>` replaced and
//! the seeded texts it rewrites in the project files (SYN-53). A sync applies those above the format of the lock, up
//! to the one of the source.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{bail, Context, Result};

/// The directory of the source that holds the migrations.
const DIR: &str = "migrations";

/// A managed name replaced by a format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub old: String,
    pub new: String,
}

/// A text the source seeded in the project file `path`, and its replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rewrite {
    pub path: String,
    pub old: String,
    pub new: String,
}

/// The migration to the format `format`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Migration {
    pub format: u32,
    pub names: Vec<Name>,
    pub rewrites: Vec<Rewrite>,
}

/// The migrations of the source in `source` whose format is above `from` and up to `to`, in increasing format.
pub(crate) fn applying(source: &Path, from: u32, to: u32) -> Result<Vec<Migration>> {
    let mut migrations = Vec::new();
    for format in from.saturating_add(1)..=to {
        let path = source.join(DIR).join(format!("{format}.toml"));
        let text = match fs::read_to_string(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            other => other.with_context(|| format!("reading {}", path.display()))?,
        };
        let migration =
            parse(format, &text).with_context(|| format!("{}: not a migration", path.display()))?;
        migrations.push(migration);
    }
    Ok(migrations)
}

fn parse(format: u32, text: &str) -> Result<Migration> {
    let table: toml::Table = toml::from_str(text)?;
    let mut migration = Migration {
        format,
        ..Migration::default()
    };
    for (key, value) in &table {
        match key.as_str() {
            "names" => {
                for item in tables(key, value)? {
                    let [old, new] = strings(item, ["old", "new"])?;
                    migration.names.push(Name { old, new });
                }
            }
            "rewrite" => {
                for item in tables(key, value)? {
                    let [path, old, new] = strings(item, ["path", "old", "new"])?;
                    migration.rewrites.push(Rewrite { path, old, new });
                }
            }
            _ => bail!("unknown key `{key}`"),
        }
    }
    Ok(migration)
}

/// The tables of the list `key`.
fn tables<'a>(key: &str, value: &'a toml::Value) -> Result<Vec<&'a toml::Table>> {
    let Some(items) = value.as_array() else {
        bail!("`{key}` is not a list of tables");
    };
    items
        .iter()
        .map(|item| {
            item.as_table()
                .with_context(|| format!("`{key}` is not a list of tables"))
        })
        .collect()
}

/// The values of `keys` in `table`, each a non-empty string, and no other key.
fn strings<const N: usize>(table: &toml::Table, keys: [&str; N]) -> Result<[String; N]> {
    if let Some(key) = table.keys().find(|k| !keys.contains(&k.as_str())) {
        bail!("unknown key `{key}`");
    }
    let mut values = keys.map(|_| String::new());
    for (value, key) in values.iter_mut().zip(keys) {
        match table.get(key).and_then(toml::Value::as_str) {
            Some(text) if !text.is_empty() => *value = text.to_string(),
            _ => bail!("`{key}` is not a non-empty string"),
        }
    }
    Ok(values)
}

/// Each old name of `migrations` with the format and the name of the last migration that renamed it, following a name
/// a later migration renamed again (SYN-54).
pub(crate) fn renamed(migrations: &[Migration]) -> Vec<(u32, Name)> {
    let mut names = Vec::new();
    for (at, migration) in migrations.iter().enumerate() {
        for name in &migration.names {
            let (mut format, mut new) = (migration.format, name.new.clone());
            for later in &migrations[at + 1..] {
                if let Some(next) = later.names.iter().find(|n| n.old == new) {
                    (format, new) = (later.format, next.new.clone());
                }
            }
            names.push((
                format,
                Name {
                    old: name.old.clone(),
                    new,
                },
            ));
        }
    }
    names
}

/// Whether a letter, a digit or `_` borders `line[start..end]`: the match is part of a longer word.
fn in_a_word(line: &str, start: usize, end: usize) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line[..start].chars().next_back().is_some_and(word)
        || line[end..].chars().next().is_some_and(word)
}

/// Whether `line` holds `name.old` as a word, outside an occurrence of `name.new`, which may contain it.
pub(crate) fn cites(line: &str, name: &Name) -> bool {
    let news: Vec<usize> = line.match_indices(&name.new).map(|(i, _)| i).collect();
    line.match_indices(&name.old).any(|(i, _)| {
        let end = i + name.old.len();
        !in_a_word(line, i, end) && !news.iter().any(|&j| j <= i && end <= j + name.new.len())
    })
}

/// `text` with every occurrence of `old` that spans whole lines replaced by `new`, `None` when there is none.
pub(crate) fn rewritten(text: &str, old: &str, new: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = 0;
    let mut changed = false;
    let mut from = 0;
    while let Some(found) = text[from..].find(old) {
        let start = from + found;
        let end = start + old.len();
        let starts_a_line = start == 0 || text.as_bytes()[start - 1] == b'\n';
        let ends_a_line = old.ends_with('\n')
            || end == text.len()
            || text[end..].starts_with('\n')
            || text[end..].starts_with("\r\n");
        if starts_a_line && ends_a_line {
            out.push_str(&text[rest..start]);
            out.push_str(new);
            rest = end;
            from = end;
            changed = true;
        } else {
            from = start + text[start..].chars().next().map_or(1, char::len_utf8);
        }
    }
    changed.then(|| {
        out.push_str(&text[rest..]);
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewritten_replaces_whole_lines_only() {
        assert_eq!(
            rewritten("a\nab\nba\na", "a", "x").as_deref(),
            Some("x\nab\nba\nx")
        );
        assert_eq!(rewritten("ab\n", "a", "x"), None);
        assert_eq!(
            rewritten("a\nb\nc", "a\nb\n", "y\n").as_deref(),
            Some("y\nc")
        );
    }

    // spec: SYN-54 (internal: the matching of a name contained in its replacement, cheaper here than a git project)
    #[test]
    fn a_line_that_only_cites_the_new_name_containing_the_old_one_does_not_cite_it() {
        let name = Name {
            old: "check".into(),
            new: "check-new".into(),
        };
        assert!(!cites("run check-new", &name));
        assert!(cites("run check-new and check", &name));
        assert!(cites("check", &name));
    }
}
