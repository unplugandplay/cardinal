//! Helpers of the tests that run the `harness` binary on temp dirs, offline. The cache, the user settings, the `PATH`
//! (a failing `curl` and `cargo`, a `cardinal` and a `harness` that know every subcommand, and a few programs of the
//! system) live in a
//! temp home: the prefetch of the tools binary (BIN-13), the retention warning (SYN-28), the hooks `check` judges
//! (BIN-15) and git never reach outside it. The fakes are
//! shell scripts: these tests run on Unix.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::{tempdir, TempDir};

/// The configuration and the lock of a project.
pub const CONFIG: &str = ".claude/cardinal.toml";
pub const LOCK: &str = ".claude/cardinal.lock";
/// The file of a source that declares its format, and the format the fixtures declare: the one this `cardinal` reads,
/// with the names above.
pub const SOURCE_FORMAT: &str = "cardinal-format";
pub const FORMAT: &str = "8\n";

/// A new empty source of the format `FORMAT`.
pub fn source() -> TempDir {
    let source = tempdir().unwrap();
    put(source.path(), SOURCE_FORMAT, FORMAT);
    source
}

/// The programs of the system the binary and the fakes run.
const SYSTEM: [&str; 6] = ["git", "cat", "cp", "mkdir", "chmod", "pwd"];

pub fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn get(dir: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(dir.join(rel)).ok()
}

/// What one run of the binary gave.
#[derive(Debug)]
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The lines of stdout, in order.
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// The first word of the outcome printed for `path` (`added`, `conflict`…), if any.
    pub fn outcome(&self, path: &str) -> Option<&str> {
        let suffix = format!(": {path}");
        self.stdout
            .lines()
            .find_map(|line| line.strip_suffix(&suffix))
            .map(|outcome| outcome.split(' ').next().unwrap())
    }
}

/// The home the binary runs in: its cache, its user settings (a year of retention) and its `PATH`.
pub struct Home {
    dir: TempDir,
}

impl Home {
    pub fn new() -> Self {
        let dir = tempdir().unwrap();
        put(
            dir.path(),
            "config/settings.json",
            r#"{"cleanupPeriodDays": 365}"#,
        );
        // the binary sees only these programs of the system, and the fakes of `bin`: a program a test removes from
        // `bin` is missing, whatever the caller has installed
        let sys = dir.path().join("sys");
        fs::create_dir_all(&sys).unwrap();
        let caller = std::env::var_os("PATH").unwrap_or_default();
        for program in SYSTEM {
            let found = std::env::split_paths(&caller)
                .map(|d| d.join(program))
                .find(|p| p.is_file())
                .unwrap_or_else(|| panic!("{program} is not on the PATH"));
            #[cfg(unix)]
            std::os::unix::fs::symlink(found, sys.join(program)).unwrap();
        }
        let home = Self { dir };
        // no network: `curl` fails as on a missing asset, and the prefetch of a git source would build the tools
        // binary with `cargo install`: it fails at once here; the `cardinal` on the path knows every subcommand, so
        // `check` refuses no hook whatever is installed (BIN-15)
        home.program("curl", "exit 22\n");
        home.program("cargo", "exit 1\n");
        home.program("cardinal", "exit 0\n");
        home
    }

    /// The temp home itself.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The cache of the binary (`XDG_CACHE_HOME`).
    pub fn cache(&self) -> PathBuf {
        self.dir.path().join("cache")
    }

    /// Put an `sh` script running `body` as `program`, first on the `PATH` of the binary.
    pub fn program(&self, program: &str, body: &str) {
        script(&self.dir.path().join("bin").join(program), body);
    }

    /// The `PATH` of the binary: the fakes of `bin`, then the programs of `SYSTEM`.
    pub fn path_var(&self) -> std::ffi::OsString {
        std::env::join_paths([self.dir.path().join("bin"), self.dir.path().join("sys")]).unwrap()
    }

    /// The binary, with this home: its cache, its user settings, its `PATH`, outside CI and any session.
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
        cmd.env("PATH", self.path_var())
            .env("XDG_CACHE_HOME", self.cache())
            .env("CLAUDE_CONFIG_DIR", self.dir.path().join("config"))
            .env_remove("CI")
            .env_remove("CLAUDE_PROJECT_DIR");
        self.isolated(&mut cmd);
        cmd
    }

    /// `harness <args> --root <root>`.
    pub fn harness(&self, root: &Path, args: &[&str]) -> Run {
        self.harness_env(root, args, &[])
    }

    /// `harness <args> --root <root>` with the environment variables `env` set.
    pub fn harness_env(&self, root: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
        let mut cmd = self.command();
        cmd.args(args)
            .arg("--root")
            .arg(root)
            .envs(env.iter().copied());
        output(cmd, "")
    }

    /// `git <args>` in `dir` with a test identity, which must succeed; its trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir);
        let out = self.isolated(&mut cmd).output().unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// `cmd` with this home and no git configuration nor repository of the caller: a test run from a git hook or
    /// with a global `core.hooksPath` or `tag.gpgSign` neither touches the real repository nor fails.
    pub fn isolated<'a>(&self, cmd: &'a mut Command) -> &'a mut Command {
        cmd.env("HOME", self.dir.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"));
        for var in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_COMMON_DIR",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            cmd.env_remove(var);
        }
        cmd
    }

    /// `harness <args>`, which must exit 0.
    pub fn ok(&self, root: &Path, args: &[&str]) -> Run {
        let run = self.harness(root, args);
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        run
    }
}

/// Run `cmd` with `stdin`.
pub fn output(mut cmd: Command, stdin: &str) -> Run {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // a tool that does not read its input closes the pipe first
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    let out = child.wait_with_output().unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// Write an executable `sh` script running `body`. A child process writes it, so that no write descriptor of this
/// process, which another test thread's fork could inherit, keeps it busy when it is executed (`ETXTBSY`).
pub fn script(path: &Path, body: &str) {
    let draft = path.with_extension("draft");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&draft, format!("#!/bin/sh\n{body}")).unwrap();
    let copied = Command::new("cp").arg(&draft).arg(path).status().unwrap();
    assert!(copied.success());
    fs::remove_file(draft).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// What a path of a tree holds.
#[derive(Debug, PartialEq, Eq)]
pub enum Node {
    Dir,
    File(Vec<u8>, bool),
    Link(PathBuf),
}

/// Every directory, file (bytes and executable bit) and symlink under `dir`: two equal snapshots are an identical tree.
pub fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Node> {
    let mut nodes = BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(next) = todo.pop() {
        for entry in fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(dir).unwrap().to_path_buf();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            let node = if kind.is_symlink() {
                Node::Link(fs::read_link(&path).unwrap())
            } else if kind.is_dir() {
                todo.push(path);
                Node::Dir
            } else {
                Node::File(fs::read(&path).unwrap(), executable(&path))
            };
            nodes.insert(rel, node);
        }
    }
    nodes
}

#[cfg(unix)]
pub fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o111 != 0
}

#[cfg(unix)]
pub fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(not(unix))]
pub fn executable(_path: &Path) -> bool {
    false
}

/// A source of `FORMAT` with `files/a.txt`, `files/sub/b.txt` and `seed/s.txt`, and a project initialised on it.
pub fn project(home: &Home) -> (TempDir, TempDir) {
    let source = source();
    put(source.path(), "files/a.txt", "a1");
    put(source.path(), "files/sub/b.txt", "b1");
    put(source.path(), "seed/s.txt", "seed1");
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    (source, root)
}

/// The managed hook of the review gate.
pub const RECORD: &str = "cardinal run review-gate record";

/// The `hooks` table running `RECORD` after `Skill`.
pub fn record_hook() -> serde_json::Value {
    serde_json::json!({"PostToolUse": [{"matcher": "Skill", "hooks": [{"type": "command", "command": RECORD}]}]})
}

/// The project of `project`, made a git repository, whose source manages `RECORD` and seeds empty settings for it:
/// `sync` installs the review gate in it.
pub fn gated(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    let fragment = serde_json::json!({"hooks": record_hook()});
    put(
        source.path(),
        "entries/.claude/settings.json",
        &fragment.to_string(),
    );
    put(source.path(), "seed/.claude/settings.json", "{}");
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    (source, root)
}
