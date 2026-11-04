//! Copy the managed files of a Cardinal source into a project, and detect drift.
//!
//! A source holds `files/` (managed: overwritten on every sync while untouched locally) and `seed/`
//! (project-owned: copied once, never again), and may hold `entries/.claude/settings.json`, the entries of the
//! project's settings it manages (`entries.rs`). `.claude/cardinal.lock` records the hash of each managed file as
//! written and the managed entries, so a later sync tells a local edit from a source change.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::{self, Write as _};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

/// Project configuration, relative to the project root.
pub const CONFIG: &str = ".claude/cardinal.toml";
/// The personal configuration of the person at the keyboard, relative to the project root: git ignores it and Cardinal
/// never manages it; its `[project]` values override those of `CONFIG` for the personal keys (ADR 0018).
pub const PERSONAL_CONFIG: &str = ".claude/cardinal.local.toml";
/// Hashes of the managed files as last written, relative to the project root.
pub const LOCK: &str = ".claude/cardinal.lock";
/// The table of the configuration that names the source and its pinned revision.
pub(crate) const TABLE: &str = "cardinal";
/// The lowest format this `cardinal` syncs: the first whose issues are ordered by the rank of each file alone (SYN-51,
/// ADR 0020).
const MIN_FORMAT: u32 = 5;
/// The exit code of a failure that is not the verdict of a hook: `run` that cannot get the tool or does not know it,
/// the launcher that cannot parse its arguments, an error of the other subcommands. Not 2, which Claude Code reads as
/// the refusal of a hook (BIN-7, BIN-14, BIN-17, SYN-37).
pub const NOT_A_REFUSAL: u8 = 3;
/// The format of the managed content this `cardinal` reads, and every lower one down to `MIN_FORMAT` (SYN-40): equal
/// to the `cardinal-format` of its revision (SYN-43).
pub const FORMAT: u32 = 8;
/// The file of the source that declares the format its content needs, relative to the source.
const SOURCE_FORMAT: &str = "cardinal-format";

mod budget;
pub use budget::warnings as budget_warnings;
mod entries;
mod features;
mod gate;
mod launcher;
mod memory;
mod migration;
pub mod run;
mod settings;
pub mod tools;

/// The configuration of the project at `root`.
#[must_use]
pub fn config_of(root: &Path) -> PathBuf {
    root.join(CONFIG)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SyncOptions {
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Added,
    Updated,
    Removed,
    Seeded,
    Unchanged,
    /// Managed file modified locally: left as it is.
    Conflict,
    /// File dropped from the source but modified locally: left as it is, no longer managed.
    Kept,
    /// A hook of the project's own moved aside so that the managed one can chain it.
    Moved,
    /// A project file whose seeded text a migration rewrote (SYN-53).
    Rewritten,
    /// Not done, for the reason given.
    Skipped(&'static str),
}

impl Outcome {
    /// Whether the project file was written: a conflict, a kept file and a skip leave it as it was.
    fn wrote(&self) -> bool {
        matches!(
            self,
            Outcome::Added
                | Outcome::Updated
                | Outcome::Removed
                | Outcome::Seeded
                | Outcome::Moved
                | Outcome::Rewritten
        )
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Outcome::Added => "added",
            Outcome::Updated => "updated",
            Outcome::Removed => "removed",
            Outcome::Seeded => "seeded",
            Outcome::Unchanged => "unchanged",
            Outcome::Conflict => "conflict (modified locally, fix it in the source or use --force)",
            Outcome::Kept => "kept (dropped from the source but modified locally)",
            Outcome::Moved => "moved (your hook now runs after the review gate)",
            Outcome::Rewritten => "rewritten",
            Outcome::Skipped(why) => return write!(f, "skipped ({why})"),
        })
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub entries: Vec<(String, Outcome)>,
    /// What the project's settings lack or run in vain once synced (SYN-27): printed, never a failure.
    pub warnings: Vec<String>,
    /// The format the lock records once synced (as it is under `--dry-run`), `None` without a lock.
    lock_format: Option<u32>,
}

impl Report {
    #[must_use]
    pub fn conflicts(&self) -> usize {
        self.entries
            .iter()
            .filter(|(_, o)| *o == Outcome::Conflict)
            .count()
    }

    #[must_use]
    pub fn outcome(&self, path: &str) -> Option<Outcome> {
        self.entries
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, o)| *o)
    }
}

/// Hash of `bytes`, followed by `+x` for an executable file: a mode change is a change.
fn fingerprint(bytes: &[u8], executable: bool) -> String {
    let mut print = hash(bytes);
    if executable {
        print.push_str("+x");
    }
    print
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

/// Fingerprint of the file at `path`, `None` when it cannot be read.
fn fingerprint_of(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(fingerprint(&bytes, is_executable(path)))
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Relative paths (with `/`) of every file under `dir`, sorted. A missing `dir` is empty.
fn list(dir: &Path) -> Result<Vec<String>> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// A path of the lock must stay inside the project: relative, no `..`.
fn is_inside(rel: &str) -> bool {
    Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}

/// What the lock records: the fingerprint of each managed file, the managed entries of the settings, and the format of
/// the source it was synced from (1 when it records none, SYN-41).
#[derive(Debug, Default)]
struct Lock {
    files: BTreeMap<String, String>,
    entries: Vec<entries::Entry>,
    format: u32,
}

/// The mark of a lock line that holds a managed entry of the settings instead of a file.
const ENTRY: &str = "entry";
/// The mark of the lock line that holds the format of the source it was synced from, followed by one space: a
/// `cardinal` older than SYN-41 skips a line with no two spaces.
const FORMAT_LINE: &str = "format ";

/// The lock of the project, `None` when it has none yet; an error when its source has a format this `cardinal` does
/// not read, judged before any other line (SYN-42).
fn read_lock(root: &Path) -> Result<Option<Lock>> {
    let path = root.join(LOCK);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let lines = || text.lines().filter(|l| !l.starts_with('#'));
    let mut lock = Lock {
        format: 1,
        ..Lock::default()
    };
    for format in lines().filter_map(|line| line.strip_prefix(FORMAT_LINE)) {
        let format: u32 = format
            .parse()
            .with_context(|| format!("{}: not a format: {format}", path.display()))?;
        if format > FORMAT {
            bail!("{LOCK} was synced from a source of the format {format}, this one reads up to {FORMAT}: install it with {}", install_hint(root));
        }
        lock.format = format;
    }
    for line in lines().filter(|l| !l.starts_with(FORMAT_LINE)) {
        let Some((h, rel)) = line.split_once("  ") else {
            continue;
        };
        if h == ENTRY {
            let entry = entries::Entry::from_lock(rel)
                .with_context(|| format!("reading {}", path.display()))?;
            lock.entries.push(entry);
            continue;
        }
        if !is_inside(rel) {
            bail!("{}: path outside the project: {rel}", path.display());
        }
        lock.files.insert(rel.to_string(), h.to_string());
    }
    Ok(Some(lock))
}

/// The managed files the lock of the project at `root` lists, sorted by path; none when it has no lock.
pub(crate) fn managed_files(root: &Path) -> Result<Vec<String>> {
    Ok(read_lock(root)?
        .map(|lock| lock.files.into_keys().collect())
        .unwrap_or_default())
}

/// The command that installs the `cardinal` of the revision the configuration pins, quoted, or what to install when it
/// cannot be read.
fn install_hint(root: &Path) -> String {
    match source_and_rev(root) {
        Ok((source, rev)) => format!("`{}`", launcher::install(&source, rev.as_deref())),
        Err(_) => "the cardinal of this revision".to_string(),
    }
}

/// Write the lock of a sync from a source of the format `format`.
fn write_lock(root: &Path, lock: &Lock, format: u32) -> Result<()> {
    let mut text = String::from(
        "# Written by `cardinal sync`: hashes of the managed files, then the managed settings entries. Do not edit.\n",
    );
    let _ = writeln!(text, "{FORMAT_LINE}{format}");
    for (path, h) in &lock.files {
        let _ = writeln!(text, "{h}  {path}");
    }
    for entry in &lock.entries {
        let _ = writeln!(text, "{ENTRY}  {}", entry.to_lock());
    }
    write(&root.join(LOCK), text.as_bytes())
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

/// `write`, then make the file executable or not as in the source.
fn write_as(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    write(path, bytes)?;
    set_executable(path, executable)
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    let mode = permissions.mode();
    // Only the execute bits change, and only where the read bits are: the usual `chmod +x` / `-x`.
    let mode = if executable {
        mode | ((mode & 0o444) >> 2)
    } else {
        mode & !0o111
    };
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions).with_context(|| format!("chmod {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

fn read_config(root: &Path) -> Result<toml::Table> {
    let path = config_of(root);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "no {CONFIG} in {}: run `cardinal init <source>`",
            root.display()
        )
    })?;
    text.parse()
        .with_context(|| format!("parsing {}", path.display()))
}

// A plain suffix test, not `Path::extension`: `/x/.git` is a repository and `X.GIT` or `x.git/` a
// directory (SYN-49).
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_git_url(source: &str) -> bool {
    source.contains("://") || source.starts_with("git@") || source.ends_with(".git")
}

/// A scp-like `[user@]host:path` git source: no `://`, and a `:` before its first `/`, as git reads it.
pub(crate) fn is_scp_like(source: &str) -> bool {
    !source.contains("://")
        && source
            .split('/')
            .next()
            .is_some_and(|head| head.contains(':'))
}

/// The source as git and cargo read it (SYN-50): a git source that is a path becomes the absolute `file://` URL of
/// `root.join(source)`, `root` with its symlinks resolved, so the clone, its cache key and `cargo install --git` do not
/// depend on the working directory nor on the path the root was reached by; a URL, a scp-like `host:path` and a local
/// directory are returned as written.
fn resolved(root: &Path, source: &str) -> String {
    if !is_git_url(source) || source.contains("://") || is_scp_like(source) {
        return source.to_string();
    }
    let Ok(root) = fs::canonicalize(root).or_else(|_| std::path::absolute(root)) else {
        return source.to_string();
    };
    let mut path = PathBuf::from("/");
    for part in root.join(source).components() {
        match part {
            Component::Normal(name) => path.push(name),
            Component::ParentDir => {
                path.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
        }
    }
    let mut url = String::from("file://");
    for &byte in path_bytes(&path).iter() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            url.push(char::from(byte));
        } else {
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}

/// The bytes of `path` as the system names it: its raw bytes on unix, its UTF-8 elsewhere.
#[cfg(unix)]
fn path_bytes(path: &Path) -> std::borrow::Cow<'_, [u8]> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().into()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> std::borrow::Cow<'_, [u8]> {
    path.to_string_lossy().into_owned().into_bytes().into()
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    let status = cmd.args(args).status().context("running git")?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn resolves(dir: &Path, rev: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn cache_dir() -> Result<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("no cache directory (set HOME or XDG_CACHE_HOME)")
}

fn cache_or_default(cache: Option<&Path>) -> Result<PathBuf> {
    cache.map_or_else(cache_dir, |c| Ok(c.to_path_buf()))
}

/// Where the clone of the git source `url` lives in `cache`.
fn clone_dir(cache: &Path, url: &str) -> PathBuf {
    cache.join("cardinal").join(&hash(url.as_bytes())[..12])
}

/// Clone (first time) or fetch the git source in `cache`, and return its local directory.
fn fetch(cache: &Path, url: &str) -> Result<PathBuf> {
    let dir = clone_dir(cache, url);
    if dir.join(".git").exists() {
        git(Some(&dir), &["fetch", "--quiet", "--tags", "origin"])?;
        // `fetch` leaves `origin/HEAD` alone: follow a default branch the remote renamed (best effort).
        let _ = git_output(&dir, &["remote", "set-head", "origin", "--auto"]);
    } else {
        fs::create_dir_all(&dir)?;
        git(None, &["clone", "--quiet", url, &dir.to_string_lossy()])?;
    }
    Ok(dir)
}

/// Full commit hash of `rev` (default: the remote's default branch) in a fetched source.
fn resolve_in(dir: &Path, url: &str, rev: Option<&str>) -> Result<String> {
    if rev.is_some_and(|r| r.starts_with('-')) {
        bail!("{url}: invalid revision {}", rev.unwrap_or_default());
    }
    // A branch name must resolve to the remote branch the fetch just moved, not to a stale local one.
    let candidates = match rev {
        Some(rev) => vec![format!("origin/{rev}"), rev.to_string()],
        None => vec!["origin/HEAD".to_string()],
    };
    if let Some(target) = candidates.iter().find(|c| resolves(dir, c)) {
        return git_output(dir, &["rev-parse", &format!("{target}^{{commit}}")]);
    }
    let rev = rev.unwrap_or("origin/HEAD");
    if !is_full_hash(rev) {
        bail!("{url}: unknown revision {rev}");
    }
    // No branch or tag points to the pinned commit any more: ask the source for it by hash (hosts differ
    // on whether they serve it; a refusal is reported below).
    let _ = git(Some(dir), &["fetch", "--quiet", "origin", rev]);
    if !resolves(dir, rev) {
        bail!("{url}: the pinned commit {rev} is gone from the source (or the source is unreachable: see git's message above); run `cardinal update` to pin a current revision");
    }
    git_output(dir, &["rev-parse", &format!("{rev}^{{commit}}")])
}

/// Replace the `cardinal` of the `PATH` (else the running one) with the binary of `commit` of the git source `source`, fetched as `run` fetches it
/// (BIN-19, BIN-21); `source` is resolved against `root` as the configuration would be. Returns the path replaced.
///
/// # Errors
/// When `commit` is not a full hash, `source` is not a git source, the binary cannot be fetched or the running one
/// cannot be replaced.
pub fn upgrade(root: &Path, source: &str, commit: &str) -> Result<PathBuf> {
    if !is_full_hash(commit) {
        bail!("`{commit}` is not a full commit hash: pass the `rev` that `cardinal update` writes");
    }
    if !is_git_url(source) {
        bail!(
            "`{source}` is not a git source: install the cardinal of a local source with `{}`",
            launcher::install(source, None)
        );
    }
    let binary = run::binary_of(
        &cache_dir()?,
        &resolved(root, source),
        commit,
        &run::Programs::default(),
    )?;
    let exe = launcher_on_path().map_or_else(
        || std::env::current_exe().context("finding the running cardinal"),
        Ok,
    )?;
    run::replace(&binary, &exe)?;
    Ok(exe)
}

/// The first executable `cardinal` of an absolute directory of the `PATH`, as the shell finds it, its symlinks left as they are: the launcher the hooks run, which `upgrade`
/// replaces rather than a link target of the cache or the binary it was started from (BIN-19).
fn launcher_on_path() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("cardinal"))
        .find(|path| path.is_file() && is_executable(path))
}

/// A 40 (SHA-1) or 64 (SHA-256) hexadecimal digit commit name, as `update` writes it to `rev`.
fn is_full_hash(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Local checkout of a git source in the user cache, at `rev` (default: the remote's default branch).
fn checkout(cache: Option<&Path>, url: &str, rev: Option<&str>) -> Result<PathBuf> {
    let dir = fetch(&cache_or_default(cache)?, url)?;
    let commit = resolve_in(&dir, url, rev)?;
    git(Some(&dir), &["checkout", "--quiet", "--detach", &commit])?;
    Ok(dir)
}

/// The text of the configuration with `rev` set in its table, every other line left as it is.
///
/// The line becomes `rev = "<commit>"  # <name>`; a missing `rev` is inserted right after `source`.
fn set_rev(text: &str, commit: &str, name: &str) -> Result<String> {
    if name.contains(char::is_control) {
        bail!("invalid revision name {name:?}");
    }
    let line = if name == commit {
        format!("rev = \"{commit}\"")
    } else {
        format!("rev = \"{commit}\"  # {name}")
    };
    let key_is = |l: &str, key: &str| {
        l.trim_start()
            .strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    };
    let (config, table) = (CONFIG, TABLE);
    let header = format!("[{table}]");
    let mut in_table = false;
    let mut source_at = None;
    let mut rev_at = None;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    for (i, l) in lines.iter().enumerate() {
        let trimmed = l.trim();
        if is_table_header(trimmed) {
            in_table = trimmed.starts_with(&header);
        } else if in_table && key_is(l, "rev") {
            rev_at = Some(i);
        } else if in_table && key_is(l, "source") {
            source_at = Some(i);
        }
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    if let Some(i) = rev_at {
        let ending = &lines[i][lines[i].trim_end_matches(['\r', '\n']).len()..];
        out[i] = format!("{line}{ending}");
    } else if let Some(i) = source_at {
        if !out[i].ends_with('\n') {
            out[i].push_str(eol);
        }
        out.insert(i + 1, format!("{line}{eol}"));
    } else {
        bail!("{config}: missing `source` in the {header} table");
    }
    let out = out.concat();
    // Quoted or dotted keys are valid TOML this line-based edit does not see: refuse rather than corrupt.
    let written = out
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| t.get(table)?.get("rev")?.as_str().map(|r| r == commit));
    if written != Some(true) {
        bail!("{config}: cannot rewrite `rev` automatically, set `rev = \"{commit}\"` in {header} by hand");
    }
    Ok(out)
}

/// A `[table]` line, as opposed to a continuation line of a multi-line array such as `["a"],`.
fn is_table_header(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('[') else {
        return false;
    };
    let Some((name, after)) = rest.split_once(']') else {
        return false;
    };
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
        && (after.trim().is_empty() || after.trim_start().starts_with('#'))
}

/// The `source`, resolved (SYN-50), and the pinned `rev` of the `[cardinal]` table of the configuration.
fn source_and_rev(root: &Path) -> Result<(String, Option<String>)> {
    let config = read_config(root)?;
    let table = config.get(TABLE).and_then(toml::Value::as_table);
    let text = |key: &str| {
        table
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    let source = text("source")
        .with_context(|| format!("{CONFIG}: missing `source` in the [{TABLE}] table"))?;
    Ok((resolved(root, &source), text("rev")))
}

/// The format the source in `dir` declares, 1 when it declares none.
fn format_of(dir: &Path) -> Result<u32> {
    let path = dir.join(SOURCE_FORMAT);
    match fs::read_to_string(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(1),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        Ok(text) => text
            .trim()
            .parse()
            .with_context(|| format!("{}: not a whole number", path.display())),
    }
}

/// An error when the source `name` in `dir` needs a newer `cardinal`, which `install` installs (SYN-40), or an older
/// one (SYN-51); a directory without `files/` is no source, which the sync reports (SYN-10).
fn readable(dir: &Path, name: &str, install: &str) -> Result<()> {
    if !dir.join("files").is_dir() {
        return Ok(());
    }
    let format = format_of(dir)?;
    if format > FORMAT {
        bail!("{name} needs a cardinal that reads the format {format}, this one reads up to {FORMAT}: install it with `{install}`");
    }
    if format < MIN_FORMAT {
        bail!("{name} has the format {format}, this cardinal syncs the format {MIN_FORMAT} and higher");
    }
    Ok(())
}

/// Create the configuration from the source and the project-owned defaults of the source (SYN-9).
///
/// # Errors
/// When the project already has a configuration or the source cannot be read.
pub fn init(root: &Path, source: &str, rev: Option<&str>) -> Result<()> {
    let path = root.join(CONFIG);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    let (dir, install) = if is_git_url(source) {
        let url = resolved(root, source);
        let dir = checkout(None, &url, rev)?;
        let commit = git_output(&dir, &["rev-parse", "HEAD"])?;
        let install = launcher::install(&url, Some(&commit));
        (dir, install)
    } else {
        (root.join(source), launcher::install(source, None))
    };
    readable(&dir, source, &install)?;
    let mut table = toml::Table::new();
    table.insert("source".into(), source.into());
    if let Some(rev) = rev {
        table.insert("rev".into(), rev.into());
    }
    let mut config = toml::Table::new();
    config.insert(TABLE.into(), table.into());
    let mut text = config.to_string();
    if let Ok(defaults) = fs::read_to_string(dir.join("seed").join(CONFIG)) {
        text.push('\n');
        text.push_str(&defaults);
    }
    write(&path, text.as_bytes())
}

/// Bring the project in step with its source.
///
/// # Errors
/// When the configuration or the source cannot be read, or a file cannot be written.
pub fn sync(root: &Path, options: &SyncOptions) -> Result<Report> {
    let (source, rev) = source_and_rev(root)?;
    let (dir, commit) = if is_git_url(&source) {
        let dir = checkout(None, &source, rev.as_deref())?;
        let commit = git_output(&dir, &["rev-parse", "HEAD"])?;
        (dir, Some(commit))
    } else {
        (root.join(&source), None)
    };
    readable(
        &dir,
        &source,
        &launcher::install(&source, commit.as_deref()),
    )?;
    let mut report = sync_from(root, &dir, *options)?;
    if is_git_url(&source) {
        let hint = report.lock_format.and_then(format_hint);
        report.warnings.extend(hint);
    }
    if let (Some(commit), false) = (&commit, options.dry_run) {
        report.warnings.extend(prefetch(
            &cache_dir()?,
            &source,
            commit,
            &run::Programs::default(),
        ));
    }
    Ok(report)
}

/// The hint at `cardinal update` of a lock of a lower format than this `cardinal` reads (SYN-55); the caller gives it
/// to a git source only, a local directory has no revision to update.
fn format_hint(format: u32) -> Option<String> {
    (format < FORMAT).then(|| {
        format!(
            "hint: {LOCK} is of the format {format}, this cardinal reads the format {FORMAT}: run `cardinal update` to move the project to a newer revision"
        )
    })
}

/// Fetch the tools binary of `commit` into the cache, as `cardinal run` would: a warning when it fails (BIN-13).
fn prefetch(cache: &Path, source: &str, commit: &str, programs: &run::Programs) -> Option<String> {
    run::binary_of(cache, source, commit, programs)
        .err()
        .map(|e| format!("warning: {e:#} (`cardinal run` will try again)"))
}

fn sync_from(root: &Path, source: &Path, options: SyncOptions) -> Result<Report> {
    let managed = source.join("files");
    let seeds = source.join("seed");
    if !managed.is_dir() {
        bail!(
            "{} has no files/ directory: is `source` right in {}?",
            source.display(),
            CONFIG
        );
    }
    let previous = read_lock(root)?;
    let lock_format = previous.as_ref().map(|lock| lock.format);
    let format = format_of(source)?;
    // read before anything is written: a migration that cannot be read stops the sync (SYN-52)
    let migrations = match &previous {
        Some(lock) => migration::applying(source, lock.format, format)?,
        None => Vec::new(),
    };
    let Lock {
        files: lock,
        entries: recorded,
        ..
    } = previous.unwrap_or_default();
    let wanted = entries::of_source(source)?;
    let all_files = list(&managed)?;
    // the features the project turns off are content the source does not manage (FEA-3), judged before any write
    let (files, wanted) = turned_on(root, source, &all_files, wanted)?;
    // decided first: a hook of the user that cannot be chained stops the sync before it writes anything
    let gate = gate::plan(root, gate::manages(&wanted, files.iter()))?;
    // installed before the managed files are written, so a failure leaves the project as it was
    if let (Some(gate), false) = (&gate, options.dry_run) {
        if !gate.removes() {
            gate.apply()?;
        }
    }
    let mut report = Report::default();

    let file_set: BTreeSet<&String> = files.iter().collect();
    let new_lock = write_managed(root, &managed, &files, &lock, options, &mut report)?;
    remove_dropped(root, &lock, &file_set, options, &mut report)?;
    // a managed path is never a seed, even one of a feature turned off (FEA-15)
    let all_set: BTreeSet<&String> = all_files.iter().collect();
    write_seeds(root, &seeds, &all_set, options, &mut report)?;

    // after the seeds: a first sync merges into the settings it was just seeded with, written or not
    let seeded = report.outcome(entries::SETTINGS) == Some(Outcome::Seeded);
    let base = if seeded { &seeds } else { root };
    let text = fs::read_to_string(base.join(entries::SETTINGS)).ok();
    // settings seeded again hold nothing of the project's: no recorded entry is in conflict there
    let recorded = if seeded { Vec::new() } else { recorded };
    let merge = entries::merge(text.as_deref(), &wanted, &recorded, options.force);
    if let (Some(text), false) = (&merge.write, options.dry_run) {
        write(&root.join(entries::SETTINGS), text.as_bytes())?;
    }
    report.entries.extend(merge.outcomes);

    // removed only when the lock no longer records the gate: settings left alone keep its hooks (SYN-60)
    let gate = gate.filter(|gate| !gate.removes() || !gate::manages(&merge.lock, files.iter()));
    if let Some(gate) = &gate {
        report.entries.extend(gate.outcomes());
    }

    let new_lock = Lock {
        files: new_lock,
        entries: merge.lock,
        format,
    };
    // a project file as this run leaves it: a removed one is gone, the settings are merged
    let removed: BTreeSet<String> = report
        .entries
        .iter()
        .filter(|(_, outcome)| *outcome == Outcome::Removed)
        .map(|(rel, _)| rel.clone())
        .collect();
    let merged = merge.write.clone().filter(|_| options.dry_run);
    let as_left = |rel: &str| -> Option<String> {
        if removed.contains(rel) {
            return None;
        }
        match (&merged, rel == entries::SETTINGS) {
            (Some(text), true) => Some(text.clone()),
            _ => fs::read_to_string(root.join(rel)).ok(),
        }
    };
    let rewritten = rewrite_seeded(root, &migrations, &file_set, &as_left, options, &mut report)?;
    if !options.dry_run {
        write_lock(root, &new_lock, format)?;
        // removed last: a sync that fails before leaves the gate its lock still records, the next one removes it
        if let Some(gate) = gate.filter(gate::Plan::removes) {
            gate.apply()?;
        }
    }
    // judged on the project as the sync leaves it, or would under `--dry-run`
    let done = |path: &str, outcome| report.outcome(path) == Some(outcome);
    let present = |path: &str| {
        done(path, Outcome::Added) || (root.join(path).is_file() && !done(path, Outcome::Removed))
    };
    let gated = gate::manages(&new_lock.entries, new_lock.files.keys());
    let attribution = !features::is_off(root, features::NO_AI_ATTRIBUTION);
    report.warnings = match &merge.settings {
        Some(settings) => settings::warnings_of(settings, gated, &present, attribution),
        None => settings::warnings(base, gated, &present, attribution),
    };
    report.warnings.extend(merge.warning);
    report
        .warnings
        .extend(leftovers(root, &migrations, &new_lock, &|rel| {
            rewritten.get(rel).cloned().or_else(|| as_left(rel))
        }));
    report.lock_format = if options.dry_run {
        lock_format
    } else {
        Some(format)
    };
    Ok(report)
}

/// The managed files and entries of `source` the project at `root` keeps: those of no feature it turns off.
fn turned_on(
    root: &Path,
    source: &Path,
    files: &[String],
    wanted: Vec<entries::Entry>,
) -> Result<(Vec<String>, Vec<entries::Entry>)> {
    let left = features::left_out(root, source, files, &wanted)?;
    let files = files
        .iter()
        .filter(|f| !left.files.contains(*f))
        .cloned()
        .collect();
    let wanted = wanted
        .into_iter()
        .filter(|e| !left.entries.contains(e))
        .collect();
    Ok((files, wanted))
}

/// Write the managed files of `managed` the project lacks or left as locked (all of them with `force`); returns the new
/// file lock.
fn write_managed(
    root: &Path,
    managed: &Path,
    files: &[String],
    lock: &BTreeMap<String, String>,
    options: SyncOptions,
    report: &mut Report,
) -> Result<BTreeMap<String, String>> {
    let mut new_lock = BTreeMap::new();
    for rel in files {
        let bytes = fs::read(managed.join(rel))?;
        let executable = is_executable(&managed.join(rel));
        let want = fingerprint(&bytes, executable);
        let have = fingerprint_of(&root.join(rel));
        let locked = lock.get(rel);
        let outcome = match &have {
            None => Outcome::Added,
            Some(h) if *h == want => Outcome::Unchanged,
            Some(h) if Some(h) == locked || options.force => Outcome::Updated,
            Some(_) => Outcome::Conflict,
        };
        match outcome {
            Outcome::Added | Outcome::Updated => {
                if !options.dry_run {
                    write_as(&root.join(rel), &bytes, executable)?;
                }
                new_lock.insert(rel.clone(), want);
            }
            Outcome::Unchanged => {
                new_lock.insert(rel.clone(), want);
            }
            _ => {
                if let Some(old) = locked {
                    new_lock.insert(rel.clone(), old.clone());
                }
            }
        }
        report.entries.push((rel.clone(), outcome));
    }
    Ok(new_lock)
}

/// Remove the locked files the source dropped, unless the project changed them.
fn remove_dropped(
    root: &Path,
    lock: &BTreeMap<String, String>,
    file_set: &BTreeSet<&String>,
    options: SyncOptions,
    report: &mut Report,
) -> Result<()> {
    for (rel, locked) in lock.iter().filter(|(rel, _)| !file_set.contains(rel)) {
        let path = root.join(rel);
        match fingerprint_of(&path) {
            None => {}
            Some(h) if h == *locked => {
                if !options.dry_run {
                    fs::remove_file(&path)?;
                }
                report.entries.push((rel.clone(), Outcome::Removed));
            }
            Some(_) => report.entries.push((rel.clone(), Outcome::Kept)),
        }
    }
    Ok(())
}

/// Rewrite the seeded texts of `migrations` in the project files, the managed ones and the lock excepted (SYN-53);
/// returns the new text of each file rewritten, written or not.
fn rewrite_seeded(
    root: &Path,
    migrations: &[migration::Migration],
    file_set: &BTreeSet<&String>,
    as_left: &dyn Fn(&str) -> Option<String>,
    options: SyncOptions,
    report: &mut Report,
) -> Result<BTreeMap<String, String>> {
    let mut texts: BTreeMap<&str, String> = BTreeMap::new();
    let mut changed = Vec::new();
    for rewrite in migrations.iter().flat_map(|m| &m.rewrites) {
        let rel = rewrite.path.as_str();
        if !is_inside(rel) || rel == LOCK || file_set.contains(&rewrite.path) {
            continue;
        }
        if !texts.contains_key(rel) {
            let Some(text) = as_left(rel) else {
                continue;
            };
            texts.insert(rel, text);
        }
        let text = texts.get_mut(rel).expect("read above");
        if let Some(new) = migration::rewritten(text, &rewrite.old, &rewrite.new) {
            *text = new;
            if !changed.contains(&rel) {
                changed.push(rel);
            }
        }
    }
    let mut done = BTreeMap::new();
    for rel in changed {
        if !options.dry_run {
            write(&root.join(rel), texts[rel].as_bytes())?;
        }
        report.entries.push((rel.to_string(), Outcome::Rewritten));
        done.insert(rel.to_string(), texts[rel].clone());
    }
    Ok(done)
}

/// The directories of the project that record its history: an old name stays there (SYN-54).
const HISTORY: [&str; 2] = ["issues/done/", "docs/adr/"];

/// A warning for each line of a project-owned file git tracks that still cites an old name of `migrations`, each file
/// judged as `as_left` gives it, the run done (SYN-54); none outside a git repository.
fn leftovers(
    root: &Path,
    migrations: &[migration::Migration],
    lock: &Lock,
    as_left: &dyn Fn(&str) -> Option<String>,
) -> Vec<String> {
    let names = migration::renamed(migrations);
    if names.is_empty() {
        return Vec::new();
    }
    // quiet: outside a git repository, git says so on stderr
    let listed = match Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .stderr(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => return Vec::new(),
    };
    let mut paths: Vec<&str> = listed
        .split('\0')
        .filter(|rel| {
            !rel.is_empty()
                && *rel != LOCK
                && !lock.files.contains_key(*rel)
                && !HISTORY.iter().any(|dir| rel.starts_with(dir))
        })
        .collect();
    paths.sort_unstable();
    let mut warnings = Vec::new();
    for rel in paths {
        let Some(text) = as_left(rel) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            for (format, name) in &names {
                if migration::cites(line, name) {
                    warnings.push(format!(
                        "warning: {rel}:{} cites {}, named {} since the format {format}",
                        number + 1,
                        name.old,
                        name.new
                    ));
                }
            }
        }
    }
    warnings
}

/// Copy the seeds of `seeds` the project does not have.
fn write_seeds(
    root: &Path,
    seeds: &Path,
    file_set: &BTreeSet<&String>,
    options: SyncOptions,
    report: &mut Report,
) -> Result<()> {
    for rel in list(seeds)? {
        let dst = root.join(&rel);
        // `symlink_metadata`: a dangling symlink is something the project has. A managed path is not a seed, nor the
        // configuration, which `init` writes.
        if file_set.contains(&rel) || rel == CONFIG || fs::symlink_metadata(&dst).is_ok() {
            continue;
        }
        if !options.dry_run {
            write_as(
                &dst,
                &fs::read(seeds.join(&rel))?,
                is_executable(&seeds.join(&rel)),
            )?;
        }
        report.entries.push((rel, Outcome::Seeded));
    }
    Ok(())
}

/// The commands of the hooks of `source` that only features the project at `root` turns off run (FEA-14); none when the
/// source or the features cannot be read, which the sync then reports.
fn turned_off(root: &Path, source: &Path) -> Vec<String> {
    let (Ok(files), Ok(wanted)) = (list(&source.join("files")), entries::of_source(source)) else {
        return Vec::new();
    };
    let Ok(left) = features::left_out(root, source, &files, &wanted) else {
        return Vec::new();
    };
    let kept: Vec<&str> = wanted
        .iter()
        .filter(|e| !left.entries.contains(e))
        .filter_map(entries::Entry::command)
        .collect();
    left.entries
        .iter()
        .filter_map(entries::Entry::command)
        .filter(|c| !kept.contains(c))
        .map(str::to_string)
        .collect()
}

/// What `update` did: the revision before and after, and the `sync` that followed.
#[derive(Debug)]
pub struct UpdateReport {
    /// The `rev` the project had, if any.
    pub previous: Option<String>,
    /// The commit the project was moved to, or would be.
    pub rev: String,
    /// False when nothing was written: `--dry-run`, or a conflict.
    pub applied: bool,
    /// True when the update left something to commit: a project file, the configuration or the lock it changed
    /// (SYN-61).
    pub changed: bool,
    pub sync: Report,
}

/// Move the project to `rev` of its git source (default: the head of the default branch): sync to it, then write
/// the resolved commit to `rev` in the configuration. A conflict, an unknown revision, an unreadable source or a hook the `cardinal` on the path refuses (BIN-16) leave the project as it was;
/// an I/O error during the write itself is not rolled back, as for `sync`.
///
/// # Errors
/// When the source is a local directory, the revision is unknown, or the configuration or sync fails.
pub fn update(root: &Path, rev: Option<&str>, options: &SyncOptions) -> Result<UpdateReport> {
    let mut done = update_in(None, root, rev, *options)?;
    if done.applied {
        let (source, _) = source_and_rev(root)?;
        done.sync.warnings.extend(prefetch(
            &cache_dir()?,
            &source,
            &done.rev,
            &run::Programs::default(),
        ));
    }
    Ok(done)
}

fn update_in(
    cache: Option<&Path>,
    root: &Path,
    rev: Option<&str>,
    options: SyncOptions,
) -> Result<UpdateReport> {
    let (source, previous) = source_and_rev(root)?;
    if !is_git_url(&source) {
        bail!("{CONFIG}: `source` is a local directory, which has no revision to update");
    }
    let dir = fetch(&cache_or_default(cache)?, &source)?;
    let commit = resolve_in(&dir, &source, rev)?;
    git(Some(&dir), &["checkout", "--quiet", "--detach", &commit])?;
    readable(
        &dir,
        &format!("{source} at {commit}"),
        &launcher::install(&source, Some(&commit)),
    )?;
    // The hooks of the target must not lock the agent out: the installed launcher has to know their subcommands. A
    // fragment that cannot be read fails the sync below, before anything is written.
    let off = turned_off(root, &dir);
    let mut refused: Vec<String> = fs::read_to_string(dir.join(entries::FRAGMENT))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .map(|fragment| launcher::refused(&fragment))
        .unwrap_or_default()
        .iter()
        .filter(|command| !off.contains(*command))
        .map(|command| {
            format!("the installed cardinal cannot run the hook `{command}` of {commit}")
        })
        .collect();
    if !refused.is_empty() {
        refused.push(format!(
            "install the cardinal of {commit} first, `{}`, then run `cardinal update` again",
            launcher::install(&source, Some(&commit))
        ));
    }
    if !refused.is_empty() && !options.dry_run {
        bail!("{}", refused.join("\n"));
    }
    let path = config_of(root);
    let current = fs::read_to_string(&path)?;
    // judged before anything is written: a configuration `update` cannot edit stops it (SYN-12)
    set_rev(&current, &commit, rev.unwrap_or("default branch"))?;
    // Nothing is written before the dry run shows a clean sync.
    let mut preview = sync_from(
        root,
        &dir,
        SyncOptions {
            dry_run: true,
            ..options
        },
    )?;
    preview
        .warnings
        .extend(refused.iter().map(|line| format!("warning: {line}")));
    let applied = !options.dry_run && preview.conflicts() == 0;
    let lock = fs::read(root.join(LOCK)).ok();
    let mut changed = false;
    let sync = if applied {
        let sync = sync_from(root, &dir, options)?;
        // the configuration as the sync left it: a migration may have rewritten it (SYN-53)
        let text = set_rev(
            &fs::read_to_string(&path)?,
            &commit,
            rev.unwrap_or("default branch"),
        )?;
        write(&path, text.as_bytes())?;
        changed = text != current
            || fs::read(root.join(LOCK)).ok() != lock
            || sync.entries.iter().any(|(_, outcome)| outcome.wrote());
        sync
    } else {
        preview
    };
    Ok(UpdateReport {
        previous,
        rev: commit,
        applied,
        changed,
        sync,
    })
}

/// What `check` found: `problems` make it fail, `warnings` do not.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// Managed files modified or deleted since the last sync, and the native review gate when the project should have
    /// it and does not, and the hook commands the launcher on the path refuses (BIN-15).
    pub problems: Vec<String>,
    /// What the project's settings lack or run in vain (SYN-24 to SYN-26).
    pub warnings: Vec<String>,
}

/// Managed files modified or deleted since the last sync, one line each, and the native review gate when the project
/// should have it and does not. Under CI (the `CI` environment variable is set) the gate is not looked at: a fresh
/// clone has no git hooks, and nobody commits there. It also warns of what the project's settings lack: the deny rules of
/// the review waiver (when the review hook is among the managed files), the hooks they run, the attribution turned off.
/// A hook command whose subcommand the `cardinal` on the path refuses is a problem too
/// (BIN-15).
///
/// # Errors
/// When the project was never synced (no lock file) or the lock cannot be read.
pub fn check(root: &Path) -> Result<Checked> {
    check_with(root, in_ci())
}

/// Whether `CI` says so: set, and not empty, `0` or `false`.
fn in_ci() -> bool {
    std::env::var("CI")
        .is_ok_and(|v| !matches!(v.trim().to_lowercase().as_str(), "" | "0" | "false"))
}

/// The warnings on the retention of the transcripts (SYN-28), printed by `check`, `sync` and `update`; none under CI,
/// and none on the user settings when the project turns off the measure of the harness that reads them (FEA-28): its
/// own settings still sweep the transcripts of every project. Kept out of `check` and `sync`, which read the project
/// only: these read the user settings too.
#[must_use]
pub fn retention_warnings(root: &Path) -> Vec<String> {
    if in_ci() {
        return Vec::new();
    }
    if features::is_off(root, features::HARNESS_MEASURE) {
        return settings::project_retention(root);
    }
    let user = config_dir().map(|dir| dir.join("settings.json"));
    settings::retention(root, user.as_deref())
}

/// The configuration directory of Claude Code: `CLAUDE_CONFIG_DIR`, or else `~/.claude`; `None` when neither is set.
pub(crate) fn config_dir() -> Option<PathBuf> {
    config_dir_of(
        std::env::var_os("CLAUDE_CONFIG_DIR"),
        std::env::var_os("HOME"),
    )
}

fn config_dir_of(config_dir: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let set = |var: Option<OsString>| var.filter(|v| !v.is_empty()).map(PathBuf::from);
    set(config_dir).or_else(|| set(home).map(|h| h.join(".claude")))
}

fn check_with(root: &Path, ci: bool) -> Result<Checked> {
    let Some(lock) = read_lock(root)? else {
        bail!("no {LOCK}: run `cardinal sync` first");
    };
    let gated = gate::manages(&lock.entries, lock.files.keys());
    let text = fs::read_to_string(root.join(entries::SETTINGS)).ok();
    let hooks = text
        .as_deref()
        .and_then(|text| serde_json::from_str(text).ok())
        .map(|settings| launcher::refused(&settings))
        .unwrap_or_default();
    let install = install_hint(root);
    let problems = lock
        .files
        .iter()
        .filter_map(|(rel, locked)| match fingerprint_of(&root.join(rel)) {
            None => Some(format!("missing: {rel}")),
            Some(print) if print != *locked => Some(format!("modified: {rel}")),
            Some(_) => None,
        })
        .chain(entries::missing(text.as_deref(), &lock.entries))
        .chain(gate::problem(root, !ci && gated))
        .chain(hooks.iter().map(|command| {
            format!(
                "refused: the installed cardinal cannot run the hook `{command}` of {}: install {install} first",
                entries::SETTINGS
            )
        }))
        .collect();
    let attribution = !features::is_off(root, features::NO_AI_ATTRIBUTION);
    let mut warnings =
        settings::warnings(root, gated, &|path| root.join(path).is_file(), attribution);
    if source_and_rev(root).is_ok_and(|(source, _)| is_git_url(&source)) {
        warnings.extend(format_hint(lock.format));
    }
    Ok(Checked { problems, warnings })
}

#[cfg(test)]
mod tests;
