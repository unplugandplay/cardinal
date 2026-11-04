//! Behavior test for `cardinal tool audit-sources`: counts, typed-message filter, instruction files and memory.

/// The project `repo` under `tmp`, its instruction file, memory and transcripts; returns the project directory.
fn fixture(tmp: &std::path::Path) -> std::path::PathBuf {
    let repo = tmp.join("repo");
    let proj = tmp.join("projects/-tmp-repo");
    std::fs::create_dir_all(proj.join("s1/subagents")).unwrap();
    std::fs::create_dir_all(proj.join("memory")).unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("CLAUDE.md"), "# rules\n").unwrap();
    std::fs::write(proj.join("memory/feedback_x.md"), "memo\n").unwrap();
    std::fs::write(proj.join("memory/MEMORY.md"), "index\n").unwrap();
    let cwd = repo.display().to_string();
    let line = |t: &str, extra: &str, content: &str| {
        format!(
            "{{\"type\":\"{t}\",\"cwd\":{cwd:?}{extra},\"message\":{{\"content\":{content}}}}}\n"
        )
    };
    let transcript = [
        line("user", "", "\"never commit without asking\""),
        line(
            "user",
            "",
            "[{\"type\":\"text\",\"text\":\"always run\\nthe tests\"}]",
        ),
        line(
            "user",
            "",
            "[{\"type\":\"tool_result\",\"content\":\"tool output\"}]",
        ),
        line("user", ",\"isMeta\":true", "\"meta text\""),
        line("user", "", "\"<system-reminder>x</system-reminder>\""),
        line("user", "", "\"<div> is broken\\tat\\\\n\""),
        line("user", ",\"isSidechain\":true", "\"sidechain text\""),
        line(
            "user",
            "",
            "\"This session is being continued from a previous conversation\"",
        ),
        line("assistant", "", "\"I answer\""),
    ]
    .concat();
    std::fs::write(proj.join("s1.jsonl"), transcript).unwrap();
    std::fs::write(
        proj.join("s1/subagents/a.jsonl"),
        line("user", "", "\"subagent prompt\""),
    )
    .unwrap();
    repo
}

#[test]
fn audit_sources() {
    let dir = std::path::PathBuf::from("/tools");
    let tmp = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&tmp);
    let repo = fixture(&tmp);

    let run = |args: &[&str]| {
        let out = tool_command(dir.join("audit_sources.rs"))
            .arg(tmp.join("projects"))
            .args(args)
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
    let (ok, summary) = run(&[]);
    check(
        "summary counts 1 session and 3 typed messages",
        ok && summary.contains("-tmp-repo: 1 sessions, 3 messages"),
    );
    check(
        "summary lists the instruction file of the project",
        summary.contains(&format!("  instructions: {}/CLAUDE.md", repo.display())),
    );
    check(
        "summary lists the memory files",
        summary.contains("memory: 1 files"),
    );
    let (_, messages) = run(&["--messages"]);
    check(
        "typed messages are printed, newlines escaped",
        messages.contains("never commit without asking")
            && messages.contains("always run\\nthe tests"),
    );
    check(
        "a typed message starting with '<' is kept, tab and backslash escaped",
        messages.contains("<div> is broken\\tat\\\\n"),
    );
    check(
        "tool results, meta, reminders, assistant and subagents are skipped",
        [
            "tool output",
            "meta text",
            "system-reminder",
            "I answer",
            "subagent prompt",
            "sidechain text",
            "being continued",
        ]
        .iter()
        .all(|s| !messages.contains(s)),
    );

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed, "a check failed: see the FAIL lines");
}

// spec: AUD-1
#[test]
fn audit_sources_lists_every_instruction_file_once() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let proj = tmp.path().join("projects/-tmp-repo");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(repo.join(".claude/rules/sub")).unwrap();
    std::fs::create_dir_all(repo.join("docs")).unwrap();
    let write = |path: &str, text: &str| std::fs::write(repo.join(path), text).unwrap();
    write("AGENTS.md", "# agents\n");
    std::os::unix::fs::symlink("AGENTS.md", repo.join("CLAUDE.md")).unwrap();
    // a linked memory file imports from the directory of the file it points to
    write("docs/main.md", "see @x.md and @~/notes.md\n");
    std::os::unix::fs::symlink("../docs/main.md", repo.join(".claude/CLAUDE.md")).unwrap();
    write("docs/x.md", "imported\n");
    write("docs/y.md", "not imported\n");
    write(".claude/rules/a.md", "always\n");
    // personal and kept out of git, loaded at launch with its imports
    write("CLAUDE.local.md", "mine @docs/local.md\n");
    write("docs/local.md", "imported by the local file\n");
    write(
        ".claude/rules/sub/b.md",
        "---\npaths: [\"src/**\"]\n---\nscoped\n",
    );
    write(".claude/rules/c.txt", "not a rule\n");
    let shared = tmp.path().join("shared.md");
    std::fs::write(&shared, "shared rule\n").unwrap();
    std::os::unix::fs::symlink(&shared, repo.join(".claude/rules/linked.md")).unwrap();
    let cwd = repo.display().to_string();
    std::fs::write(
        proj.join("s1.jsonl"),
        format!("{{\"type\":\"user\",\"cwd\":{cwd:?},\"message\":{{\"content\":\"hi\"}}}}\n"),
    )
    .unwrap();

    let out = tool_command("audit_sources.rs")
        .arg(tmp.path().join("projects"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let listed: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("  instructions: "))
        .map(|l| l.strip_prefix(&format!("{cwd}/")).unwrap_or(l).to_string())
        .collect();
    assert_eq!(
        listed,
        [
            ".claude/rules/a.md".into(),
            ".claude/rules/sub/b.md".into(),
            "AGENTS.md".into(),
            "CLAUDE.local.md".into(),
            "docs/local.md".into(),
            "docs/main.md".into(),
            "docs/x.md".into(),
            std::fs::canonicalize(&shared)
                .unwrap()
                .display()
                .to_string(),
        ]
    );
}

// spec: AUD-1
#[test]
fn audit_sources_lists_the_ancestors_up_to_the_repository_root() {
    let tmp = tempfile::tempdir().unwrap();
    let outer = tmp.path().join("x");
    let repo = outer.join("r");
    let cwd = repo.join("a/b");
    let proj = tmp.path().join("projects/-x-r-a-b");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    std::fs::create_dir_all(repo.join(".claude")).unwrap();
    std::fs::create_dir_all(repo.join("a/.claude/rules")).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let write = |path: &std::path::Path, text: &str| std::fs::write(path, text).unwrap();
    write(&outer.join("CLAUDE.md"), "above the repository\n");
    write(&repo.join("CLAUDE.md"), "repository\n");
    // the home directory: its `.claude/` holds the user-level files
    write(&repo.join(".claude/CLAUDE.md"), "user level\n");
    write(&repo.join("a/CLAUDE.md"), "ancestor\n");
    write(&repo.join("a/.claude/rules/r.md"), "ancestor rule\n");
    // an import above the working directory, inside the repository
    write(
        &cwd.join("CLAUDE.md"),
        "working directory @../../docs/up.md\n",
    );
    std::fs::create_dir_all(repo.join("docs")).unwrap();
    write(&repo.join("docs/up.md"), "imported from below\n");
    // a project rule linked into the home directory's `.claude/` is the project's
    std::fs::create_dir_all(repo.join(".claude/shared")).unwrap();
    write(&repo.join(".claude/shared/s.md"), "shared\n");
    std::os::unix::fs::symlink(
        "../../../.claude/shared/s.md",
        repo.join("a/.claude/rules/s.md"),
    )
    .unwrap();
    let cwd_text = cwd.display().to_string();
    std::fs::write(
        proj.join("s1.jsonl"),
        format!("{{\"type\":\"user\",\"cwd\":{cwd_text:?},\"message\":{{\"content\":\"hi\"}}}}\n"),
    )
    .unwrap();

    let listed = |cmd: &mut std::process::Command| -> Vec<String> {
        let out = cmd
            .arg(tmp.path().join("projects"))
            .env("HOME", &repo)
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.strip_prefix("  instructions: "))
            .map(String::from)
            .collect()
    };
    let under = |root: &std::path::Path, path: &str| root.join(path).display().to_string();
    // another configuration directory: the `.claude/` of the home directory is a project's
    let other = listed(tool_command("audit_sources.rs").env("CLAUDE_CONFIG_DIR", tmp.path()));
    assert!(other.contains(&under(&repo, ".claude/CLAUDE.md")));
    assert_eq!(
        listed(tool_command("audit_sources.rs").env_remove("CLAUDE_CONFIG_DIR")),
        [
            under(&repo, ".claude/shared/s.md"),
            under(&repo, "CLAUDE.md"),
            under(&repo, "a/.claude/rules/r.md"),
            under(&repo, "a/CLAUDE.md"),
            under(&cwd, "CLAUDE.md"),
            under(&repo, "docs/up.md"),
        ]
    );
}

// spec: AUD-2
#[test]
fn audit_sources_lists_the_user_level_files_before_the_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path().join("config");
    let proj = tmp.path().join("projects/-tmp-repo");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(config.join("rules/sub")).unwrap();
    let write = |path: &str, text: &str| std::fs::write(config.join(path), text).unwrap();
    write("CLAUDE.md", "see @notes.md and @~/elsewhere.md\n");
    write("notes.md", "imported\n");
    write("unread.md", "not imported\n");
    write("rules/a.md", "always\n");
    write("rules/sub/b.md", "---\npaths: [\"src/**\"]\n---\nscoped\n");
    write("rules/c.txt", "not a rule\n");
    let cwd = tmp.path().join("repo").display().to_string();
    std::fs::write(
        proj.join("s1.jsonl"),
        format!("{{\"type\":\"user\",\"cwd\":{cwd:?},\"message\":{{\"content\":\"hi\"}}}}\n"),
    )
    .unwrap();
    let user_level = |cmd: &mut std::process::Command| {
        let out = cmd.arg(tmp.path().join("projects")).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let first: Vec<String> = stdout
            .lines()
            .map_while(|l| l.strip_prefix("instructions: "))
            .map(String::from)
            .collect();
        assert!(stdout
            .lines()
            .nth(first.len())
            .unwrap()
            .starts_with("-tmp-repo:"));
        first
    };
    let expected = |dir: &std::path::Path| {
        ["CLAUDE.md", "notes.md", "rules/a.md", "rules/sub/b.md"]
            .map(|p| dir.join(p).display().to_string())
    };

    // a relative configuration directory is printed absolute
    let set = user_level(
        tool_command("audit_sources.rs")
            .current_dir(tmp.path())
            .env("CLAUDE_CONFIG_DIR", "config")
            .env("HOME", tmp.path().join("nowhere")),
    );
    assert_eq!(set, expected(&config));

    let home_config = tmp.path().join(".claude");
    std::fs::rename(&config, &home_config).unwrap();
    let unset = user_level(
        tool_command("audit_sources.rs")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env("HOME", tmp.path()),
    );
    assert_eq!(unset, expected(&home_config));
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
    // no user-level file of the machine running the tests
    cmd.arg("tool")
        .arg(stem)
        .env("CLAUDE_CONFIG_DIR", "/nonexistent-claude-config");
    cmd
}
