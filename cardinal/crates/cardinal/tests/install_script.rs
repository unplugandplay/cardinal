//! `.github/install_cardinal.sh`, the CI installer of the managed content, run with `sh` and a fake `cargo` and `curl`
//! first on the path; it reads the configuration with the real `python3`, 3.11 or later (`tomllib`).
#![cfg(unix)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::{output, put, script, Run, CONFIG};
use tempfile::tempdir;

/// Run the installer in `root` with fakes in `bin`: `cargo` logs its arguments to `<bin>/log`, `curl` finds nothing.
fn install(root: &Path, bin: &Path) -> Run {
    script(
        &bin.join("cargo"),
        &format!("echo \"$*\" >> '{}/log'\n", bin.display()),
    );
    script(&bin.join("curl"), "exit 22\n");
    let installer =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../files/.github/install_cardinal.sh");
    let mut cmd = Command::new("sh");
    let path = std::env::join_paths(std::iter::once(bin.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    cmd.arg(installer)
        .current_dir(root)
        .env("PATH", path)
        .env("HOME", bin)
        .env_remove("XDG_CACHE_HOME");
    output(cmd, "")
}

// spec: SYN-50
#[test]
fn a_path_git_source_is_installed_from_its_file_url_from_the_project_root() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("a bé%");
    let root = base.join("project");
    fs::create_dir_all(&root).unwrap();
    // the root is read with its symlinks resolved, a path source as written
    let physical = fs::canonicalize(dir.path()).unwrap();
    let encoded = "a%20b%C3%A9%25";
    for (source, url) in [
        (
            "../x.git".to_string(),
            format!("file://{}/{encoded}/x.git", physical.display()),
        ),
        (
            format!("{}/./x.git", base.display()),
            format!("file://{}/{encoded}/x.git", dir.path().display()),
        ),
        (
            "git@mirror/x.git".to_string(),
            format!(
                "file://{}/{encoded}/project/git%40mirror/x.git",
                physical.display()
            ),
        ),
        ("host:x.git".to_string(), "ssh://host/x.git".to_string()),
    ] {
        let bin = tempdir().unwrap();
        put(
            &root,
            CONFIG,
            &format!("[cardinal]\nsource = \"{source}\"\nrev = \"main\"\n"),
        );

        let run = install(&root, bin.path());

        assert_eq!(run.code, Some(0), "{source}: {run:?}");
        let log = fs::read_to_string(bin.path().join("log")).unwrap_or_default();
        assert_eq!(
            log,
            format!("install --quiet --force --git {url} --rev main --locked\n"),
            "{source}: {run:?}"
        );
    }
}

#[test]
fn a_work_tree_without_the_configuration_fails_naming_it() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stderr.contains(CONFIG), "{run:?}");
    assert!(!bin.path().join("log").exists(), "{run:?}");
}

// spec: SYN-57
#[test]
fn a_configuration_without_a_source_fails_naming_the_key_and_the_file() {
    for config in [
        "[cardinal]\nrev = \"main\"\n",
        "[cardinal]\nsource = \"\"\n",
        "[cardinal]\nsource = 1\n",
        "[other]\nsource = \"host:x.git\"\n",
        "[cardinal]\nsource = \"host:x.git\n",
        "[cardinal]\nsource = \"host:x.git\"\n# \u{ff}\n",
    ] {
        let root = tempdir().unwrap();
        let bin = tempdir().unwrap();
        // `\u{ff}` stands for a byte that is not UTF-8
        let bytes: Vec<u8> = config
            .chars()
            .map(|c| if c == '\u{ff}' { 0xff } else { c as u8 })
            .collect();
        fs::create_dir_all(root.path().join(".claude")).unwrap();
        fs::write(root.path().join(CONFIG), bytes).unwrap();

        let run = install(root.path(), bin.path());

        assert_eq!(run.code, Some(1), "{config}: {run:?}");
        assert!(
            run.stderr
                .contains(&format!("no source read from {CONFIG}")),
            "{config}: {run:?}"
        );
        assert!(!bin.path().join("log").exists(), "{config}: {run:?}");
    }
}

// spec: SYN-58
#[test]
fn a_runner_without_tomllib_fails_naming_python() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();
    put(root.path(), CONFIG, "[cardinal]\nsource = \"host:x.git\"\n");
    script(
        &bin.path().join("python3"),
        "echo \"ModuleNotFoundError: No module named 'tomllib'\" >&2\nexit 1\n",
    );

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stderr.contains("Python 3.11"), "{run:?}");
    assert!(!bin.path().join("log").exists(), "{run:?}");
}

// spec: SYN-56
#[test]
fn every_toml_form_of_the_source_and_rev_is_installed() {
    for config in [
        "[cardinal]\nsource = 'host:x.git'\nrev = 'main'\n",
        "[cardinal]\n  source\t= \"host:x.git\"\n\trev = \"main\"\n",
        "[other]\nsource = \"elsewhere:y.git\"\nrev = \"other\"\n[cardinal]\nsource = \"host:x.git\"\nrev = \"main\"\n",
        "cardinal = { source = \"host:x.git\", rev = \"main\" }\n",
        "cardinal.source = \"host:x.git\"\ncardinal.rev = \"main\"\n",
        "[cardinal]\n\"source\" = \"host:x.git\"\n'rev' = \"main\"\n",
        "[cardinal]\nsource = '''\nhost:x.git'''\nrev = \"\"\"main\"\"\"\n",
        "[cardinal]\nsource = \"\\u0068ost:x.git\" # a comment\nrev = \"main\"\n",
    ] {
        let root = tempdir().unwrap();
        let bin = tempdir().unwrap();
        put(root.path(), CONFIG, config);

        let run = install(root.path(), bin.path());

        assert_eq!(run.code, Some(0), "{config}: {run:?}");
        let log = fs::read_to_string(bin.path().join("log")).unwrap_or_default();
        assert_eq!(
            log, "install --quiet --force --git ssh://host/x.git --rev main --locked\n",
            "{config}: {run:?}"
        );
    }
}

// spec: SYN-56
#[test]
fn an_escaped_quote_and_a_rev_that_is_no_string_are_read_as_cardinal_reads_them() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        "[cardinal]\nsource = \"host:x\\\"y.git\"\nrev = 1\n",
    );

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    let log = fs::read_to_string(bin.path().join("log")).unwrap_or_default();
    assert_eq!(
        log, "install --quiet --force --git ssh://host/x\"y.git --locked\n",
        "{run:?}"
    );
}

// spec: SYN-56
#[test]
fn a_source_of_a_newline_is_read_as_a_source() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();
    put(root.path(), CONFIG, "[cardinal]\nsource = \"\\n\"\n");

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    assert!(
        run.stdout.contains("source '\n' is a local path"),
        "{run:?}"
    );
}
