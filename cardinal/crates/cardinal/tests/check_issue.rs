//! Behavior test for `cardinal tool check-issue`: accepts a good issue, flags each template violation.

const GOOD: &str = "---\npriority: medium\n---\n# Fix the thing\n\n# Goal\n\nMake it work.\n\n# Approach\n\n1. Do it.\n\n# Resources\n\n* `a.rs`\n";

#[test]
fn check_issue() {
    let script = std::path::PathBuf::from("/tools/check_issue.rs");
    let tmp = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues/done")).unwrap();
    std::fs::write(tmp.join("issues/done/old.md"), "x").unwrap();

    let run = |file: &str, body: &str| {
        std::fs::write(tmp.join(file), body).unwrap();
        let out = tool_command(&script)
            .arg(file)
            .current_dir(&tmp)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (ok, out) = run("issues/good.md", GOOD);
    check("good issue passes", ok && out.is_empty());
    let (ok, out) = run("issues/Bad-Name.md", GOOD);
    check(
        "non snake_case name is flagged",
        !ok && out.contains("snake_case"),
    );
    let (ok, out) = run("issues/old.md", GOOD);
    check(
        "name already in done/ is flagged",
        !ok && out.contains("already exists"),
    );
    let (ok, out) = run(
        "issues/a.md",
        &GOOD.replace("# Approach\n\n1. Do it.\n\n", ""),
    );
    check(
        "missing section is flagged",
        !ok && out.contains("`# Approach`"),
    );
    let (ok, out) = run("issues/b.md", &format!("{GOOD}\n# Tracking\n"));
    check(
        "tracking section is flagged",
        !ok && out.contains("Tracking"),
    );
    let (ok, out) = run("issues/c.md", &GOOD.replace("Make it work.", ""));
    check("empty goal is flagged", !ok && out.contains("empty"));
    let (ok, out) = run(
        "issues/d.md",
        &GOOD.replace("Make it work.", "See /home/me/x."),
    );
    check(
        "private path is flagged",
        !ok && out.contains("private path"),
    );
    let (ok, out) = run("issues/e.md", &GOOD.replace("# Fix the thing\n\n", ""));
    check("missing title is flagged", !ok && out.contains("title"));

    let (ok, out) = run(
        "issues/f.md",
        &GOOD.replace(
            "# Resources",
            "# Specs\n\n* `specs/rerun.md`: RER-2\n\n# Resources",
        ),
    );
    check("filled specs section passes", ok && out.is_empty());
    let (ok, out) = run(
        "issues/g.md",
        &GOOD.replace("# Resources", "# Specs\n\n# Resources"),
    );
    check(
        "empty specs section is flagged",
        !ok && out.contains("`# Specs` is empty"),
    );

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// Runs `check-issue issues/a.md` on `text` in a directory holding the open issue `b` and the closed one `old`:
/// (success, stdout).
fn check_front_matter(text: &str) -> (bool, String) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("issues/done")).unwrap();
    std::fs::write(tmp.path().join("issues/b.md"), "# B\n").unwrap();
    std::fs::write(tmp.path().join("issues/done/old.md"), "# Old\n").unwrap();
    std::fs::write(tmp.path().join("issues/a.md"), text).unwrap();
    let out = tool_command("check_issue.rs")
        .arg("issues/a.md")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

// spec: CKI-1
/// A new issue needs one of the three priorities.
#[test]
fn a_missing_or_unknown_priority_is_flagged() {
    let body = GOOD
        .split_once("---\n# ")
        .map(|(_, b)| format!("# {b}"))
        .unwrap();
    for text in [body.clone(), format!("---\npriority: urgent\n---\n{body}")] {
        let (ok, out) = check_front_matter(&text);
        assert!(
            !ok && out.contains("no priority: set `priority: high`"),
            "{out}"
        );
    }
}

// spec: CKI-2
/// `depends_on` must be a list of known issues other than the issue itself.
#[test]
fn a_wrong_dependency_is_flagged() {
    let with = |deps: &str| {
        GOOD.replace(
            "priority: medium\n",
            &format!("priority: medium\ndepends_on: {deps}\n"),
        )
    };
    let (ok, out) = check_front_matter(&with("[b, gone]"));
    assert!(
        !ok && out.contains("`gone`") && !out.contains("`b`"),
        "{out}"
    );
    let (ok, out) = check_front_matter(&with("[a]"));
    assert!(!ok && out.contains("names the issue itself"), "{out}");
    let (ok, out) = check_front_matter(&with("b"));
    assert!(!ok && out.contains("not a list"), "{out}");
}

// spec: CKI-3
/// A priority with no dependency, an empty list, or dependencies on open and closed issues pass.
#[test]
fn valid_keys_pass() {
    for deps in ["", "depends_on: []\n", "depends_on: [b, old]\n"] {
        let (ok, out) = check_front_matter(
            &GOOD.replace("priority: medium\n", &format!("priority: medium\n{deps}")),
        );
        assert!(ok && out.is_empty(), "{deps}: {out}");
    }
}

/// `cardinal tool <name>` for the script path the checks were written against (`next_issue.rs` is `next-issue`).
fn tool_command(script: impl AsRef<std::path::Path>) -> std::process::Command {
    let stem = script
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('_', "-");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.arg("tool").arg(stem);
    cmd
}

// spec: FEA-9
/// A co-author trailer is flagged, unless the project turns off the feature `no-ai-attribution`.
#[test]
fn an_attribution_line_is_flagged_unless_the_project_turns_off_the_ban() {
    let body = GOOD.replace("Make it work.", "Co-Authored-By: someone");
    let (ok, out) = check_front_matter(&body);
    assert!(!ok && out.contains("attribution line"), "{out}");
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("issues/done")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
    std::fs::write(
        tmp.path().join(".claude/cardinal.toml"),
        "[cardinal]\nsource = \"x\"\ndisabled = [\"no-ai-attribution\"]\n",
    )
    .unwrap();
    std::fs::write(tmp.path().join("issues/a.md"), &body).unwrap();
    let out = tool_command("check_issue.rs")
        .arg("issues/a.md")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && stdout.is_empty(), "{stdout}");
    // the issue finds the same root, named from the root or from its own directory
    let check = |file: &str, dir: &std::path::Path| {
        let out = tool_command("check_issue.rs")
            .arg(file)
            .current_dir(dir)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (ok, out) = check("a.md", &tmp.path().join("issues"));
    assert!(ok && out.is_empty(), "{out}");
    // the same file is flagged once the project leaves the ban on
    std::fs::write(
        tmp.path().join(".claude/cardinal.toml"),
        "[cardinal]\nsource = \"x\"\n",
    )
    .unwrap();
    for (file, dir) in [
        ("issues/a.md", tmp.path().to_path_buf()),
        ("a.md", tmp.path().join("issues")),
    ] {
        let (ok, out) = check(file, &dir);
        assert!(!ok && out.contains("attribution line"), "{file}: {out}");
    }
}

// spec: CKI-4
/// A done issue is refused with one problem, named from the root or from its own directory.
#[test]
fn a_done_issue_is_refused_with_one_problem() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("issues/done")).unwrap();
    std::fs::write(tmp.path().join("issues/b.md"), "# B\n").unwrap();
    std::fs::write(
        tmp.path().join("issues/done/a.md"),
        GOOD.replace("priority: medium\n", "priority: medium\ndepends_on: [b]\n"),
    )
    .unwrap();
    for (file, dir) in [
        ("issues/done/a.md", tmp.path().to_path_buf()),
        ("a.md", tmp.path().join("issues/done")),
    ] {
        let out = tool_command("check_issue.rs")
            .arg(file)
            .current_dir(&dir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !out.status.success()
                && stdout
                    == format!("{file}: a done issue: check-issue checks a new issue of issues/\n"),
            "{file}: {stdout}"
        );
    }
}
