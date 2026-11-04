//! The tools of `cardinal tool <name>`, run as a hook or a recipe runs them: the registry, `commit-check` and
//! `check-commits`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::{tempdir, TempDir};

/// Exit code, stdout and stderr of `cardinal tool <args>` run in `cwd` with `stdin`.
fn tool(cwd: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .arg("tool")
        .args(args)
        .current_dir(cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// A git repository whose `.claude/cardinal.toml` is `toml` (none when `None`).
fn project(toml: Option<&str>) -> TempDir {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    if let Some(toml) = toml {
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(dir.path().join(".claude/cardinal.toml"), toml).unwrap();
    }
    dir
}

// spec: BIN-7
#[test]
fn no_tool_or_an_unknown_one_exits_3_listing_the_tools() {
    let dir = tempdir().unwrap();
    for args in [&[][..], &["no-such-tool"][..]] {
        let (code, stdout, stderr) = tool(dir.path(), args, "");
        assert_eq!(code, 3, "{args:?}");
        assert_eq!(stdout, "");
        assert!(
            stderr.contains("commit-check") && stderr.contains("check-commits"),
            "{stderr}"
        );
        assert!(stderr.starts_with("cardinal: "), "{stderr}");
    }
}

#[test]
fn commit_check_enforces_each_rule_of_agents_md() {
    let repo = project(Some("[project]\ncommit_scopes = [\"sync\"]\n"));
    let long_line = "word ".repeat(16);
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "valid subject only",
            "fix(sync): retry on 429 with backoff\n".into(),
            "",
        ),
        (
            "valid without scope",
            "fix: handle empty repository\n".into(),
            "",
        ),
        (
            "built-in scope",
            "docs(cardinal): describe the hooks\n".into(),
            "",
        ),
        (
            "scope of before the rename",
            "docs(harness): describe the hooks\n".into(),
            "scope",
        ),
        (
            "body and footer",
            "feat(sync): add progress\n\nLog one line per unit.\n\nRefs: #12\n".into(),
            "",
        ),
        (
            "Claude Code as the subject matter",
            "docs: describe the Claude Code hooks\n\nThe Anthropic CLI runs them.\n".into(),
            "",
        ),
        (
            "subject of exactly 72 chars",
            format!("feat(docs): {}\n", "a".repeat(72 - 12)),
            "",
        ),
        (
            "subject too long",
            format!("feat(docs): {}\n", "a".repeat(73 - 12)),
            "72",
        ),
        ("unknown type", "feature(docs): add flag\n".into(), "type"),
        (
            "unknown scope",
            "feat(frontend): add flag\n".into(),
            "scope",
        ),
        (
            "capitalized description",
            "feat(docs): Add flag\n".into(),
            "lowercase",
        ),
        (
            "trailing period",
            "feat(docs): add flag.\n".into(),
            "period",
        ),
        (
            "missing colon format",
            "feat(cli) add flag\n".into(),
            "format",
        ),
        (
            "no blank line before body",
            "feat(docs): add flag\nbody here\n".into(),
            "blank",
        ),
        (
            "body line too long",
            format!("feat(docs): add flag\n\n{long_line}\n"),
            "72",
        ),
        (
            "co-authored-by trailer",
            "feat(docs): add flag\n\nCo-Authored-By: Someone <a@b.c>\n".into(),
            "attribution",
        ),
        (
            "generated-with line",
            "feat(docs): add flag\n\nGenerated with Claude Code\n".into(),
            "attribution",
        ),
        (
            "markdown heading",
            "feat(docs): add flag\n\n# Why\n".into(),
            "markdown",
        ),
        (
            "code fence",
            "feat(docs): add flag\n\n```\nx\n```\n".into(),
            "markdown",
        ),
        ("emoji", "feat(docs): add flag \u{1F680}\n".into(), "emoji"),
    ];
    check_messages(repo.path(), cases);
}

// spec: VRD-1, VRD-2, VRD-13
#[test]
fn commit_check_refuses_an_expected_effect_it_cannot_read() {
    let repo = project(Some("[project]\ncommit_scopes = [\"files\"]\n"));
    let message = |line: &str| format!("feat(files): trim the skill\n\n{line}\n");
    let cases: Vec<(&str, String, &str)> = vec![
        ("median metric", message("Expected-effect: turns down"), ""),
        (
            "cache read share",
            message("Expected-effect: cache_read_share up"),
            "",
        ),
        (
            "main context peak",
            message("Expected-effect: main_context_peak down"),
            "",
        ),
        (
            "advisor metrics",
            message("Expected-effect: advisor_calls up\nExpected-effect: advisor_input_tokens down\nExpected-effect: advisor_output_tokens down"),
            "",
        ),
        (
            "quality ratio",
            message("Expected-effect: findings_per_review down"),
            "",
        ),
        (
            "docs ratio",
            message("Expected-effect: doc_lines_per_spec_line up"),
            "",
        ),
        (
            "tokens per issue",
            message("Expected-effect: input_tokens_per_issue down\nExpected-effect: output_tokens_per_issue down"),
            "",
        ),
        (
            "fixed context",
            message("Expected-effect: total_bytes up"),
            "",
        ),
        (
            "rule share",
            message("Expected-effect: breached_share:commit-attribution down"),
            "",
        ),
        (
            "waste bytes",
            message("Expected-effect: bytes_per_session:oversized-result down"),
            "",
        ),
        (
            "waste bytes with no rule",
            message("Expected-effect: bytes_per_session: down"),
            "Expected-effect",
        ),
        (
            "waste bytes of a rule that is no waste",
            message("Expected-effect: bytes_per_session:no-verify down"),
            "Expected-effect",
        ),
        (
            "unknown metric",
            message("Expected-effect: speed up"),
            "Expected-effect",
        ),
        (
            "unknown direction",
            message("Expected-effect: turns lower"),
            "line 3",
        ),
        (
            "rule with no name",
            message("Expected-effect: breached_share: down"),
            "Expected-effect",
        ),
        (
            "extra word",
            message("Expected-effect: turns down a lot"),
            "Expected-effect",
        ),
    ];
    check_messages(repo.path(), cases);
}

#[test]
fn commit_check_passes_the_trailer_of_who_validated() {
    let repo = project(Some("[project]\ncommit_scopes = [\"sync\"]\n"));
    let message = "feat(sync): add progress\n\nValidated-by: Jane Doe <jane@example.com>\n";
    check_messages(
        repo.path(),
        vec![("Validated-by trailer", message.into(), "")],
    );
}

/// Each case: a name, a message, and the hint its refusal must hold (empty: the message is valid).
fn check_messages(repo: &Path, cases: Vec<(&str, String, &str)>) {
    for (name, message, hint) in cases {
        let (code, stdout, _) = tool(repo, &["commit-check"], &message);
        if hint.is_empty() {
            assert_eq!((code, stdout.as_str()), (0, ""), "{name}");
        } else {
            assert_eq!(code, 1, "{name}: {stdout}");
            assert!(stdout.contains(hint), "{name}: {stdout}");
        }
    }
}

#[test]
fn commit_check_reads_the_scopes_of_the_repository_root() {
    let lookalike = project(Some(
        "[project]\ncommit_scopes_extra = [\"wrong\"]\ncommit_scopes = [\"right\"]\n",
    ));
    let commented = project(Some(
        "[project]\ncommit_scopes = [\n  \"a\", # a \"quote\" and a ]\n  \"b\",\n]\n",
    ));
    let nested = project(Some("[project]\ncommit_scopes = [\"deep\"]\n"));
    fs::create_dir_all(nested.path().join("a/b")).unwrap();
    let cases = [
        (lookalike.path().to_path_buf(), "feat(right): add flag\n", 0),
        (lookalike.path().to_path_buf(), "feat(wrong): add flag\n", 1),
        (commented.path().to_path_buf(), "feat(b): add flag\n", 0),
        (nested.path().join("a/b"), "feat(deep): add flag\n", 0),
    ];
    for (cwd, message, expected) in cases {
        let (code, stdout, stderr) = tool(&cwd, &["commit-check"], message);
        assert_eq!(code, expected, "{message}: {stdout}{stderr}");
    }
}

#[test]
fn commit_check_exits_2_when_the_configuration_cannot_be_read() {
    let missing = project(None);
    let invalid = project(Some("[project\ncommit_scopes = \n"));
    let not_a_repo = tempdir().unwrap();
    for (dir, hint) in [
        (missing.path(), "cannot read"),
        (invalid.path(), "cardinal.toml"),
        (not_a_repo.path(), "git"),
    ] {
        let (code, _, stderr) = tool(dir, &["commit-check"], "feat(docs): add flag\n");
        assert_eq!(code, 2, "{stderr}");
        assert!(
            stderr.starts_with("error: ") && stderr.contains(hint),
            "{stderr}"
        );
    }
}

// spec: FEA-8
#[test]
fn commit_checks_accept_an_attribution_when_the_project_turns_off_the_ban() {
    let repo = project(Some(
        "[cardinal]\nsource = \"x\"\ndisabled = [\"no-ai-attribution\"]\n\n[project]\ncommit_scopes = []\n",
    ));
    let message = "feat: add flag\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n";
    let (code, stdout, stderr) = tool(repo.path(), &["commit-check"], message);
    assert_eq!((code, stdout.as_str()), (0, ""), "{stderr}");
    let (_, stdout, _) = tool(
        repo.path(),
        &["commit-check"],
        "Feat: Big.\n\nCo-Authored-By: x\n",
    );
    assert!(
        stdout.contains("unknown type") && !stdout.contains("attribution"),
        "{stdout}"
    );
    git(
        repo.path(),
        &["commit", "-q", "--allow-empty", "-m", "chore: start"],
    );
    git(repo.path(), &["tag", "base"]);
    git(
        repo.path(),
        &["commit", "-q", "--allow-empty", "-m", message],
    );
    let (code, stdout, _) = tool(repo.path(), &["check-commits", "base", "HEAD"], "");
    assert_eq!(code, 0, "{stdout}");
}

// spec: FEA-21
#[test]
fn commit_checks_leave_the_format_to_a_project_that_turns_off_the_convention() {
    // no `commit_scopes`: the convention off reads none
    let repo = project(Some(
        "[cardinal]\nsource = \"x\"\ndisabled = [\"conventional-commits\"]\n",
    ));
    let message = format!("Added the parser.\n{}\n# notes\n", "x".repeat(90));
    let (code, stdout, stderr) = tool(repo.path(), &["commit-check"], &message);
    assert_eq!((code, stdout.as_str()), (0, ""), "{stderr}");
    let (code, stdout, _) = tool(
        repo.path(),
        &["commit-check"],
        "Added the parser.\n\nCo-Authored-By: x\nExpected-effect: nothing sideways\n",
    );
    assert_eq!(code, 1);
    assert_eq!(stdout.lines().count(), 2, "{stdout}");
    assert!(
        stdout.contains("line 3: no Co-Authored-By")
            && stdout.contains("line 4: write `Expected-effect"),
        "{stdout}"
    );
    git(
        repo.path(),
        &["commit", "-q", "--allow-empty", "-m", "start"],
    );
    git(repo.path(), &["tag", "base"]);
    git(
        repo.path(),
        &["commit", "-q", "--allow-empty", "-m", "Added the parser."],
    );
    let (code, stdout, _) = tool(repo.path(), &["check-commits", "base", "HEAD"], "");
    assert_eq!(code, 0, "{stdout}");
}

#[test]
fn check_commits_validates_every_non_merge_commit_of_the_range() {
    let repo = project(Some("[project]\ncommit_scopes = []\n"));
    let commit = |message: &str| {
        git(
            repo.path(),
            &["commit", "-q", "--allow-empty", "-m", message],
        );
    };
    commit("chore: start");
    git(repo.path(), &["tag", "base"]);
    commit("feat: add a thing");
    let (code, stdout, _) = tool(repo.path(), &["check-commits", "base", "HEAD"], "");
    assert_eq!(code, 0, "{stdout}");

    commit("Bad subject.");
    let (code, stdout, _) = tool(repo.path(), &["check-commits", "base", "HEAD"], "");
    assert_eq!(code, 1);
    assert_eq!(stdout.matches("== ").count(), 2, "{stdout}");
    assert!(stdout.contains("format must be"), "{stdout}");

    let (code, _, stderr) = tool(repo.path(), &["check-commits", "base"], "");
    assert_eq!(code, 2);
    assert!(stderr.contains("usage"), "{stderr}");
}

// spec: BIN-14
#[test]
fn run_exits_3_when_it_cannot_get_the_tool() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["run", "commit-check"])
        .current_dir(dir.path())
        .env_remove("CLAUDE_PROJECT_DIR")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cardinal.toml"), "{stderr}");
}

// spec: BIN-17
#[test]
fn the_launcher_exits_3_when_it_cannot_parse_its_arguments() {
    let dir = tempdir().unwrap();
    let bin = Path::new(env!("CARGO_BIN_EXE_cardinal"));
    let launch = |args: &[&str]| {
        Command::new(bin)
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap()
    };
    for args in [
        &["newcmd"][..],
        &["check", "--no-such-flag"],
        &["init"],
        &[],
    ] {
        let out = launch(args);
        assert_eq!(out.status.code(), Some(3), "{bin:?} {args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("Usage:"), "{bin:?} {args:?}: {stderr}");
    }
    for args in [
        &["--help"][..],
        &["--version"],
        &["help", "check"],
        &["help", "run"],
        &["help", "update"],
    ] {
        let out = launch(args);
        assert_eq!(out.status.code(), Some(0), "{bin:?} {args:?}");
        assert!(!out.stdout.is_empty(), "{bin:?} {args:?}: nothing printed");
    }
}

/// Every file of `dir`, recursively.
fn files_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

// spec: BIN-8
#[test]
fn every_tool_the_shared_harness_runs_exists() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = files_under(&repo.join("files"));
    sources.push(repo.join("entries/.claude/settings.json"));
    let tools: Vec<&str> = cardinal::tools::TOOLS.iter().map(|t| t.name).collect();
    let mut runs = 0;
    for path in sources {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (at, _) in text.match_indices("cardinal run ") {
            let rest = &text[at + "cardinal run ".len()..];
            let name = rest.split_whitespace().next().unwrap_or("");
            let name = name.trim_end_matches(['"', '`', ')', ',', ';']);
            // a placeholder, or the wildcard of an allow rule
            if name.is_empty() || name.starts_with('<') || name == "*" {
                continue;
            }
            runs += 1;
            assert!(
                tools.contains(&name),
                "{} runs the unknown tool `{name}`",
                path.display()
            );
        }
    }
    assert!(runs > 0, "no `cardinal run` found: the scan is broken");
}

// spec: BIN-8
#[test]
fn nothing_the_source_ships_runs_rust_script_or_the_name_before_the_rename() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = ["files", "entries", "seed"]
        .iter()
        .flat_map(|dir| files_under(&repo.join(dir)));
    for path in shipped {
        let text = fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !text.contains("rust-script"),
            "{} still names rust-script: run the tool through `cardinal run`",
            path.display()
        );
        assert!(
            !text.contains("harness run"),
            "{} still runs `harness run`: run the tool through `cardinal run`",
            path.display()
        );
    }
}
