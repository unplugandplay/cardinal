//! Behavior test for `cardinal tool check-specs`: accepts good specs, flags each violation of `specs/README.md`.

use std::cell::Cell;
use std::path::Path;
use std::process::Command;

const SPEC: &str = "---\nid: RER\nstatus: accepted\nadrs: [0005]\n---\n\n# Rerun\n\n## Intent\n\nReuse.\n\n## Requirements\n\n- **RER-1** (deterministic) A pass must reuse a saved result.\n- **RER-2** (deterministic) A run must resume.\n\n## Scenarios\n\n- (RER-1) Given a, when b, then c.\n\n## Non-goals\n\n- None.\n";
const TESTS: &str = "// spec: RER-1, RER-2\n#[test]\nfn reuses() {}\n";

#[test]
fn check_specs() {
    let script = std::path::PathBuf::from("/tools/check_specs.rs");
    let tmp = tempfile::tempdir().unwrap().keep();
    let failed = Cell::new(false);
    let report = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed.set(failed.get() || !ok);
    };
    let strict = Cell::new(false);
    let case =
        |name: &str, spec: &str, tests: &str, others: &[(&str, &str)], expect: Option<&str>| {
            let _ = std::fs::remove_dir_all(&tmp);
            for dir in ["specs", "crates/a/src", ".claude/hooks", "docs/adr"] {
                std::fs::create_dir_all(tmp.join(dir)).unwrap();
            }
            std::fs::write(tmp.join("docs/adr/0005-incremental.md"), "x").unwrap();
            std::fs::write(tmp.join("specs/README.md"), "not a spec").unwrap();
            std::fs::write(tmp.join("specs/rerun.md"), spec).unwrap();
            std::fs::write(tmp.join("crates/a/src/t.rs"), tests).unwrap();
            for (path, body) in others {
                std::fs::create_dir_all(tmp.join(path).parent().unwrap()).unwrap();
                std::fs::write(tmp.join(path), body).unwrap();
            }
            let mut command = tool_command(&script);
            command.arg(&tmp);
            if strict.get() {
                command.arg("--strict");
            }
            let out = command.output().unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let ok = match expect {
                None => out.status.success() && stdout.is_empty(),
                Some(text) => !out.status.success() && stdout.contains(text),
            };
            report(name, ok);
            if !ok {
                println!("{stdout}{stderr}");
            }
            stderr
        };

    case("good spec with traced tests passes", SPEC, TESTS, &[], None);
    case(
        "a draft spec needs no test",
        &SPEC.replace("accepted", "draft"),
        "",
        &[],
        None,
    );
    traceability_checks(&case);
    format_checks(&case);
    tree_checks(&case);
    outside_git(&case);
    any_layout(&case, &report);
    any_language(&case);
    any_language_placement(&case);

    deprecation_checks(&case, &report);

    marker_checks(&case, &report);
    wording_checks(&case, &report);
    boundary_checks(&case, &report);
    internal_dirs_checks(&case, &report);

    strict.set(true);
    case(
        "strict: a managed test citing a spec the project lacks is flagged",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: PAR-1\nfn t() {}\n")],
        Some("PAR-1"),
    );
    case(
        "strict: a managed test citing a defined requirement passes",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: RER-1\nfn t() {}\n")],
        None,
    );

    case(
        "strict: an unknown id of a present spec is flagged",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: RER-9\nfn t() {}\n")],
        Some("RER-9"),
    );
    strict.set(false);

    let bad = tool_command(&script)
        .arg("--strcit")
        .arg(tmp.join("missing"))
        .output()
        .unwrap();
    let ok = bad.status.code() == Some(2);
    report("an unknown option is rejected", ok);
    // the tree of the last case still holds the unknown id RER-9 under `.claude/`
    let first = tool_command(&script)
        .arg("--strict")
        .arg(&tmp)
        .output()
        .unwrap();
    let ok = !first.status.success() && String::from_utf8_lossy(&first.stdout).contains("RER-9");
    report("--strict may come before the root", ok);

    in_git(&script, &report);

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed.get(), "a check failed: see the FAIL lines");
}

/// The citations of the tests against the requirements of the specs.
fn traceability_checks(case: &Case) {
    case(
        "untraced requirement of an accepted spec is flagged",
        SPEC,
        "// spec: RER-1\nfn t() {}\n",
        &[],
        Some("RER-2"),
    );
    case(
        "unknown id in a test is flagged",
        SPEC,
        &format!("{TESTS}// spec: RER-9\n"),
        &[],
        Some("RER-9"),
    );
    case(
        "a managed test citing a spec the project lacks passes",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: PAR-1\nfn t() {}\n")],
        None,
    );
    case(
        "a managed test citing an unknown id of a present spec is flagged",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: RER-9\nfn t() {}\n")],
        Some("RER-9"),
    );
    case(
        "a crate test citing a spec the project lacks is flagged",
        SPEC,
        &format!("{TESTS}// spec: PAR-1\nfn t() {{}}\n"),
        &[],
        Some("PAR-1"),
    );
}

/// The front matter, sections and requirement lines of a spec.
fn format_checks(case: &Case) {
    case(
        "bad status is flagged",
        &SPEC.replace("accepted", "done"),
        TESTS,
        &[],
        Some("status"),
    );
    case(
        "requirement with another prefix is flagged",
        &SPEC.replace("**RER-2**", "**ABC-2**"),
        TESTS,
        &[],
        Some("ABC-2"),
    );
    case(
        "duplicate requirement id is flagged",
        &SPEC.replace("**RER-2**", "**RER-1**"),
        TESTS,
        &[],
        Some("duplicate"),
    );
    case(
        "missing section is flagged",
        &SPEC.replace("## Non-goals\n\n- None.\n", ""),
        TESTS,
        &[],
        Some("Non-goals"),
    );
    case(
        "unknown adr is flagged",
        &SPEC.replace("[0005]", "[0099]"),
        TESTS,
        &[],
        Some("0099"),
    );
    case(
        "unknown kind is flagged",
        &SPEC.replace("(deterministic) A run", "(manual) A run"),
        TESTS,
        &[],
        Some("kind"),
    );
    case(
        "withdrawn requirement needs no test",
        &SPEC.replace("A run must resume.", "withdrawn: A run must resume."),
        "// spec: RER-1\nfn t() {}\n",
        &[],
        None,
    );
    case(
        "duplicate spec id across files is flagged",
        SPEC,
        TESTS,
        &[("specs/other.md", SPEC)],
        Some("RER"),
    );

    case(
        "a live requirement that mentions withdrawn is still traced",
        &SPEC.replace("A run must resume.", "A withdrawn run must resume."),
        "// spec: RER-1\nfn t() {}\n",
        &[],
        Some("RER-2"),
    );
    case(
        "malformed requirement id is flagged",
        &SPEC.replace("**RER-2**", "**rer-2**"),
        TESTS,
        &[],
        Some("rer-2"),
    );
    case(
        "citation not above a fn is flagged",
        SPEC,
        &format!("{TESTS}// spec: RER-1\nlet x = 1;\n"),
        &[],
        Some("right above"),
    );
    case(
        "citation above attributes and a fn passes",
        SPEC,
        "// spec: RER-1, RER-2\n#[tokio::test]\n#[ignore]\nasync fn t() {}\n",
        &[],
        None,
    );
    case(
        "spec without front matter does not collide with another",
        "# no front matter\n",
        TESTS,
        &[("specs/other.md", "# none either\n")],
        Some("front matter"),
    );
}

/// The tests found in every directory of the tree, whatever its name.
fn tree_checks(case: &Case) {
    case(
        "a test under a module directory named worktrees is traced",
        SPEC,
        "// spec: RER-1\nfn t() {}\n",
        &[("crates/a/src/worktrees/t.rs", "// spec: RER-2\nfn t() {}\n")],
        None,
    );
    case(
        "a test under tests/target is traced",
        SPEC,
        "// spec: RER-1\nfn t() {}\n",
        &[("crates/a/tests/target/t.rs", "// spec: RER-2\nfn t() {}\n")],
        None,
    );
    case(
        "a citation under src/worktrees is checked",
        SPEC,
        TESTS,
        &[("crates/a/src/worktrees/t.rs", "// spec: RER-9\nfn t() {}\n")],
        Some("RER-9"),
    );
    case(
        "a test of a harness script under .claude traces a requirement",
        SPEC,
        "",
        &[(".claude/hooks/test_h.rs", TESTS)],
        None,
    );
    case(
        "a citation under .claude not above a fn is flagged",
        SPEC,
        TESTS,
        &[(".claude/hooks/test_h.rs", "// spec: RER-1\nlet x = 1;\n")],
        Some("right above"),
    );
}

/// The deprecated `quality` kind and `implemented` status, read with a warning.
fn deprecation_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let quality = SPEC.replace(
        "(deterministic) A run must resume.",
        "(quality) Docs must be good.",
    );
    let stderr = case(
        "a quality requirement needs no test",
        &quality,
        "// spec: RER-1\nfn t() {}\n",
        &[],
        None,
    );
    report(
        "a quality requirement is warned of",
        stderr.contains("RER-2: kind `quality` is deprecated"),
    );
    let withdrawn = quality.replace("Docs must", "withdrawn: Docs must");
    let stderr = case(
        "a withdrawn quality requirement passes",
        &withdrawn,
        "// spec: RER-1\nfn t() {}\n",
        &[],
        None,
    );
    report(
        "a withdrawn quality requirement is not warned of",
        !stderr.contains("kind `quality` is deprecated"),
    );

    let implemented = SPEC.replace("status: accepted", "status: implemented");
    case(
        "an implemented spec is enforced like accepted",
        &implemented,
        "// spec: RER-1\nfn t() {}\n",
        &[],
        Some("RER-2"),
    );
    let stderr = case(
        "an implemented spec passes as accepted",
        &implemented,
        TESTS,
        &[],
        None,
    );
    report(
        "an implemented spec is warned of",
        stderr.contains(
            "warning: specs/rerun.md: status `implemented` is deprecated, use `accepted`",
        ),
    );
}

/// Runs one checker case: name, spec, crate tests, other files, expected stdout text (`None`: passes); returns stderr.
type Case<'a> = dyn Fn(&str, &str, &str, &[(&str, &str)], Option<&str>) -> String + 'a;

// spec: PAR-16
/// The cases above run outside a git repository: every `.rs` file of the root is scanned there.
fn outside_git(case: &Case) {
    case(
        "outside git, a test under .claude/worktrees is traced",
        SPEC,
        "// spec: RER-1\nfn t() {}\n",
        &[(".claude/worktrees/x/t.rs", "// spec: RER-2\nfn t() {}\n")],
        None,
    );
    case(
        "outside git, a citation under .claude/worktrees is checked",
        SPEC,
        TESTS,
        &[(".claude/worktrees/x/t.rs", "// spec: RER-9\nfn t() {}\n")],
        Some("RER-9"),
    );
}

// spec: TRC-9, TRC-10
/// A crate at the root is read like one under `crates/`; the build output and the skipped directories are not.
fn any_layout(case: &Case, report: &dyn Fn(&str, bool)) {
    let cited = |id: &str| format!("// spec: {id}\n#[test]\nfn t() {{}}\n");
    let stderr = case(
        "a crate at the root is traced",
        SPEC,
        "",
        &[
            ("src/lib.rs", &cited("RER-1")),
            ("tests/cli.rs", &cited("RER-2")),
        ],
        None,
    );
    report(
        "a test under the root src is internal, one under the root tests is not",
        boundary_lines(&stderr) == [warning("1 requirement", "RER-1")],
    );
    case(
        "a citation in a build script must sit above a test fn",
        SPEC,
        TESTS,
        &[("build.rs", "// spec: RER-1\nconst X: u8 = 1;\n")],
        Some("build.rs:1:"),
    );
    case(
        "a citation in an example is checked",
        SPEC,
        TESTS,
        &[("examples/e.rs", &cited("RER-9"))],
        Some("RER-9"),
    );
    case(
        "the build output beside a Cargo.toml is not read",
        SPEC,
        &cited("RER-1"),
        &[
            ("Cargo.toml", "[workspace]\n"),
            ("target/debug/build/x/out/t.rs", &cited("RER-2")),
        ],
        Some("RER-2"),
    );
    case(
        "node_modules is not read",
        SPEC,
        TESTS,
        &[("node_modules/x/t.rs", &cited("RER-9"))],
        None,
    );
    case(
        "the skipped_dirs of cardinal.toml are not read",
        SPEC,
        TESTS,
        &[
            (
                ".claude/cardinal.toml",
                "[project]\nskipped_dirs = [\"vendor\"]\n",
            ),
            ("vendor/a/tests/t.rs", &cited("RER-9")),
        ],
        None,
    );
    case(
        "a cardinal.toml that does not parse is a problem, not an empty skipped_dirs",
        SPEC,
        TESTS,
        &[(".claude/cardinal.toml", "[project\n")],
        Some(".claude/cardinal.toml"),
    );
}

// spec: TRC-11, TRC-12, TRC-15
/// The test files a project declares are read with the comment marker of their extension, the `.rs` files staying
/// the default.
fn any_language(case: &Case) {
    let config = |patterns: &str| {
        (
            ".claude/cardinal.toml",
            format!("[project]\nspec_files = [{patterns}]\n"),
        )
    };
    let py = config("\"test_*.py\"");
    case(
        "the spec_files are read instead of the .rs files",
        SPEC,
        "// spec: RER-1\n#[test]\nfn t() {}\n",
        &[
            (py.0, &py.1),
            (
                "tests/test_cli.py",
                "# spec: RER-1, RER-2\ndef test_x():\n    pass\n",
            ),
            (
                "tests/helper.py",
                "# spec: RER-9\ndef helper():\n    pass\n",
            ),
        ],
        None,
    );
    case(
        "a .rs file is not read when spec_files leaves it out",
        SPEC,
        "// spec: RER-1, RER-2\n#[test]\nfn t() {}\n",
        &[(py.0, &py.1)],
        Some("requirement RER-1: no test cites"),
    );
    let go = config("\"*_test.go\"");
    case(
        "a Go test beside the code is read with //",
        SPEC,
        "",
        &[
            (go.0, &go.1),
            (
                "pkg/a/x_test.go",
                "// spec: RER-1, RER-2\nfunc TestX(t *testing.T) {\n}\n",
            ),
        ],
        None,
    );
    let mixed = config("\"*.rs\", \"*.py\", \"*.foo\"");
    case(
        "a # line of a .rs file is no citation",
        SPEC,
        "// spec: RER-1\n#[test]\nfn t() {}\n",
        &[
            (mixed.0, &mixed.1),
            ("crates/a/src/u.rs", "# spec: RER-2\nfn u() {}\n"),
        ],
        Some("requirement RER-2: no test cites"),
    );
    case(
        "a // line of a .py file is no citation",
        SPEC,
        "",
        &[
            (mixed.0, &mixed.1),
            (
                "t/test_a.py",
                "# spec: RER-1, RER-2\ndef test_a():\n    pass\n// spec: RER-9\n",
            ),
        ],
        None,
    );
    case(
        "an unknown extension takes any of the four markers",
        SPEC,
        "",
        &[
            (mixed.0, &mixed.1),
            ("t/a.foo", "-- spec: RER-1\nx\n; spec: RER-2\ny\n"),
        ],
        None,
    );
    let bad = config("\"tests/*.py\"");
    case(
        "a spec_files pattern holding a / is a problem",
        SPEC,
        TESTS,
        &[(bad.0, &bad.1)],
        Some("`tests/*.py`"),
    );
}

// spec: TRC-11, TRC-12, TRC-16
/// The Lisp marker repeated, the literal characters of a pattern and the type of the keys.
fn any_language_forms(case: &Case) {
    case(
        "a ;; line of a Lisp file is a citation",
        SPEC,
        "",
        &[
            (
                ".claude/cardinal.toml",
                "[project]\nspec_files = [\"*.clj\"]\n",
            ),
            (
                "test/a.clj",
                ";; spec: RER-1, RER-2\n(deftest a (is true))\n",
            ),
        ],
        None,
    );
    case(
        "every character of a pattern but * stands for itself",
        SPEC,
        "",
        &[
            (
                ".claude/cardinal.toml",
                "[project]\nspec_files = [\"t[1]_?.py\"]\n",
            ),
            (
                "t/t[1]_?.py",
                "# spec: RER-1, RER-2\ndef test_a():\n    pass\n",
            ),
            ("t/t1_a.py", "# spec: RER-9\ndef test_a():\n    pass\n"),
        ],
        None,
    );
    for key in ["spec_files", "internal_dirs"] {
        case(
            "a key that is not a list of strings is a problem",
            SPEC,
            TESTS,
            &[(
                ".claude/cardinal.toml",
                &format!("[project]\n{key} = \"x\"\n"),
            )],
            Some(&format!("`{key}`")),
        );
    }
}
// spec: TRC-13
/// Outside a `.rs` file, a citation sits right above a code line, past comments and decorators.
fn any_language_placement(case: &Case) {
    let mixed = (
        ".claude/cardinal.toml",
        "[project]\nspec_files = [\"*.rs\", \"*.py\"]\n",
    );
    case(
        "comments and decorators may sit between a citation and its test",
        SPEC,
        "",
        &[
            mixed,
            (
                "t/test_a.py",
                "# spec: RER-1, RER-2\n# slow\n@pytest.mark.slow\ndef test_a():\n    pass\n",
            ),
        ],
        None,
    );
    case(
        "a citation on the last line of a .py file is a problem",
        SPEC,
        TESTS,
        &[
            mixed,
            ("t/test_a.py", "def test_a():\n    pass\n# spec: RER-1\n"),
        ],
        Some("t/test_a.py:3:"),
    );
    case(
        "a citation above a blank line of a .py file is a problem",
        SPEC,
        TESTS,
        &[
            mixed,
            ("t/test_a.py", "# spec: RER-1\n\ndef test_a():\n    pass\n"),
        ],
        Some("t/test_a.py:1:"),
    );
    any_language_forms(case);
}

// spec: TRC-9
/// Outside git, a symbolic link is not followed: neither a cycle nor a tree outside the root is read.
#[cfg(unix)]
#[test]
fn outside_git_a_symbolic_link_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let write = |root: &Path, path: &str, body: &str| {
        std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        std::fs::write(root.join(path), body).unwrap();
    };
    write(dir.path(), "docs/adr/0005-incremental.md", "x");
    write(dir.path(), "specs/rerun.md", SPEC);
    write(dir.path(), "crates/a/src/t.rs", TESTS);
    write(outside.path(), "x/bad.rs", "// spec: RER-9\nfn t() {}\n");
    std::os::unix::fs::symlink("..", dir.path().join("crates/up")).unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("ext")).unwrap();
    let out = tool_command("/tools/check_specs.rs")
        .arg(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && stdout.is_empty(), "{stdout}");
}

// spec: PAR-14, TRC-10
/// In a git repository the files come from git: a worktree nested in the scanned one is never read, ignored by
/// `info/exclude` or seen as an embedded repository, while an untracked test is still traced.
fn in_git(script: &Path, report: &dyn Fn(&str, bool)) {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&root);
    let write = |path: &str, body: &str| {
        std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        std::fs::write(root.join(path), body).unwrap();
    };
    write("docs/adr/0005-incremental.md", "x");
    write("specs/rerun.md", SPEC);
    write("crates/a/src/t.rs", "// spec: RER-1\nfn t() {}\n");
    write("crates/a/src/worktrees/w.rs", "fn w() {}\n");
    write(
        ".claude/cardinal.toml",
        "[project]\nskipped_dirs = [\"vendor\"]\n",
    );
    write("vendor/a/t.rs", "// spec: RER-9\nfn t() {}\n");
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&root)
            .output()
            .is_ok_and(|o| o.status.success());
        assert!(ok, "git {args:?} failed");
    };
    // a tracked link into a nested worktree is not followed
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../../y/crates/a/src/bad.rs",
        root.join("crates/a/src/link.rs"),
    )
    .unwrap();
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&["worktree", "add", "-q", "-b", "x", ".claude/worktrees/x"]);
    git(&["worktree", "add", "-q", "-b", "y", "crates/y"]);
    write("crates/a/src/untracked.rs", "// spec: RER-2\nfn t() {}\n");
    for nested in [".claude/worktrees/x", "crates/y"] {
        write(
            &format!("{nested}/crates/a/src/bad.rs"),
            "// spec: RER-9\nfn t() {}\n",
        );
    }
    let run = || {
        let out = tool_command(script).arg(&root).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        if !out.status.success() {
            println!("{stdout}");
        }
        out.status.success() && stdout.is_empty()
    };
    report("in git, the nested worktrees git does not ignore and the skipped_dirs are not read, an untracked test is traced", run());
    write(".git/info/exclude", "/.claude/worktrees/\n");
    report("in git, the nested worktree git ignores is not read", run());
    std::fs::write(root.join(".git/index"), "corrupt").unwrap();
    let out = tool_command(script).arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    report(
        "in git, a failing git ls-files fails the check without walking the disk",
        !out.status.success() && stdout.contains("git ls-files") && !stdout.contains("RER-9"),
    );
    let _ = std::fs::remove_dir_all(&root);
}

// spec: CLR-1, CLR-2, CLR-5
fn marker_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let marked = SPEC.replace("then c.", "then c. [NEEDS CLARIFICATION: which exit code?]");
    case(
        "a clarification marker in an accepted spec is flagged with its line",
        &marked,
        TESTS,
        &[],
        Some("specs/rerun.md:20:"),
    );
    let draft = marked.replace("status: accepted", "status: draft");
    let stderr = case(
        "a clarification marker in a draft spec passes",
        &draft,
        "",
        &[],
        None,
    );
    report(
        "a clarification marker in a draft spec is warned of with its question",
        stderr.contains("warning: specs/rerun.md:20: needs clarification: which exit code?"),
    );
    let spanned = SPEC.replace(
        "then c.",
        "then c. `[NEEDS CLARIFICATION: x]` and ``a ` [NEEDS CLARIFICATION: y]``",
    );
    case(
        "a clarification marker in a code span of an accepted spec passes",
        &spanned,
        TESTS,
        &[],
        None,
    );
    let stderr = case(
        "a clarification marker in a code span of a draft spec passes",
        &spanned.replace("status: accepted", "status: draft"),
        "",
        &[],
        None,
    );
    report(
        "a clarification marker in a code span is not warned of",
        !stderr.contains("needs clarification"),
    );
    let fenced = SPEC.replace(
        "- None.\n",
        "- None.\n\n```markdown\n[NEEDS CLARIFICATION: x]\n```\n",
    );
    case(
        "a clarification marker in a fenced block of an accepted spec passes",
        &fenced,
        TESTS,
        &[],
        None,
    );
    let nested = SPEC.replace(
        "- None.\n",
        "- None.\n\n````markdown\n```\n````\n\n[NEEDS CLARIFICATION: hidden?]\n",
    );
    case(
        "a marker after a fence holding a shorter fence is flagged",
        &nested,
        TESTS,
        &[],
        Some("NEEDS CLARIFICATION"),
    );
    let mixed = SPEC.replace(
        "- None.\n",
        "- None.\n\n~~~\n```\n~~~\n\n[NEEDS CLARIFICATION: hidden?]\n",
    );
    case(
        "a marker after a tilde fence holding a backtick fence is flagged",
        &mixed,
        TESTS,
        &[],
        Some("NEEDS CLARIFICATION"),
    );
    let bracketed = draft.replace("which exit code?", "keep the [ADR 5] limit?");
    let stderr = case(
        "a question with brackets passes in a draft",
        &bracketed,
        "",
        &[],
        None,
    );
    report(
        "a question with brackets is warned of in full",
        stderr.contains("needs clarification: keep the [ADR 5] limit?"),
    );
}

// spec: TRC-1, TRC-2, TRC-3, TRC-4, TRC-5, TRC-6, TRC-7, TRC-8
fn boundary_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let boundary = |name: &str, tests: &str, others: &[(&str, &str)]| {
        let stderr = case(name, SPEC, tests, others, None);
        boundary_lines(&stderr)
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    };
    let lines = boundary("internal proofs only pass", TESTS, &[]);
    report(
        "requirements proven only under src are warned of in one line",
        lines == [warning("2 requirements", "RER-1, RER-2")],
    );
    let lines = boundary(
        "a boundary proof passes",
        TESTS,
        &[("crates/a/tests/cli.rs", "// spec: RER-1\nfn t() {}\n")],
    );
    report(
        "a requirement also proven outside src is not listed",
        lines == [warning("1 requirement", "RER-2")],
    );
    let lines = boundary(
        "managed tests are boundary proofs",
        "",
        &[(
            ".claude/hooks/test_h.rs",
            "// spec: RER-1, RER-2\nfn t() {}\n",
        )],
    );
    report(
        "a spec proven outside src is not warned of",
        lines.is_empty(),
    );
    let only_rer1 = [warning("1 requirement", "RER-1")];
    let draft = SPEC.replace("status: accepted", "status: draft");
    let stderr = case(
        "an uncited draft requirement passes",
        &draft,
        "// spec: RER-1\nfn t() {}\n",
        &[],
        None,
    );
    report(
        "an uncited requirement is not listed",
        boundary_lines(&stderr) == only_rer1,
    );
    let withdrawn = SPEC.replace("A run must resume.", "withdrawn: resumes.");
    let stderr = case(
        "a withdrawn requirement cited under src passes",
        &withdrawn,
        TESTS,
        &[],
        None,
    );
    report(
        "a withdrawn requirement is not listed",
        boundary_lines(&stderr) == only_rer1,
    );
    let lines = boundary(
        "a managed test under a src directory passes",
        "",
        &[(
            ".claude/hooks/src/test_h.rs",
            "// spec: RER-1, RER-2\nfn t() {}\n",
        )],
    );
    report(
        "a managed test under a src directory is a boundary proof",
        lines.is_empty(),
    );
    reason_checks(case, report);
}

// spec: TRC-1, TRC-2, TRC-3, TRC-4, TRC-5, TRC-6, TRC-7, TRC-8
/// The reason an internal proof gives, `(internal: <reason>)`: it lifts the boundary warning, and must be well formed.
fn reason_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let boundary = |name: &str, tests: &str, others: &[(&str, &str)]| {
        let stderr = case(name, SPEC, tests, others, None);
        boundary_lines(&stderr)
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    };
    let reasoned = "// spec: RER-1, RER-2 (internal: the RER-9 fixture (git) is slow)\nfn t() {}\n";
    let lines = boundary("internal proofs with a reason pass", reasoned, &[]);
    report(
        "requirements whose internal proofs give a reason are not listed",
        lines.is_empty(),
    );
    let lines = boundary(
        "a reasoned and a bare internal proof pass",
        reasoned,
        &[("crates/a/src/u.rs", "// spec: RER-2\nfn u() {}\n")],
    );
    report(
        "a requirement with one internal proof giving no reason is listed",
        lines == [warning("1 requirement", "RER-2")],
    );
    for blank in ["(internal: )", "(internal:)"] {
        case(
            &format!("a blank reason `{blank}` is flagged"),
            SPEC,
            &format!("// spec: RER-1, RER-2 {blank}\nfn t() {{}}\n"),
            &[],
            Some("crates/a/src/t.rs:1: `(internal: <reason>)` gives no reason"),
        );
    }
    for (bad, line) in [
        (
            "RER-1 (internal: slow) RER-2",
            "RER-1, RER-2 (internal: slow) RER-2",
        ),
        (
            "RER-1, RER-2 (internal: slow",
            "RER-1, RER-2 (internal: slow",
        ),
        (
            "RER-1 (internal: slow) RER-2)",
            "RER-1, RER-2 (internal: slow) RER-2)",
        ),
        (
            "(internal: slow)",
            "RER-1, RER-2\nfn u() {}\n// spec: (internal: slow)",
        ),
    ] {
        case(
            &format!("a malformed reason `{bad}` is flagged"),
            SPEC,
            &format!("// spec: {line}\nfn t() {{}}\n"),
            &[],
            Some("`(internal: <reason>)` must follow the ids and end the line"),
        );
    }
}

// spec: TRC-14, TRC-15
/// The `internal_dirs` of `cardinal.toml` replace `src` as the directories of the internal tests.
fn internal_dirs_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let stderr = case(
        "internal_dirs replace src",
        SPEC,
        "// spec: RER-2\n#[test]\nfn t() {}\n",
        &[
            (
                ".claude/cardinal.toml",
                "[project]\ninternal_dirs = [\"lib\"]\n",
            ),
            ("lib/a.rs", "// spec: RER-1\n#[test]\nfn t() {}\n"),
        ],
        None,
    );
    case(
        "an internal_dirs entry holding a / is a problem",
        SPEC,
        TESTS,
        &[(
            ".claude/cardinal.toml",
            "[project]\ninternal_dirs = [\"tests/unit\"]\n",
        )],
        Some("`tests/unit`"),
    );
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("proven only by tests under"))
        .collect();
    report(
        "a test under an internal_dirs entry is internal, one under src is not",
        lines == [warning("1 requirement", "RER-1").replace("under `src`", "under `lib`")],
    );
}

/// The boundary warning of `specs/rerun.md` for `count` requirements, `ids`.
fn warning(count: &str, ids: &str) -> String {
    format!(
        "warning: specs/rerun.md: {count} proven only by tests under `src` with no reason: {ids} \
         (give it as `spec: ID (internal: <reason>)` after the comment marker)"
    )
}

/// The boundary warnings of a checker's stderr.
fn boundary_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|l| l.contains("only by tests under `src`"))
        .collect()
}

// spec: CLR-3, CLR-4, CLR-5, CLR-6, CLR-7
fn wording_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let draft = SPEC.replace("status: accepted", "status: draft");
    let stderr = case("a clear draft passes", &draft, "", &[], None);
    report(
        "a clear draft is not warned of",
        !stderr.contains("warning"),
    );
    let vague = draft.replace("A run must resume.", "A run Should resume fast, etc.");
    let stderr = case("a vague draft requirement passes", &vague, "", &[], None);
    for word in ["should", "fast", "etc."] {
        report(
            &format!("a vague word is warned of: {word}"),
            stderr.contains(&format!(
                "warning: specs/rerun.md: requirement RER-2: vague word `{word}`"
            )),
        );
    }
    report(
        "a requirement with no must is warned of",
        stderr.contains("warning: specs/rerun.md: requirement RER-2: holds 0 `must`, expected 1"),
    );
    let two = draft.replace("A run must resume.", "A run must resume and must not stop.");
    let stderr = case(
        "a draft requirement with two musts passes",
        &two,
        "",
        &[],
        None,
    );
    report(
        "a requirement with two musts is warned of",
        stderr.contains("requirement RER-2: holds 2 `must`, expected 1"),
    );
    let negated = draft.replace("A run must resume.", "A run must not stop.");
    let stderr = case(
        "a draft requirement with must not passes",
        &negated,
        "",
        &[],
        None,
    );
    report("must not counts as one must", !stderr.contains("warning"));
    let quiet = draft.replace(
        "A run must resume.",
        "A run must print `may_fail`, \"you may retry, must you\" and `must`.",
    );
    let stderr = case("words in code spans and quotes pass", &quiet, "", &[], None);
    report(
        "words in code spans and quotes are not warned of",
        !stderr.contains("warning"),
    );
    let inch = draft.replace(
        "A run must resume.",
        "A run must print a 12\" banner, then may stop.",
    );
    let stderr = case("an unbalanced quote passes", &inch, "", &[], None);
    report(
        "the text after an unbalanced quote is still checked",
        stderr.contains("vague word `may`") && !stderr.contains("`must`, expected 1"),
    );
    let contracted = draft.replace(
        "A run must resume.",
        "A run shouldn't stop and mustn't print.",
    );
    let stderr = case("contractions pass", &contracted, "", &[], None);
    report(
        "a contracted vague word is warned of",
        stderr.contains("vague word `should`"),
    );
    report(
        "mustn't counts as a must",
        !stderr.contains("`must`, expected 1"),
    );
    wording_scope_checks(case, report);
}

// spec: CLR-3, CLR-4, CLR-5, CLR-6, CLR-7
/// The requirements the wording is not checked in: with no kind, withdrawn, or of an accepted spec.
fn wording_scope_checks(case: &Case, report: &dyn Fn(&str, bool)) {
    let draft = SPEC.replace("status: accepted", "status: draft");
    let kindless = draft.replace("(deterministic) A run must resume.", "A run must resume.");
    let stderr = case(
        "a requirement with no kind is flagged",
        &kindless,
        "",
        &[],
        Some("kind"),
    );
    report(
        "a requirement with no kind gets no wording warning",
        !stderr.contains("`must`, expected 1"),
    );
    let withdrawn = draft.replace("A run must resume.", "withdrawn: A run may resume.");
    let stderr = case(
        "a withdrawn vague requirement passes",
        &withdrawn,
        "",
        &[],
        None,
    );
    report(
        "a withdrawn requirement is not warned of",
        !stderr.contains("warning"),
    );
    let accepted = SPEC.replace("A run must resume.", "A run may resume.");
    let stderr = case(
        "a vague accepted requirement passes",
        &accepted,
        TESTS,
        &[],
        None,
    );
    report(
        "an accepted spec gets no wording warning",
        !stderr
            .lines()
            .any(|l| l.contains("warning") && !boundary_lines(&stderr).contains(&l)),
    );
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

/// In git the files come sorted, managed tests first: a crate test citing an unknown id is still flagged, and so is
/// every other file citing it.
#[test]
fn an_unknown_id_is_flagged_in_every_crate_file() {
    let root = tempfile::tempdir().unwrap();
    let write = |path: &str, body: &str| {
        std::fs::create_dir_all(root.path().join(path).parent().unwrap()).unwrap();
        std::fs::write(root.path().join(path), body).unwrap();
    };
    write("docs/adr/0005-incremental.md", "x");
    write("specs/rerun.md", SPEC);
    write("crates/a/tests/t.rs", TESTS);
    let cite = "// spec: PAR-1, RER-9\nfn t() {}\n";
    write(".claude/hooks/h.rs", cite);
    write("crates/a/tests/x.rs", cite);
    write("crates/b/tests/y.rs", cite);
    let ok = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(root.path())
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git init failed");
    let out = tool_command("check_specs.rs")
        .arg(root.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "{stdout}");
    for line in [
        "crates/a/tests/x.rs: cites `PAR-1`",
        "crates/b/tests/y.rs: cites `PAR-1`",
        "crates/a/tests/x.rs: cites `RER-9`",
        "crates/b/tests/y.rs: cites `RER-9`",
        ".claude/hooks/h.rs: cites `RER-9`",
    ] {
        assert!(stdout.contains(line), "missing {line:?} in:\n{stdout}");
    }
    assert!(
        !stdout.contains(".claude/hooks/h.rs: cites `PAR-1`"),
        "{stdout}"
    );
}
