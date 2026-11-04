use super::*;

#[test]
fn set_rev_ignores_array_lines_and_refuses_what_it_cannot_edit() {
    let text = "[cardinal]\nextra = [\n  [\"a\"],\n]\nsource = \"x\"\nrev = \"old\"\n";
    let out = set_rev(text, "abc", "v1").unwrap();
    assert!(out.ends_with("rev = \"abc\"  # v1\n"), "{out}");
    assert_eq!(out.matches("rev =").count(), 1);
    assert!(set_rev(
        "[cardinal]\nsource = \"x\"\n\"rev\" = \"old\"\n",
        "abc",
        "v1",
    )
    .is_err());
    assert!(set_rev("[cardinal]\nsource = \"x\"\n", "abc", "a\nb").is_err());
}

#[test]
fn only_a_full_hexadecimal_hash_is_fetched_by_hash() {
    assert!(is_full_hash(&"a1".repeat(20)));
    assert!(is_full_hash(&"A1".repeat(32)));
    assert!(!is_full_hash(&"a1".repeat(6)));
    assert!(!is_full_hash(&"g".repeat(40)));
    assert!(!is_full_hash("main"));
}

// spec: SYN-43 (internal: the format compiled in the binary is not printed)
#[test]
fn the_binary_reads_the_format_of_its_own_revision() {
    let declared =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cardinal-format"))
            .unwrap();
    assert_eq!(declared.trim().parse::<u32>().unwrap(), FORMAT);
}

// spec: SYN-49 (internal: the forms of `git@` and `https://` need a remote the binary tests cannot reach)
#[test]
fn a_git_source_is_a_url_or_ends_with_dot_git() {
    for git in [
        "https://host/x",
        "file:///x",
        "git@host:x",
        "/x/.git",
        "../x.git",
    ] {
        assert!(is_git_url(git), "{git}");
    }
    for local in ["../x.git/", "/x/X.GIT", "/x", "x.gitx"] {
        assert!(!is_git_url(local), "{local}");
    }
}

// spec: SYN-50 (internal: a scp-like source needs a remote the binary tests cannot reach)
#[cfg(unix)]
#[test]
fn a_path_git_source_becomes_an_absolute_file_url_and_any_other_source_is_kept() {
    let root = Path::new("/p/q");
    for (source, url) in [
        ("../x.git", "file:///p/x.git"),
        ("./a/../b c/x.git", "file:///p/q/b%20c/x.git"),
        ("/x/.git", "file:///x/.git"),
        ("/x/%é.git", "file:///x/%25%C3%A9.git"),
    ] {
        assert_eq!(resolved(root, source), url, "{source}");
    }
    for kept in [
        "git@host:x.git",
        "host:x.git",
        "file:///x",
        "https://host/x",
        "../x",
    ] {
        assert_eq!(resolved(root, kept), kept);
    }
}

// spec: SYN-50 (internal: the binary tests see the URL only through a clone or the log of a fake cargo)
#[cfg(unix)]
#[test]
fn a_path_git_source_is_resolved_from_the_physical_root_and_its_raw_bytes() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    fs::create_dir_all(real.join("project")).unwrap();
    std::os::unix::fs::symlink(&real, dir.path().join("link")).unwrap();
    assert_eq!(
        resolved(&dir.path().join("link/project"), "../x.git"),
        resolved(&real.join("project"), "../x.git")
    );
    assert_eq!(
        resolved(Path::new(OsStr::from_bytes(b"/caf\xe9")), "x.git"),
        "file:///caf%E9/x.git"
    );
}
