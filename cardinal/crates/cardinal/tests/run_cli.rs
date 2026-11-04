//! `cardinal run`, run as the binary with fake `curl` and `cargo` on no network (spec `specs/tool-binaries.md`), and
//! the prefetch of `sync` and `update`.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{executable, get, output, put, script, Home, Run, CONFIG, FORMAT, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE: &str = "https://github.com/owner/repo.git";

/// A fake tools binary (`cardinal tool <name> <code> ...`): prints its arguments, its working directory, stdin and its
/// own path to stdout, a line to stderr, and exits with `<code>`.
const TOOL: &str =
    "echo \"args: $*\"; pwd; cat; echo \"bin: $0\"; echo \"to stderr\" >&2; exit \"$3\"\n";

/// The platform of the release assets: the Rust target triple, `musl` on Linux.
fn target() -> String {
    let os = match std::env::consts::OS {
        "linux" => "unknown-linux-musl",
        "macos" => "apple-darwin",
        other => other,
    };
    format!("{}-{os}", std::env::consts::ARCH)
}

/// A test home whose `curl` serves the files of `<home>/served` by the last part of the URL and whose `cargo install`
/// writes `TOOL` as `<root>/bin/<bin>` (with `cargo`), each logging its call to `<home>/log`.
struct Launcher {
    home: Home,
}

impl Launcher {
    fn new() -> Self {
        let home = Home::new();
        fs::create_dir_all(home.path().join("served")).unwrap();
        fs::write(home.path().join("log"), "").unwrap();
        home.program(
            "curl",
            &format!(
                r#"out=""; url=""
while [ $# -gt 0 ]; do case "$1" in --output) out="$2"; shift 2;; -*) shift;; *) url="$1"; shift;; esac; done
echo "$url" >> '{home}/log'
file='{home}/served/'"${{url##*/}}"
[ -f "$file" ] || exit 22
if [ -n "$out" ]; then cat "$file" > "$out"; else cat "$file"; fi
"#,
                home = home.path().display()
            ),
        );
        Self { home }
    }

    fn with_cargo(self) -> Self {
        self.with_cargo_building("cardinal")
    }

    /// A `cargo install` that builds `bin`: `harness` for a commit from before the rename.
    fn with_cargo_building(self, bin: &str) -> Self {
        self.home.program(
            "cargo",
            &format!(
                r#"echo "$*" >> '{home}/log'
root=""; while [ $# -gt 0 ]; do case "$1" in --root) root="$2"; shift 2;; *) shift;; esac; done
mkdir -p "$root/bin"; printf '#!/bin/sh\n{TOOL}' > "$root/bin/{bin}"
"#,
                home = self.home.path().display()
            ),
        );
        self
    }

    /// Publish `binary` as the asset `<name>-<target>` of the platform, with the digest `digest` (its own when
    /// `None`).
    fn publish(&self, name: &str, binary: &str, digest: Option<&str>) {
        let asset = format!("{name}-{}", target());
        let served = self.home.path().join("served");
        fs::write(served.join(&asset), binary).unwrap();
        let digest = digest.map_or_else(|| sha256(binary.as_bytes()), str::to_string);
        fs::write(
            served.join(format!("{asset}.sha256")),
            format!("{digest}  {asset}\n"),
        )
        .unwrap();
    }

    fn log(&self) -> String {
        fs::read_to_string(self.home.path().join("log")).unwrap()
    }

    /// Where the binary of `commit` for this platform is cached (BIN-2).
    fn cached(&self, commit: &str) -> PathBuf {
        self.home
            .cache()
            .join("cardinal/bin")
            .join(commit)
            .join(target())
            .join("cardinal")
    }

    /// What the directory of the cached binary of `COMMIT` holds besides the binary.
    fn leftovers(&self) -> Vec<String> {
        let dir = self.cached(COMMIT).parent().unwrap().to_path_buf();
        fs::read_dir(dir).map_or_else(
            |_| Vec::new(),
            |d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n != "cardinal")
                    .collect()
            },
        )
    }

    /// `cardinal run <args>` in `cwd` with `stdin`, with `CLAUDE_PROJECT_DIR` set to `project` if any.
    fn run_in(&self, cwd: &Path, project: Option<&Path>, args: &[&str], stdin: &str) -> Run {
        let mut cmd = self.home.command();
        cmd.arg("run").args(args).current_dir(cwd);
        if let Some(project) = project {
            cmd.env("CLAUDE_PROJECT_DIR", project);
        }
        output(cmd, stdin)
    }

    /// `cardinal run t 0` in `root`.
    fn run(&self, root: &Path) -> Run {
        self.run_in(root, None, &["t", "0"], "")
    }
}

/// A project pinned to `COMMIT` of `source`.
fn pinned(source: &str) -> TempDir {
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{source}\"\nrev = \"{COMMIT}\"\n"),
    );
    root
}

/// The binary that ran: the `bin:` line `TOOL` prints.
fn ran(run: &Run) -> PathBuf {
    let line = run.stdout.lines().find_map(|l| l.strip_prefix("bin: "));
    PathBuf::from(line.unwrap_or_else(|| panic!("no tool ran: {run:?}")))
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

// spec: BIN-1, BIN-2
#[test]
fn run_executes_the_cached_binary_of_the_pinned_commit_with_no_fetch() {
    let l = Launcher::new().with_cargo();
    script(&l.cached(COMMIT), TOOL);
    let root = pinned(SOURCE);
    let sub = root.path().join(".claude/sub");
    fs::create_dir_all(&sub).unwrap();

    let run = l.run_in(&sub, None, &["my-tool", "3", "--flag"], "the input\n");

    assert_eq!(run.code, Some(3), "{run:?}");
    let cwd = fs::canonicalize(&sub).unwrap();
    assert_eq!(
        run.stdout,
        format!(
            "args: tool my-tool 3 --flag\n{}\nthe input\nbin: {}\n",
            cwd.display(),
            l.cached(COMMIT).display()
        )
    );
    assert_eq!(run.stderr, "to stderr\n");
    assert_eq!(l.log(), "", "no download nor build");
}

// spec: BIN-1
#[test]
fn a_working_directory_outside_any_project_falls_back_to_the_project_dir() {
    let l = Launcher::new();
    script(&l.cached(COMMIT), TOOL);
    let root = pinned(SOURCE);
    let elsewhere = tempdir().unwrap();

    let run = l.run_in(elsewhere.path(), Some(root.path()), &["t", "0"], "");
    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(run.stdout.starts_with("args: tool t 0\n"), "{run:?}");

    let run = l.run_in(elsewhere.path(), None, &["t", "0"], "");
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("cardinal.toml"), "{run:?}");

    // inside a project, its own pin wins over the project dir: the binary of the other pin is not there
    let other = tempdir().unwrap();
    put(
        other.path(),
        CONFIG,
        &format!(
            "[cardinal]\nsource = \"{SOURCE}\"\nrev = \"{}\"\n",
            "f".repeat(40)
        ),
    );
    let run = l.run_in(root.path(), Some(other.path()), &["t", "0"], "");
    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(ran(&run), l.cached(COMMIT), "{run:?}");
}

// spec: BIN-2
#[test]
fn a_rev_that_is_no_hash_resolves_in_the_clone_sync_left_without_fetching() {
    let l = Launcher::new();
    let (origin, root, commit) = git_source(&l.home);
    let config = get(root.path(), CONFIG).unwrap();
    put(root.path(), CONFIG, &format!("{config}rev = \"main\"\n"));
    l.home.ok(root.path(), &["sync"]);
    script(&l.cached(&commit), TOOL);
    // the source is gone: a fetch would fail
    fs::remove_dir_all(origin.path()).unwrap();

    let run = l.run(root.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(ran(&run), l.cached(&commit), "{run:?}");
}

// spec: BIN-3, BIN-4
#[test]
fn a_missing_binary_is_downloaded_with_its_digest_and_installed_whole() {
    let l = Launcher::new();
    l.publish("cardinal", &format!("#!/bin/sh\n{TOOL}"), None);
    let root = pinned(SOURCE);

    let run = l.run(root.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(run.stdout.starts_with("args: tool t 0\n"), "{run:?}");
    assert!(executable(&l.cached(COMMIT)));
    let asset = format!(
        "https://github.com/owner/repo/releases/download/cardinal-{COMMIT}/cardinal-{}",
        target()
    );
    assert_eq!(l.log(), format!("{asset}\n{asset}.sha256\n"));
    assert!(l.leftovers().is_empty(), "{:?}", l.leftovers());
    // the next run finds it
    assert_eq!(l.run(root.path()).code, Some(0));
    assert_eq!(l.log().lines().count(), 2);
}

// spec: BIN-3
#[test]
fn a_commit_from_before_the_rename_downloads_the_harness_release() {
    let l = Launcher::new();
    l.publish("harness", &format!("#!/bin/sh\n{TOOL}"), None);
    let root = pinned(SOURCE);

    let run = l.run(root.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(executable(&l.cached(COMMIT)));
    let release = "https://github.com/owner/repo/releases/download";
    let (new, old) = (
        format!("{release}/cardinal-{COMMIT}/cardinal-{}", target()),
        format!("{release}/harness-{COMMIT}/harness-{}", target()),
    );
    assert_eq!(l.log(), format!("{new}\n{old}\n{old}.sha256\n"));
    assert!(l.leftovers().is_empty(), "{:?}", l.leftovers());
}

// spec: BIN-10
#[test]
fn an_asset_that_does_not_match_its_digest_is_not_installed() {
    let l = Launcher::new().with_cargo();
    l.publish("cardinal", "tampered", Some(&sha256(b"genuine")));
    let root = pinned(SOURCE);

    let run = l.run(root.path());

    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(
        run.stderr.contains(COMMIT) && run.stderr.contains("SHA-256"),
        "{run:?}"
    );
    assert!(!l.cached(COMMIT).exists());
    assert!(l.leftovers().is_empty(), "{:?}", l.leftovers());
    assert!(!l.log().contains("install"), "a mismatch is not built over");
}

// spec: BIN-5
#[test]
fn with_no_asset_the_binary_is_built_with_cargo_install() {
    for (source, url, downloads) in [
        (SOURCE, SOURCE, true),
        (
            "https://gitlab.com/o/r.git",
            "https://gitlab.com/o/r.git",
            false,
        ),
        (
            "git@gitlab.com:o/r.git",
            "ssh://git@gitlab.com/o/r.git",
            false,
        ),
        // a host alias of the ssh configuration, with no user
        ("backup:o/r.git", "ssh://backup/o/r.git", false),
    ] {
        for bin in ["cardinal", "harness"] {
            let l = Launcher::new().with_cargo_building(bin);
            let root = pinned(source);

            let run = l.run(root.path());

            assert_eq!(run.code, Some(0), "{source}: {run:?}");
            assert!(run.stdout.starts_with("args: tool t 0\n"), "{run:?}");
            assert!(executable(&l.cached(COMMIT)), "{source}");
            let log = l.log();
            let install = log.lines().find(|l| l.starts_with("install")).unwrap();
            assert!(
                install.contains(&format!("--locked --git {url} --rev {COMMIT} --root")),
                "{install}"
            );
            // no crate named: the single package of the commit, `harness` before the rename
            assert!(
                Path::new(install).extension().is_some_and(|e| e == "root"),
                "{install}"
            );
            // a source not on GitHub is built without a download
            assert_eq!(!log.starts_with("install"), downloads, "{source}: {log}");
            assert!(l.leftovers().is_empty(), "{:?}", l.leftovers());
        }
    }
}

// spec: SYN-50
#[test]
fn run_from_a_subdirectory_builds_a_relative_git_source_from_the_project_root() {
    let l = Launcher::new().with_cargo();
    let dir = tempdir().unwrap();
    let physical = fs::canonicalize(dir.path()).unwrap();
    let base = dir.path().join("a b");
    l.home
        .git(dir.path(), &["init", "-q", "-b", "main", "a b/x.git"]);
    let repo = base.join("x.git");
    put(&repo, "files/a.txt", "a");
    l.home.git(&repo, &["add", "."]);
    l.home.git(&repo, &["commit", "-q", "-m", "x"]);
    let commit = l.home.git(&repo, &["rev-parse", "HEAD"]);
    let root = base.join("project");
    put(
        &root,
        CONFIG,
        "[cardinal]\nsource = \"../x.git\"\nrev = \"main\"\n",
    );
    fs::create_dir_all(root.join("sub")).unwrap();

    let run = l.run_in(&root.join("sub"), None, &["t", "0"], "");

    assert_eq!(run.code, Some(0), "{run:?}");
    let url = format!("file://{}/a%20b/x.git", physical.display());
    let log = l.log();
    assert!(
        log.contains(&format!("--git {url} --rev {commit} --root")),
        "{log}"
    );
}

// spec: BIN-11
#[test]
fn with_no_asset_nor_a_build_the_fetch_fails_naming_the_commit_the_platform_and_update() {
    let l = Launcher::new();
    let root = pinned(SOURCE);
    // the `cargo` of the home fails, then there is none
    for missing in [false, true] {
        if missing {
            fs::remove_file(l.home.path().join("bin/cargo")).unwrap();
        }

        let run = l.run(root.path());

        assert_eq!(run.code, Some(3), "{run:?}");
        assert!(
            run.stderr.contains(COMMIT)
                && run.stderr.contains(&target())
                && run.stderr.contains("cardinal update"),
            "{run:?}"
        );
        assert!(!l.cached(COMMIT).exists());
        assert!(l.leftovers().is_empty(), "{:?}", l.leftovers());
    }
}

/// A project on a local source, and the path of the binary a successful build of it reports.
fn local_project(l: &Launcher) -> (TempDir, TempDir, PathBuf) {
    let source = tempdir().unwrap();
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[cardinal]\nsource = \"{}\"\n", source.path().display()),
    );
    let built = l.home.path().join("target-cardinal");
    (source, root, built)
}

/// A `cargo build` that reports `built` (written as `TOOL`), or fails when `built` is `None`.
fn cargo_build(l: &Launcher, built: Option<&Path>) {
    let body = match built {
        Some(built) => {
            script(built, TOOL);
            format!(
                "echo \"$*\" >> '{log}'\necho '{{\"reason\":\"compiler-artifact\",\"target\":{{\"name\":\"cardinal\"}},\"executable\":\"{exe}\"}}'\n",
                log = l.home.path().join("log").display(),
                exe = built.display()
            )
        }
        None => "echo 'error[E0425]: cannot find value' >&2; exit 101\n".to_string(),
    };
    l.home.program("cargo", &body);
}

// spec: BIN-6
#[test]
fn a_local_source_runs_the_crate_it_builds() {
    let l = Launcher::new();
    let (source, root, built) = local_project(&l);
    cargo_build(&l, Some(&built));

    let run = l.run(root.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(ran(&run), built, "the fresh build runs");
    let log = l.log();
    let manifest = fs::canonicalize(source.path()).unwrap().join("Cargo.toml");
    assert!(
        log.starts_with("build ")
            && log.contains("-p cardinal ")
            && log.contains(&manifest.display().to_string()),
        "{log}"
    );
}

// spec: BIN-12
#[test]
fn a_local_source_that_does_not_build_runs_its_last_good_build() {
    let l = Launcher::new();
    let (_source, root, built) = local_project(&l);
    cargo_build(&l, Some(&built));
    assert_eq!(l.run(root.path()).code, Some(0));
    // the build fails now, and its output is gone
    cargo_build(&l, None);
    fs::remove_file(&built).unwrap();

    let run = l.run(root.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(ran(&run).starts_with(l.home.cache()), "{run:?}");
    let warning: Vec<&str> = run
        .stderr
        .lines()
        .filter(|l| l.contains("failed"))
        .collect();
    assert_eq!(warning.len(), 1, "{run:?}");
    assert!(warning[0].contains("last good build"), "{run:?}");
}

// spec: BIN-12, BIN-14
#[test]
fn a_local_source_that_never_built_exits_3_with_the_build_errors() {
    let l = Launcher::new();
    let (_source, root, _built) = local_project(&l);
    cargo_build(&l, None);

    let run = l.run(root.path());

    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(
        run.stderr.contains("error[E0425]") && run.stderr.contains("no earlier build"),
        "{run:?}"
    );
}

/// A git source holding `files/a.txt` and a project pointing at it, with the hash of its head.
fn git_source(home: &Home) -> (TempDir, TempDir, String) {
    let origin = tempdir().unwrap();
    home.git(origin.path(), &["init", "-q", "-b", "main"]);
    put(origin.path(), "files/a.txt", "a");
    put(origin.path(), SOURCE_FORMAT, FORMAT);
    home.git(origin.path(), &["add", "."]);
    home.git(origin.path(), &["commit", "-q", "-m", "x"]);
    let commit = home.git(origin.path(), &["rev-parse", "HEAD"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "[cardinal]\nsource = \"file://{}\"\n",
            origin.path().display()
        ),
    );
    (origin, root, commit)
}

/// The lines of `stderr` that warn of a failed fetch of the binary of `commit`.
fn fetch_warnings<'a>(stderr: &'a str, commit: &str) -> Vec<&'a str> {
    stderr
        .lines()
        .filter(|l| l.starts_with("warning: ") && l.contains(commit))
        .collect()
}

// spec: BIN-13
#[test]
fn sync_and_update_warn_of_a_failed_prefetch_and_dry_runs_fetch_nothing() {
    let l = Launcher::new();
    let (_origin, root, commit) = git_source(&l.home);
    l.home.program(
        "cargo",
        &format!("echo \"$*\" >> '{}/log'; exit 1\n", l.home.path().display()),
    );

    let dry = l.home.ok(root.path(), &["update", "--dry-run"]);
    assert!(fetch_warnings(&dry.stderr, &commit).is_empty(), "{dry:?}");
    assert_eq!(l.log(), "", "a dry run fetches nothing");

    for args in [&["update"][..], &["sync"]] {
        let run = l.home.ok(root.path(), args);
        assert_eq!(
            fetch_warnings(&run.stderr, &commit).len(),
            1,
            "{args:?}: {run:?}"
        );
    }
    let dry = l.home.ok(root.path(), &["sync", "--dry-run"]);
    assert!(fetch_warnings(&dry.stderr, &commit).is_empty(), "{dry:?}");
    assert!(!l.cached(&commit).exists());

    // once the binary builds, it is cached and nothing is said
    let l = l.with_cargo();
    let run = l.home.ok(root.path(), &["sync"]);
    assert!(fetch_warnings(&run.stderr, &commit).is_empty(), "{run:?}");
    assert!(executable(&l.cached(&commit)));
}

/// A copy of the launcher in a directory of its own, to be replaced by `upgrade`, with its original bytes.
fn launcher_copy() -> (TempDir, PathBuf, Vec<u8>) {
    let dir = tempdir().unwrap();
    let copy = dir.path().join("cardinal");
    fs::copy(env!("CARGO_BIN_EXE_cardinal"), &copy).unwrap();
    let bytes = fs::read(&copy).unwrap();
    (dir, copy, bytes)
}

/// The `cardinal` on the `PATH` of the home of `l`.
fn on_path(l: &Launcher) -> String {
    fs::read_to_string(l.home.path().join("bin/cardinal")).unwrap()
}

/// `<launcher> upgrade <args>` with the home of `l`.
fn upgrade(l: &Launcher, launcher: &Path, args: &[&str]) -> Run {
    upgrade_with_path(l, launcher, args, &l.home.path_var())
}

/// `<launcher> upgrade <args>` with the home of `l` and the `PATH` `path`.
fn upgrade_with_path(l: &Launcher, launcher: &Path, args: &[&str], path: &std::ffi::OsStr) -> Run {
    let mut cmd = std::process::Command::new(launcher);
    cmd.env("PATH", path)
        .env("XDG_CACHE_HOME", l.home.cache())
        .env_remove("CLAUDE_PROJECT_DIR")
        .arg("upgrade")
        .args(args)
        .current_dir(l.home.path());
    l.home.isolated(&mut cmd);
    output(cmd, "")
}

// spec: BIN-19
#[test]
fn upgrade_replaces_the_running_launcher_with_the_release_asset_of_the_commit() {
    let l = Launcher::new();
    let binary = format!("#!/bin/sh\n{TOOL}");
    l.publish("cardinal", &binary, None);
    let (dir, copy, _) = launcher_copy();
    let path = std::env::join_paths([
        dir.path().to_path_buf(),
        l.home.path().join("bin"),
        l.home.path().join("sys"),
    ])
    .unwrap();

    let run = upgrade_with_path(&l, &copy, &[SOURCE, COMMIT], &path);

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(fs::read_to_string(&copy).unwrap(), binary);
    assert!(executable(&copy));
    assert!(run.stdout.contains(&copy.display().to_string()), "{run:?}");
    assert!(
        l.log().contains(&format!(
            "https://github.com/owner/repo/releases/download/cardinal-{COMMIT}/cardinal-{}",
            target()
        )),
        "{}",
        l.log()
    );
    let leftovers: Vec<_> = fs::read_dir(copy.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, ["cardinal"], "a whole file renamed into place");
}

// spec: BIN-19
#[test]
fn upgrade_replaces_the_launcher_on_the_path_not_the_binary_it_runs_from() {
    let l = Launcher::new();
    let binary = format!("#!/bin/sh\n{TOOL}");
    l.publish("cardinal", &binary, None);
    // the launcher on the path is a symlink into the cache of another commit, as a hand install may leave it
    let other = l.cached("fedcba9876543210fedcba9876543210fedcba98");
    script(&other, "exit 0\n");
    let on_path = l.home.path().join("bin/cardinal");
    fs::remove_file(&on_path).unwrap();
    std::os::unix::fs::symlink(&other, &on_path).unwrap();
    let (_dir, copy, original) = launcher_copy();

    let run = upgrade(&l, &copy, &[SOURCE, COMMIT]);

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(fs::read_to_string(&on_path).unwrap(), binary);
    assert!(!fs::symlink_metadata(&on_path).unwrap().is_symlink());
    assert_eq!(
        fs::read_to_string(&other).unwrap(),
        "#!/bin/sh\nexit 0\n",
        "the cache of the other commit"
    );
    assert_eq!(fs::read(&copy).unwrap(), original, "the binary it ran from");
}

// spec: BIN-21
#[test]
fn upgrade_refuses_what_it_cannot_install_and_leaves_the_launcher_unchanged() {
    let l = Launcher::new();
    l.publish("cardinal", "#!/bin/sh\n", Some(&"0".repeat(64)));
    let (_dir, copy, original) = launcher_copy();

    for (args, reason) in [
        (&[SOURCE, "0123456"][..], "full commit hash"),
        (&["../a-directory", COMMIT][..], "git source"),
        // the asset does not match its digest, and no `cargo` builds it
        (&[SOURCE, COMMIT][..], "SHA-256"),
    ] {
        let run = upgrade(&l, &copy, args);
        assert_ne!(run.code, Some(0), "{args:?}: {run:?}");
        assert!(run.stderr.contains(reason), "{args:?}: {run:?}");
        assert_eq!(fs::read(&copy).unwrap(), original, "{args:?}");
        assert_eq!(on_path(&l), "#!/bin/sh\nexit 0\n", "{args:?}");
    }

    let other = "fedcba9876543210fedcba9876543210fedcba98";
    let run = upgrade(&l, &copy, &[SOURCE, other]);
    assert_ne!(run.code, Some(0), "{run:?}");
    assert!(run.stderr.contains(other), "no asset nor build: {run:?}");
    assert_eq!(fs::read(&copy).unwrap(), original);
    assert_eq!(on_path(&l), "#!/bin/sh\nexit 0\n");
}
