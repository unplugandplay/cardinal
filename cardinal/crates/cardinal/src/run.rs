//! `cardinal run <tool>`: run a tool of Cardinal with the `cardinal` binary of the revision the project pins
//! (spec `specs/tool-binaries.md`, ADR 0010).
//!
//! The installed `cardinal` is only a launcher. With a git source it executes the binary cached for the pinned commit
//! and the platform, `<cache>/cardinal/bin/<commit>/<target>/cardinal`, fetched once: the release asset of the commit
//! (GitHub sources, SHA-256 checked; the `harness` release of a commit from before the rename), else built by `cargo
//! install`. With a local directory source (the repository that develops the tools) it builds that directory's crate
//! with `cargo build`, and keeps a copy of the last good build so that a working tree that does not compile does not
//! block every hook. The binary then runs `cardinal tool <name>`.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::{
    cache_dir, clone_dir, fetch, hash, is_full_hash, is_git_url, is_scp_like, resolve_in,
    set_executable, source_and_rev, CONFIG,
};

/// The external programs the launcher runs, replaced by fakes in the tests.
#[derive(Clone, Debug)]
pub struct Programs {
    pub curl: OsString,
    pub cargo: OsString,
}

impl Default for Programs {
    fn default() -> Self {
        Self {
            curl: "curl".into(),
            cargo: "cargo".into(),
        }
    }
}

/// The platform the release assets are named after. Linux builds are static (`musl`), so they run whatever the glibc.
#[must_use]
pub fn target() -> String {
    let os = match std::env::consts::OS {
        "linux" => "unknown-linux-musl",
        "macos" => "apple-darwin",
        other => other,
    };
    format!("{}-{os}", std::env::consts::ARCH)
}

/// The nearest directory from `start` up that holds `.claude/cardinal.toml`, else from `fallback` up (the project of the
/// session, when the agent left it).
pub(crate) fn project_root(start: &Path, fallback: Option<&Path>) -> Result<PathBuf> {
    let nearest = |from: &Path| {
        let from = fs::canonicalize(from).ok()?;
        from.ancestors()
            .find(|dir| dir.join(CONFIG).is_file())
            .map(Path::to_path_buf)
    };
    nearest(start)
        .or_else(|| fallback.and_then(nearest))
        .with_context(|| {
            format!(
                "no {CONFIG} in {} or above: run `cardinal init <source>`",
                start.display()
            )
        })
}

/// The command that runs `tool` with `args` for the project holding `start` (else `fallback`): the binary of the
/// pinned revision, fetched or built first when needed (BIN-1).
///
/// # Errors
/// When the project has no configuration, or the binary can be neither fetched nor built.
pub fn command(
    start: &Path,
    fallback: Option<&Path>,
    cache: Option<&Path>,
    programs: &Programs,
    tool: &[OsString],
) -> Result<Command> {
    let root = project_root(start, fallback)?;
    let cache = match cache {
        Some(cache) => cache.to_path_buf(),
        None => cache_dir()?,
    };
    let (source, rev) = source_and_rev(&root)?;
    let binary = if is_git_url(&source) {
        let commit = match rev {
            Some(rev) if is_full_hash(&rev) => rev,
            // in the clone the last `sync` fetched, so the revision of the synced files: a hook does not fetch at each
            // run; a clone that is missing or does not know the name is fetched once
            rev => {
                let clone = clone_dir(&cache, &source);
                let cached = clone
                    .join(".git")
                    .exists()
                    .then(|| resolve_in(&clone, &source, rev.as_deref()).ok())
                    .flatten();
                match cached {
                    Some(commit) => commit,
                    None => resolve_in(&fetch(&cache, &source)?, &source, rev.as_deref())?,
                }
            }
        };
        binary_of(&cache, &source, &commit, programs)?
    } else {
        local_binary(&root.join(&source), &cache, programs)?
    };
    let mut cmd = Command::new(binary);
    cmd.arg("tool").args(tool);
    Ok(cmd)
}

/// Where the binary of `commit` for this platform is cached.
fn cached(cache: &Path, commit: &str) -> PathBuf {
    cache
        .join("cardinal")
        .join("bin")
        .join(commit)
        .join(target())
        .join("cardinal")
}

/// The cached binary of `commit`, downloaded or built first when missing (BIN-2 to BIN-5, BIN-10, BIN-11).
///
/// # Errors
/// When the downloaded asset does not match its digest, or it cannot be downloaded nor built.
///
/// # Panics
/// Never: a cached binary always has a directory.
pub fn binary_of(cache: &Path, source: &str, commit: &str, programs: &Programs) -> Result<PathBuf> {
    let binary = cached(cache, commit);
    if binary.is_file() {
        return Ok(binary);
    }
    let dir = binary.parent().expect("a cached binary has a directory");
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let staging = Staging::new(dir);
    let target = target();
    for url in asset_urls(source, commit, &target) {
        if let Some(digest) = download(programs, &url, &staging.path) {
            let actual = hash(&fs::read(&staging.path)?);
            if digest != actual {
                bail!("the cardinal binary of {commit} for {target} does not match its published SHA-256 ({actual} instead of {digest}): not installed");
            }
            return staging.install(&binary);
        }
    }
    if build(programs, source, commit, &staging.path) {
        return staging.install(&binary);
    }
    bail!("no cardinal binary for {commit} on {target}: no release asset, and `cargo` is missing or failed to build it; run `cardinal update` to pin a published revision, or install Rust")
}

/// The names of the binary, its release and its asset: `cardinal`, then `harness` for a commit from before the rename.
const NAMES: [&str; 2] = ["cardinal", "harness"];

/// `https://github.com/<owner>/<repo>/releases/download/<name>-<commit>/<name>-<target>` of each of the `NAMES` for a
/// GitHub source, none for another source.
fn asset_urls(source: &str, commit: &str, target: &str) -> Vec<String> {
    let Some(path) = source
        .strip_prefix("https://github.com/")
        .or_else(|| source.strip_prefix("git@github.com:"))
        .or_else(|| source.strip_prefix("ssh://git@github.com/"))
    else {
        return Vec::new();
    };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    NAMES
        .iter()
        .map(|name| {
            format!("https://github.com/{path}/releases/download/{name}-{commit}/{name}-{target}")
        })
        .collect()
}

/// Download `url` to `to` and its digest from `<url>.sha256`; `None` when either cannot be downloaded.
fn download(programs: &Programs, url: &str, to: &Path) -> Option<String> {
    let curl = |extra: &[&std::ffi::OsStr], url: &str| {
        Command::new(&programs.curl)
            .args(["--fail", "--silent", "--location"])
            .args(extra)
            .arg(url)
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
    };
    curl(&["--output".as_ref(), to.as_os_str()], url)?;
    curl(&[], &format!("{url}.sha256")).and_then(|out| {
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .map(str::to_ascii_lowercase)
    })
}

/// The URL cargo reads for a git source: a scp-style `[user@]host:path` becomes `ssh://[user@]host/path`.
pub(crate) fn cargo_url(source: &str) -> String {
    match source.split_once(':') {
        Some((host, path)) if is_scp_like(source) => {
            format!("ssh://{host}/{}", path.trim_start_matches('/'))
        }
        _ => source.to_string(),
    }
}

/// Build the binary of `commit` with `cargo install` and put it at `to`; false when cargo is missing or fails. No crate
/// is named: the source holds one package, `cardinal`, or `harness` before the rename.
fn build(programs: &Programs, source: &str, commit: &str, to: &Path) -> bool {
    let root = to.with_extension("root");
    let url = cargo_url(source);
    let built = Command::new(&programs.cargo)
        .args([
            "install", "--quiet", "--locked", "--git", &url, "--rev", commit, "--root",
        ])
        .arg(&root)
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
        && NAMES
            .iter()
            .any(|name| fs::rename(root.join("bin").join(name), to).is_ok());
    let _ = fs::remove_dir_all(&root);
    built
}

/// Replace `to` with a copy of `binary`, renamed whole into place (BIN-19, as BIN-4).
///
/// # Errors
/// When the copy cannot be written next to `to` or renamed over it.
pub fn replace(binary: &Path, to: &Path) -> Result<()> {
    let dir = to
        .parent()
        .context("the running cardinal has no directory")?;
    let staging = Staging::new(dir);
    fs::copy(binary, &staging.path).with_context(|| format!("writing next to {}", to.display()))?;
    staging.install(to)?;
    Ok(())
}

/// A file being written next to its final place, removed unless installed.
struct Staging {
    path: PathBuf,
}

impl Staging {
    fn new(dir: &Path) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        Self {
            path: dir.join(format!(".partial-{}-{nanos}", std::process::id())),
        }
    }

    /// Make the staged file executable and rename it to `to`, so no reader sees it partly written (BIN-4).
    fn install(self, to: &Path) -> Result<PathBuf> {
        set_executable(&self.path, true)?;
        fs::rename(&self.path, to).with_context(|| format!("installing {}", to.display()))?;
        Ok(to.to_path_buf())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir_all(self.path.with_extension("root"));
    }
}

/// The binary of the crate of a local source: built now, else the last good build (BIN-6, BIN-12).
fn local_binary(source: &Path, cache: &Path, programs: &Programs) -> Result<PathBuf> {
    let source = fs::canonicalize(source)
        .with_context(|| format!("reading the source {}", source.display()))?;
    let dir = cache.join("cardinal").join("bin").join(format!(
        "local-{}",
        &hash(source.as_os_str().as_encoded_bytes())[..12]
    ));
    let last_good = dir.join("cardinal");
    let out = Command::new(&programs.cargo)
        .args([
            "build",
            "--quiet",
            "--message-format=json-render-diagnostics",
            "-p",
            "cardinal",
            "--manifest-path",
        ])
        .arg(source.join("Cargo.toml"))
        .stdin(Stdio::null())
        .output();
    let built = out
        .as_ref()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .filter(|m| m["target"]["name"] == "cardinal")
                .find_map(|m| m["executable"].as_str().map(PathBuf::from))
        });
    if let Some(built) = built {
        keep(&built, &dir, &last_good)?;
        return Ok(built);
    }
    let errors = match &out {
        Ok(o) => String::from_utf8_lossy(&o.stderr).into_owned(),
        Err(e) => format!("cannot run cargo: {e}\n"),
    };
    if last_good.is_file() {
        eprintln!(
            "cardinal: the build of {} failed, running its last good build",
            source.display()
        );
        return Ok(last_good);
    }
    bail!(
        "{errors}the build of {} failed and it has no earlier build",
        source.display()
    )
}

/// Copy `built` to `last_good` when it is newer, through a rename.
fn keep(built: &Path, dir: &Path, last_good: &Path) -> Result<()> {
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    if last_good.is_file() && modified(last_good) >= modified(built) {
        return Ok(());
    }
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let staging = Staging::new(dir);
    fs::copy(built, &staging.path).with_context(|| format!("copying {}", built.display()))?;
    staging.install(last_good).map(drop)
}
