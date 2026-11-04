//! Behavior test for `cardinal tool review-gate`: runs `record`, `check` and `native` against throwaway git
//! repositories. `hook` is the `harness` binary.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Exit code and stderr of the hook in `mode` for `event`, with `project` as the project directory.
fn run(hook: &Path, mode: &str, project: &Path, event: &serde_json::Value) -> (i32, String) {
    let mut child = Command::new(hook)
        .args(["tool", "review-gate", mode])
        .env("CLAUDE_PROJECT_DIR", project)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn review_done(hook: &Path, project: &Path, skill: &str) {
    let event = serde_json::json!({
        "tool_name": "Skill", "cwd": project, "tool_input": {"skill": skill}, "tool_response": {"success": true}
    });
    assert_eq!(run(hook, "record", project, &event).0, 0);
}

fn commit_attempt(hook: &Path, project: &Path, command: &str) -> (i32, String) {
    let event = serde_json::json!({"tool_name": "Bash", "cwd": project, "tool_input": {"command": command}});
    run(hook, "check", project, &event)
}

fn sh(project: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

// spec: PAR-9
/// A review recorded in one worktree must not cover another: the marker and the waiver live in the git directory
/// of each worktree.
fn git_dir_of(dir: &Path) -> std::path::PathBuf {
    let out = Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .current_dir(dir)
        .output()
        .unwrap();
    Path::new(String::from_utf8_lossy(&out.stdout).trim()).to_path_buf()
}

// spec: REV-15
fn worktree_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let main = root.join("main");
    let linked = root.join("linked");
    fs::create_dir_all(&main).unwrap();
    sh(&main, &["init", "-q", "."]);
    fs::write(main.join("a.txt"), "one\n").unwrap();
    sh(&main, &["add", "-A"]);
    sh(&main, &["commit", "-qm", "init"]);
    sh(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            linked.to_str().unwrap(),
        ],
    );
    fs::write(main.join("a.txt"), "two\n").unwrap();
    fs::write(linked.join("a.txt"), "two\n").unwrap();

    review_done(hook, &main, "code-review");
    let mut out = vec![
        (
            "a review in the main worktree covers it",
            native_run(hook, &main, &[]).0 == 0,
        ),
        (
            "the same diff in a linked worktree is still unreviewed",
            native_run(hook, &linked, &[]).0 == 2,
        ),
    ];
    review_done(hook, &linked, "code-review");
    out.push((
        "a review in a linked worktree covers it",
        native_run(hook, &linked, &[]).0 == 0,
    ));

    // the path the instructions give the user, `git rev-parse --git-path`, is the one the hook reads from a linked worktree
    let name = ["claude-review", "waived"].join("-");
    let given = Command::new("git")
        .args(["rev-parse", "--git-path", &name])
        .current_dir(&linked)
        .output()
        .unwrap();
    let given = linked.join(String::from_utf8_lossy(&given.stdout).trim());
    out.push((
        "the given path is not the waiver of the main worktree",
        given != git_dir_of(&main).join(&name),
    ));
    let waiver = git_dir_of(&linked).join("claude-review-waived");
    out.push((
        "the given path is the waiver of the linked worktree",
        given.parent().unwrap().canonicalize().unwrap()
            == waiver.parent().unwrap().canonicalize().unwrap(),
    ));
    fs::write(&given, "").unwrap();
    out.push((
        "a waiver of the linked worktree lets its commit through",
        native_run(hook, &linked, &[]).0 == 0,
    ));
    out.push(("the waiver stays while no commit happened", waiver.exists()));
    sh(&linked, &["commit", "-qam", "x"]);
    fs::write(linked.join("new.txt"), "x\n").unwrap();
    let main_waiver = git_dir_of(&main).join("claude-review-waived");
    fs::write(&main_waiver, "").unwrap();
    out.push((
        "the waiver of the main worktree does not cover the linked one",
        native_run(hook, &linked, &[]).0 == 2,
    ));
    out.push((
        "the commit made consumed the waiver of the linked worktree",
        !waiver.exists(),
    ));
    out.push((
        "the linked worktree leaves the main waiver alone",
        main_waiver.exists(),
    ));
    let _ = fs::remove_dir_all(&root);
    out
}

// spec: REV-17
/// `just waive-review` creates the waiver of the worktree it runs in, prints its path and changes nothing else.
fn recipe_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let recipes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../files/cardinal-review.just");
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let (main, linked) = (root.join("main"), root.join("linked"));
    fs::create_dir_all(main.join("sub")).unwrap();
    sh(&main, &["init", "-q", "."]);
    fs::write(main.join("a.txt"), "one\n").unwrap();
    fs::copy(&recipes, main.join("justfile")).unwrap();
    sh(&main, &["add", "-A"]);
    sh(&main, &["commit", "-qm", "init"]);
    sh(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            linked.to_str().unwrap(),
        ],
    );
    let name = ["claude-review", "waived"].join("-");
    let status = |dir: &Path| {
        String::from_utf8_lossy(
            &Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(dir)
                .output()
                .unwrap()
                .stdout,
        )
        .to_string()
    };
    let recipe = |dir: &Path| {
        Command::new("just")
            .arg("waive-review")
            .current_dir(dir)
            .output()
            .unwrap()
    };
    let mut out = Vec::new();

    let before = status(&main);
    let run = recipe(&main.join("sub"));
    let (main_waiver, linked_waiver) = (
        git_dir_of(&main).join(&name),
        git_dir_of(&linked).join(&name),
    );
    out.push(("the recipe runs", run.status.success()));
    out.push((
        "it creates the waiver of the main worktree, from a subdirectory",
        main_waiver.exists(),
    ));
    out.push((
        "it prints that path",
        String::from_utf8_lossy(&run.stdout).contains(main_waiver.to_str().unwrap()),
    ));
    out.push((
        "it changes nothing else in the repository",
        status(&main) == before,
    ));
    out.push((
        "it leaves the linked worktree alone",
        !linked_waiver.exists(),
    ));

    let run = recipe(&linked);
    out.push((
        "it creates the waiver of a linked worktree",
        run.status.success() && linked_waiver.exists(),
    ));
    out.push((
        "it prints the path of the linked worktree",
        String::from_utf8_lossy(&run.stdout).contains(linked_waiver.to_str().unwrap()),
    ));
    fs::write(linked.join("a.txt"), "two\n").unwrap();
    out.push((
        "the hook honours that waiver in the linked worktree",
        native_run(hook, &linked, &[]).0 == 0,
    ));
    let _ = fs::remove_dir_all(&root);
    out
}

/// Two repositories side by side: `current` reviewed and unchanged, `other` with a change nobody reviewed.
/// Returns the shell directory (`current`).
fn target_fixture(hook: &Path, _name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let (current, other) = (root.join("current"), root.join("other"));
    for repo in [&current, &other] {
        fs::create_dir_all(repo).unwrap();
        sh(repo, &["init", "-q", "."]);
        fs::write(repo.join("a.txt"), "one\n").unwrap();
        sh(repo, &["add", "-A"]);
        sh(repo, &["commit", "-qm", "init"]);
        fs::write(repo.join("a.txt"), "two\n").unwrap();
    }
    review_done(hook, &current, "code-review");
    (current, other)
}

/// Exit code and stderr of `native` run in `cwd` with `env` added, as git runs a `pre-commit` hook: no event on stdin.
fn native_run(hook: &Path, cwd: &Path, env: &[(&str, &Path)]) -> (i32, String) {
    let out = Command::new(hook)
        .args(["tool", "review-gate", "native"])
        .current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (*k, *v)))
        .env_remove("CLAUDE_PROJECT_DIR")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The waiver under the native hook: honoured, unreadable, not UTF-8 or read-only.
fn native_waiver_checks(hook: &Path, current: &Path) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    let waiver = current
        .join(".git")
        .join(["claude-review", "waived"].join("-"));
    fs::write(&waiver, "").unwrap();
    out.push((
        "native honours the waiver",
        native_run(hook, current, &[]).0 == 0,
    ));
    fs::remove_file(&waiver).unwrap();
    // a waiver that cannot be read is ignored too: it is neither stamped nor dropped, and the user is told to fix it
    fs::write(&waiver, "").unwrap();
    fs::set_permissions(&waiver, std::os::unix::fs::PermissionsExt::from_mode(0o200)).unwrap();
    // root reads a write-only file: the case cannot be built then
    if fs::File::open(&waiver).is_err() {
        let (code, stderr) = native_run(hook, current, &[]);
        out.push((
            "native refuses an unreviewed diff whose waiver it cannot read",
            code == 2 && stderr.contains("/code-review low"),
        ));
        let path = waiver.canonicalize().unwrap();
        out.push(("that refusal tells the agent to ask the user to fix the unreadable waiver, by its path", stderr.contains("could not be read") && stderr.contains("ask the user to fix") && stderr.contains(path.to_str().unwrap())));
        out.push((
            "native leaves the unreadable waiver unstamped",
            fs::metadata(&waiver).is_ok_and(|m| m.len() == 0),
        ));
        review_done(hook, current, "code-review");
        out.push((
            "native lets through a reviewed diff whose waiver it cannot read",
            native_run(hook, current, &[]).0 == 0,
        ));
        // a file the review never saw, so the next cases judge an unreviewed diff again
        fs::write(current.join("unseen.txt"), "x\n").unwrap();
    }
    fs::set_permissions(&waiver, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    fs::remove_file(&waiver).unwrap();
    // content that is not UTF-8 is read, not unreadable: it is no stamp, so it is dropped like a stale one
    fs::write(&waiver, [0xff, 0xfe]).unwrap();
    let (code, stderr) = native_run(hook, current, &[]);
    out.push((
        "native drops a waiver whose content is not UTF-8 and is no stamp",
        code == 2 && !stderr.contains("could not be read") && !waiver.exists(),
    ));
    // a waiver that cannot be stamped is ignored: the review alone judges the commit, and the user is told to fix it
    fs::write(&waiver, "").unwrap();
    let mut readonly = fs::metadata(&waiver).unwrap().permissions();
    readonly.set_readonly(true);
    fs::set_permissions(&waiver, readonly.clone()).unwrap();
    // root writes a read-only file: the case cannot be built then
    if fs::OpenOptions::new().write(true).open(&waiver).is_err() {
        let (code, stderr) = native_run(hook, current, &[]);
        out.push((
            "native refuses an unreviewed diff whose waiver it cannot stamp",
            code == 2 && stderr.contains("/code-review low"),
        ));
        let path = waiver.canonicalize().unwrap();
        out.push((
            "that refusal tells the agent to ask the user to fix the waiver, by its path",
            stderr.contains("ask the user to fix") && stderr.contains(path.to_str().unwrap()),
        ));
        review_done(hook, current, "code-review");
        out.push((
            "native lets through a reviewed diff whose waiver it cannot stamp",
            native_run(hook, current, &[]).0 == 0,
        ));
    }
    #[allow(clippy::permissions_set_readonly_false)]
    readonly.set_readonly(false);
    fs::set_permissions(&waiver, readonly).unwrap();
    fs::remove_file(&waiver).unwrap();
    out
}

// spec: REV-9, REV-10
fn native_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let (current, other) = target_fixture(hook, "rev9");
    let (code, stderr) = native_run(hook, &other, &[]);
    let mut out = vec![
        ("native refuses an unreviewed repository with a message naming the review and its level", code != 0 && stderr.contains("/code-review low")),
        ("native lets through a reviewed repository", native_run(hook, &current, &[]).0 == 0),
    ];
    // git resolved the repository: the text of the command is not involved
    let (git_dir, tree) = (other.join(".git"), other.clone());
    out.push((
        "native judges the repository named by GIT_DIR and GIT_WORK_TREE",
        native_run(
            hook,
            &current,
            &[("GIT_DIR", &git_dir), ("GIT_WORK_TREE", &tree)],
        )
        .0 != 0,
    ));
    review_done(hook, &other, "code-review");
    fs::write(current.join("new.txt"), "x\n").unwrap();
    out.push((
        "native lets through a reviewed GIT_DIR while the current directory is unreviewed",
        native_run(
            hook,
            &current,
            &[("GIT_DIR", &git_dir), ("GIT_WORK_TREE", &tree)],
        )
        .0 == 0,
    ));
    out.push((
        "native refuses the unreviewed current directory",
        native_run(hook, &current, &[]).0 != 0,
    ));
    out.extend(native_waiver_checks(hook, &current));
    fs::remove_file(current.join("new.txt")).unwrap();
    let _ = fs::remove_file(current.join("unseen.txt"));
    fs::write(current.join("a.txt"), "one\n").unwrap();
    out.push((
        "native lets through a repository with nothing to commit",
        native_run(hook, &current, &[]).0 == 0,
    ));
    let _ = fs::remove_dir_all(current.parent().unwrap());
    out
}

// spec: REV-9, REV-10
/// The hook as git runs it: installed as `pre-commit`, it refuses a real commit nobody reviewed and lets the reviewed one through.
fn native_hook_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    use std::os::unix::fs::PermissionsExt;
    let (current, _other) = target_fixture(hook, "rev9git");
    let pre_commit = current.join(".git/hooks/pre-commit");
    fs::write(
        &pre_commit,
        format!(
            "#!/bin/sh\nexec '{}' tool review-gate native\n",
            hook.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&pre_commit, fs::Permissions::from_mode(0o755)).unwrap();
    let commit = |args: &[&str]| {
        Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .current_dir(&current)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    let commits = || {
        let log = Command::new("git")
            .args(["rev-list", "--count", "HEAD"])
            .current_dir(&current)
            .output()
            .unwrap();
        String::from_utf8_lossy(&log.stdout)
            .trim()
            .parse::<u32>()
            .unwrap()
    };
    let before = commits();
    let reviewed = commit(&["commit", "-qam", "x"]) && commits() == before + 1;
    fs::write(current.join("a.txt"), "three\n").unwrap();
    let refused = !commit(&["commit", "-qam", "y"]);
    // the commit records the issues only: the unreviewed change of `a.txt` stays out of it
    fs::create_dir_all(current.join("issues")).unwrap();
    fs::write(current.join("issues/i.md"), "one\n").unwrap();
    fs::write(current.join("issues/j.md"), "one\n").unwrap();
    let staged_issue = commit(&["add", "issues/i.md"])
        && commit(&["commit", "-qm", "issue"])
        && commits() == before + 2;
    let only_issue = commit(&["add", "-N", "issues/j.md"])
        && commit(&["commit", "-qm", "issue", "--", "issues/j.md"])
        && commits() == before + 3;
    let issue_with_code = commit(&["add", "issues/i.md", "a.txt"]) && {
        fs::write(current.join("issues/i.md"), "two\n").unwrap();
        commit(&["add", "issues/i.md"]) && !commit(&["commit", "-qm", "both"])
    };
    let out = vec![
        (
            "git runs the native hook and a reviewed commit -a passes",
            reviewed,
        ),
        (
            "git refuses through the native hook a commit nobody reviewed",
            refused,
        ),
        (
            "git lets through a commit of a staged issue file while an unreviewed file stays modified",
            staged_issue,
        ),
        (
            "git lets through `commit -- issues/<file>` while an unreviewed file stays modified",
            only_issue,
        ),
        (
            "git refuses a commit whose index holds an unreviewed file next to the issue",
            issue_with_code,
        ),
        (
            "the refused commits left the history alone",
            commits() == before + 3,
        ),
    ];
    let _ = fs::remove_dir_all(current.parent().unwrap());
    out
}

// spec: REV-11
fn native_worktree_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let (main, linked) = (root.join("main"), root.join("linked"));
    fs::create_dir_all(&main).unwrap();
    sh(&main, &["init", "-q", "."]);
    fs::write(main.join("a.txt"), "one\n").unwrap();
    sh(&main, &["add", "-A"]);
    sh(&main, &["commit", "-qm", "init"]);
    sh(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            linked.to_str().unwrap(),
        ],
    );
    fs::write(main.join("a.txt"), "two\n").unwrap();
    fs::write(linked.join("a.txt"), "two\n").unwrap();
    review_done(hook, &linked, "code-review");
    let linked_git = git_dir_of(&linked);
    let out = vec![
        (
            "native lets through a reviewed linked worktree",
            native_run(hook, &linked, &[("GIT_DIR", &linked_git)]).0 == 0,
        ),
        (
            "native refuses the unreviewed main worktree",
            native_run(hook, &main, &[]).0 != 0,
        ),
    ];
    let _ = fs::remove_dir_all(&root);
    out
}

// spec: REV-12
/// What skips the native hook is refused on a commit line even when the repository is reviewed; the same words in a
/// message, or in a command that is not a commit, are not.
fn skipped_hook_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    // `check` reads the text only: no repository needed
    let current = std::env::temp_dir();
    let refused = [
        "git commit -n -m x",
        "git commit --no-verify -m x",
        "git commit -m x --no-verify",
        "git commit --no-ver -m x",
        "git commit -nm x",
        "git commit -anm x",
        "git commit -m x -n",
        "git -C . commit -n -m x",
        "git commit -m 'a b' --no-verify",
        "git -c core.hooksPath=/dev/null commit -m x",
        "git -c core.hookspath=/dev/null commit -m x",
        "git --config-env=core.hooksPath=H commit -m x",
        "git config core.hooksPath /dev/null",
        "git config --local core.hooksPath /dev/null",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/x git commit -m x",
        "git commit -m x && git commit -n -m y",
        "git commit -m a; git commit --no-verify -m b",
        "git config --get a && git -c core.hooksPath=/x commit -m x",
        "git -c core.hooksPath=/x commit -m x; echo --get",
        "git commit -m x --trailer 'a: b' -n",
        "git --work-tree=\"my dir\" commit -n -m x",
        "sudo -u git git commit -n -m x",
        "git commit -m x \\\n  --no-verify",
    ];
    let allowed = [
        "git commit -am x",
        "git commit -m 'fix -n'",
        "git commit -m \"fix --no-verify\"",
        "git commit -mn",
        "git commit -m x -- -n",
        "git commit -m \"document core.hooksPath\"",
        "git commit --message='core.hooksPath' -a",
        "git config --get core.hooksPath",
        "git config core.hooksPath",
        "git config --unset core.hooksPath",
        "git log --grep core.hooksPath",
        "git commit -m x --trailer -n",
        "git commit -F - <<EOF\ncore.hooksPath is documented here\nEOF",
        "git log -n 5 --grep commit",
        "git status -n",
    ];
    let mut out: Vec<(&'static str, bool)> = refused
        .iter()
        .map(|c| {
            (*c, {
                let (code, stderr) = commit_attempt(hook, &current, c);
                code == 2 && stderr.contains("native")
            })
        })
        .collect();
    out.extend(
        allowed
            .iter()
            .map(|c| (*c, commit_attempt(hook, &current, c).0 == 0)),
    );
    out
}

// spec: REV-21
/// A `git add` of every file is refused in any command of the line; one of named files, or the same words in a
/// message or in another command of the line, is not.
fn stage_everything_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let current = std::env::temp_dir();
    let refused = [
        "git add -A",
        "git add --all",
        "git add .",
        "git add -v -A",
        "git -C repo add .",
        "git add -A && git commit -m x",
        "cd repo; git add .",
        "git status && git add --all",
        "cat <<EOF > m\nx\nEOF\ngit add -A",
        "echo $((1 << 2))\ngit add -A",
        "cat <<END-X\nx\nEND-X\ngit add .",
        "echo \\\"x; git add -A",
        "git add -Av",
        "git add ./",
        "git add :/",
    ];
    let allowed = [
        "git add src/a.rs",
        "git add ./src",
        "git add -u",
        "git commit -am x",
        "git commit -m 'git add -A'",
        "git commit -m \"stop git add .\"",
        "git add a.rs && ls .",
        "git status . && git add b.rs",
        "git commit -m 'never `git add -A`'",
        "git commit -F - <<'EOF'\nfeat: x\n\ngit add -A; git add .\nEOF",
        "python3 - <<EOF\nprint(\"`git add --all`\")\nEOF\ngit add c.rs",
        "git commit -m \"use \\\" git add . \\\" never\"",
        "git add a.rs # not git add -A",
        "git add -n .",
        "git add . --dry-run",
    ];
    let mut out: Vec<(&'static str, bool)> = refused
        .iter()
        .map(|c| {
            (*c, {
                let (code, stderr) = commit_attempt(hook, &current, c);
                code == 2 && stderr.contains("by name")
            })
        })
        .collect();
    out.extend(
        allowed
            .iter()
            .map(|c| (*c, commit_attempt(hook, &current, c).0 == 0)),
    );
    out
}

/// Runs `mode` for `event` with a `git` that only logs its arguments, and returns the logged calls (`None` when the
/// hook itself failed, so that "no call" never means "did not run").
fn git_calls(hook: &Path, mode: &str, event: &serde_json::Value) -> Option<Vec<String>> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let log = root.join("calls.log");
    let fake = bin.join("git");
    fs::write(
        &fake,
        format!("#!/bin/sh\necho \"$*\" >> '{}'\n", log.display()),
    )
    .unwrap();
    fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new(hook)
        .args(["tool", "review-gate", mode])
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    let ran = child.wait().unwrap().success();
    let calls = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect();
    let _ = fs::remove_dir_all(&root);
    ran.then_some(calls)
}

fn bash_calls(hook: &Path, command: &str) -> Option<Vec<String>> {
    git_calls(
        hook,
        "check",
        &serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}}),
    )
}

/// A command that is not a commit costs no `git` call: the hook runs on every command of the agent.
fn cheap_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let skill = |name: &str| serde_json::json!({"tool_name": "Skill", "tool_input": {"skill": name}, "tool_response": {"success": true}});
    vec![
        (
            "a command that is not a commit runs no git",
            bash_calls(hook, "ls -la") == Some(vec![]),
        ),
        (
            "a git command that is not a commit runs no git",
            bash_calls(hook, "git status") == Some(vec![]),
        ),
        // the native hook judges the repository: `check` only reads the text of the commit
        (
            "a commit runs no git",
            bash_calls(hook, "git commit -m x") == Some(vec![]),
        ),
        (
            "a skill other than the review runs no git",
            git_calls(hook, "record", &skill("update-docs")) == Some(vec![]),
        ),
        (
            "the review skill records at the top of its work tree",
            git_calls(hook, "record", &skill("code-review"))
                .is_some_and(|c| c.iter().any(|c| c.contains("--show-toplevel"))),
        ),
    ]
}

/// Untracked files are hashed by `git` itself (their paths on its stdin), never read by the hook and piped to it.
fn streaming_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(root.join("big.bin"), vec![b'x'; 1 << 20]).unwrap();
    let log = root.join("calls.log");
    // `ls-files` lists the big file; every `hash-object` logs the size of what it is fed
    let fake = bin.join("git");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  rev-parse) [ \"$2\" = --git-path ] && echo \"$3\" ;;\n  ls-files) printf 'big.bin\\0' ;;\n  hash-object) echo \"$* $(wc -c)\" >> '{}'; echo 0000000000000000000000000000000000000000 ;;\nesac\n",
        log.display()
    );
    fs::write(&fake, script).unwrap();
    fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let event = serde_json::json!({"tool_name": "Skill", "cwd": root, "tool_input": {"skill": "code-review"}, "tool_response": {"success": true}});
    let mut child = Command::new(hook)
        .args(["tool", "review-gate", "record"])
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    let ran = child.wait().unwrap().success();
    let fed: Vec<usize> = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.rsplit(' ').next()?.parse().ok())
        .collect();
    let _ = fs::remove_dir_all(&root);
    vec![
        ("the review ran with the fake git", ran && !fed.is_empty()),
        (
            "no content of an untracked file is piped to git",
            fed.iter().all(|n| *n < 1 << 16),
        ),
    ]
}

/// The marker still follows the content of an untracked file, and only that.
fn untracked_content_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let root = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("sub")).unwrap();
    sh(&root, &["init", "-q", "."]);
    fs::write(root.join("a.txt"), "one\n").unwrap();
    sh(&root, &["add", "-A"]);
    sh(&root, &["commit", "-qm", "init"]);
    fs::write(root.join("new.txt"), "aaaa\n").unwrap();
    fs::write(root.join("odd\nname.txt"), "aaaa\n").unwrap();
    fs::create_dir_all(root.join("nested")).unwrap();
    sh(&root.join("nested"), &["init", "-q", "."]);
    let locked = root.join("locked.txt");
    fs::write(&locked, "secret\n").unwrap();
    fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o000)).unwrap();
    let marker = |hook: &Path| {
        review_done(hook, &root, "code-review");
        let text = fs::read_to_string(root.join(".git/claude-code-review.json")).unwrap();
        serde_json::from_str::<serde_json::Value>(&text).unwrap()["hash"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let first = marker(hook);
    let mut out = vec![(
        "the marker is stable while nothing changes",
        marker(hook) == first,
    )];
    fs::write(root.join("new.txt"), "bbbb\n").unwrap();
    let second = marker(hook);
    out.push((
        "a content change of the same size changes the marker",
        second != first,
    ));
    fs::write(root.join("odd\nname.txt"), "bbbb\n").unwrap();
    out.push((
        "a file with a newline in its name is covered",
        marker(hook) != second,
    ));
    // a nested repository and an unreadable file do not stop the other files from being hashed
    fs::write(root.join("new.txt"), "cccc\n").unwrap();
    out.push((
        "a nested repository and an unreadable file leave the other files covered",
        marker(hook) != second,
    ));
    fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    let _ = fs::remove_dir_all(&root);
    out
}

/// A repository whose issues commit without review, and the waiver of the rest.
fn issues_only_checks(hook: &Path, project: &Path) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    let mut check = |name, ok| out.push((name, ok));
    let judged = |dir: &Path| native_run(hook, dir, &[]).0;
    let esc = project.join("esc");
    fs::create_dir_all(esc.join("issues")).unwrap();
    sh(&esc, &["init", "-q", "."]);
    fs::write(esc.join("issues/i.md"), "one\n").unwrap();
    fs::write(esc.join("code.txt"), "one\n").unwrap();
    sh(&esc, &["add", "-A"]);
    sh(&esc, &["commit", "-qm", "init"]);
    fs::write(esc.join("issues/i.md"), "two\n").unwrap();
    check(
        "lets through a commit that only touches issues",
        judged(&esc) == 0,
    );
    fs::write(esc.join("issues/j.md"), "new\n").unwrap();
    check("lets through a new issue file", judged(&esc) == 0);
    sh(&esc, &["mv", "code.txt", "issues/code.txt"]);
    check("refuses a code file renamed into issues", judged(&esc) == 2);
    sh(&esc, &["mv", "issues/code.txt", "code.txt"]);
    fs::write(esc.join("code.txt"), "two\n").unwrap();
    check("refuses issues mixed with code", judged(&esc) == 2);
    fs::write(esc.join("docs.md"), "doc\n").unwrap();
    fs::write(esc.join("code.txt"), "one\n").unwrap();
    check("refuses documentation", judged(&esc) == 2);
    let waiver = esc.join(".git/claude-review-waived");
    fs::write(&waiver, "").unwrap();
    check("lets through a commit the user waived", judged(&esc) == 0);
    check(
        "keeps the waiver while no commit happened",
        waiver.exists() && judged(&esc) == 0,
    );
    sh(&esc, &["add", "-A"]);
    sh(&esc, &["commit", "-qm", "waived"]);
    fs::write(esc.join("docs.md"), "doc two\n").unwrap();
    check(
        "consumes the waiver once a commit happened",
        judged(&esc) == 2 && !waiver.exists(),
    );

    // a waiver spent by a commit is gone at the next check, so the user's new one is not mistaken for it
    fs::write(&waiver, "").unwrap();
    check(
        "lets through the commit of a waiver created again",
        judged(&esc) == 0,
    );
    sh(&esc, &["add", "-A"]);
    sh(&esc, &["commit", "-qm", "waived again"]);
    fs::write(esc.join("issues/k.md"), "new\n").unwrap();
    check(
        "an issues-only check drops the waiver spent by a commit",
        judged(&esc) == 0 && !waiver.exists(),
    );
    out
}

/// The first commit of a repository: refused, waived, reviewed.
fn first_commit_checks(hook: &Path, project: &Path) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    let mut check = |name, ok| out.push((name, ok));
    let judged = |dir: &Path| native_run(hook, dir, &[]).0;
    let fresh = project.join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    sh(&fresh, &["init", "-q", "."]);
    fs::write(fresh.join("c.txt"), "x\n").unwrap();
    check(
        "refuses the first commit of a repository",
        judged(&fresh) == 2,
    );
    let fresh_waiver = fresh.join(".git/claude-review-waived");
    fs::write(&fresh_waiver, "").unwrap();
    check(
        "lets through the first commit the user waived",
        judged(&fresh) == 0,
    );
    check(
        "stamps the waiver of a repository without commit",
        fs::read_to_string(&fresh_waiver).unwrap().trim() == "no-head",
    );
    fs::remove_file(&fresh_waiver).unwrap();
    review_done(hook, &fresh, "code-review");
    check(
        "lets through the reviewed first commit",
        judged(&fresh) == 0,
    );
    out
}

/// A repository with `issues/foo.md` cited from the roadmap (the last open issue of its milestone `Last`), a doc and
/// another issue, closed by `close-issue` and
/// staged, then changed by `edit` (which stages what the commit records); whether `native` lets the commit through without any review.
fn close_passes(hook: &Path, edit: impl Fn(&Path)) -> bool {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().to_path_buf();
    for dir in ["issues/done", "docs", "script", ".claude"] {
        fs::create_dir_all(project.join(dir)).unwrap();
    }
    sh(&project, &["init", "-q", "."]);
    fs::write(
        project.join("issues/foo.md"),
        "---\npriority: high\n---\n# Foo\n",
    )
    .unwrap();
    fs::write(
        project.join("issues/bar.md"),
        "---\npriority: low\n---\n# Bar\nAfter [foo](foo.md).\n",
    )
    .unwrap();
    fs::write(project.join("issues/done/old.md"), "# Old\n").unwrap();
    fs::write(
        project.join(".claude/cardinal.toml"),
        "[project]\nroadmap = [\"ROADMAP.md\"]\n",
    )
    .unwrap();
    // close-issue sets `Last` done (CLS-8); `Open` still lists `bar`, `Other` does not list `foo`
    fs::write(
        project.join("ROADMAP.md"),
        "# Roadmap\n\n## Open\n- State: in issues\n- Issues:\n  - [Foo](issues/foo.md)\n  - [Bar](issues/bar.md)\n\n\
## Last\n- State: in issues\n- Issues:\n  - [Foo](issues/foo.md)\n  - [Old](issues/done/old.md)\n\n\
## Other\n- State: in issues\n- Issues:\n  - [Old](issues/done/old.md)\n",
    )
    .unwrap();
    fs::write(
        project.join("docs/a.md"),
        "See [foo](../issues/foo.md#goal).\n",
    )
    .unwrap();
    // close-issue never rewrites a shell script, nor a submodule
    fs::write(project.join("script/x.sh"), "cat issues/foo.md\n").unwrap();
    let sub = project.join("sub");
    fs::create_dir(&sub).unwrap();
    sh(&sub, &["init", "-q", "."]);
    sh(&sub, &["commit", "-q", "--allow-empty", "-m", "one"]);
    sh(&project, &["add", "-A"]);
    sh(&project, &["commit", "-qm", "init"]);
    let closed = Command::new(hook)
        .args(["tool", "close-issue", "issues/foo.md"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(closed.status.success(), "close-issue failed");
    sh(&project, &["add", "-A"]);
    edit(&project);
    native_run(hook, &project, &[]).0 == 0
}

/// Replaces `from` by `to` in the file `name` of `project`, and stages it when `staged`.
fn replace_in(project: &Path, name: &str, from: &str, to: &str, staged: bool) {
    let path = project.join(name);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{name} lacks {from}");
    fs::write(path, text.replacen(from, to, 1)).unwrap();
    if staged {
        sh(project, &["add", "--", name]);
    }
}

// spec: REV-20
/// The commit of a close holds, outside `issues/`, the rewrites `close-issue` made: it needs no review, and anything
/// else in it does.
fn close_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    vec![
        (
            "native lets through the close of an issue with the references and the milestone close-issue rewrote",
            close_passes(hook, |_| {}),
        ),
        (
            "native lets through a close whose milestone state close-issue set was put back",
            close_passes(hook, |p| {
                let last = "## Last\n- State: done";
                replace_in(p, "ROADMAP.md", last, "## Last\n- State: in issues", true);
            }),
        ),
        (
            "native refuses a close that also sets done a milestone with an issue still open",
            !close_passes(hook, |p| {
                let open = "## Open\n- State: in issues";
                replace_in(p, "ROADMAP.md", open, "## Open\n- State: done", true);
            }),
        ),
        (
            "native refuses a close that also sets done a milestone that does not list the closed issue",
            !close_passes(hook, |p| {
                let other = "## Other\n- State: in issues";
                replace_in(p, "ROADMAP.md", other, "## Other\n- State: done", true);
            }),
        ),
        (
            "native refuses a milestone set done when the index lacks one of its issues",
            !close_passes(hook, |p| sh(p, &["rm", "-q", "--", "issues/done/old.md"])),
        ),
        (
            "native refuses a close with an edited word on a rewritten line",
            !close_passes(hook, |p| replace_in(p, "docs/a.md", "See", "Read", true)),
        ),
        (
            "native refuses a close with a file added outside issues/",
            !close_passes(hook, |p| {
                fs::write(p.join("new.md"), "x\n").unwrap();
                sh(p, &["add", "--", "new.md"]);
            }),
        ),
        (
            "native refuses a close with a file deleted outside issues/",
            !close_passes(hook, |p| sh(p, &["rm", "-qf", "--", "docs/a.md"])),
        ),
        (
            "native refuses a close with a file whose mode changed",
            !close_passes(hook, |p| sh(p, &["update-index", "--chmod=+x", "docs/a.md"])),
        ),
        (
            "native refuses a close with a submodule moved",
            !close_passes(hook, |p| {
                sh(&p.join("sub"), &["commit", "-q", "--allow-empty", "-m", "two"]);
                sh(p, &["add", "sub"]);
            }),
        ),
        (
            "native refuses a close with a reference rewritten in a file close-issue never rewrites",
            !close_passes(hook, |p| {
                replace_in(p, "script/x.sh", "issues/foo.md", "issues/done/foo.md", true);
            }),
        ),
        (
            "native refuses rewritten references when the move of the issue is not staged",
            !close_passes(hook, |p| {
                sh(p, &["mv", "issues/done/foo.md", "issues/foo.md"]);
            }),
        ),
        (
            "native lets through a close while an unstaged change waits in the work tree",
            close_passes(hook, |p| {
                replace_in(p, "docs/a.md", "See", "Read", false);
                fs::write(p.join("new.md"), "x\n").unwrap();
            }),
        ),
    ]
}

#[test]
fn review_gate() {
    let hook = Path::new(env!("CARGO_BIN_EXE_cardinal")).to_path_buf();
    let project = tempfile::tempdir().unwrap().keep();
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&project).unwrap();
    sh(&project, &["init", "-q", "."]);
    fs::write(project.join("a.txt"), "one\n").unwrap();
    sh(&project, &["add", "-A"]);
    sh(&project, &["commit", "-qm", "init"]);

    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let judged = |dir: &Path| native_run(&hook, dir, &[]).0;

    check(
        "lets through a commit with nothing changed",
        judged(&project) == 0,
    );

    fs::write(project.join("a.txt"), "two\n").unwrap();
    let (code, stderr) = native_run(&hook, &project, &[]);
    check(
        "refuses a commit without any review",
        code == 2 && stderr.contains("/code-review low"),
    );
    check(
        "check leaves the judgment of a commit to the native hook",
        commit_attempt(&hook, &project, "git commit -am x").0 == 0,
    );

    review_done(&hook, &project, "simplify");
    check("ignores the end of another skill", judged(&project) == 2);

    review_done(&hook, &project, "code-review");
    check(
        "lets through a commit of the reviewed diff",
        judged(&project) == 0,
    );

    fs::write(project.join("a.txt"), "two, fixed\n").unwrap();
    check(
        "accepts a fix made after the review in a reviewed file",
        judged(&project) == 0,
    );
    fs::write(project.join("a.txt"), "two\n").unwrap();

    fs::write(project.join("new.txt"), "x\n").unwrap();
    check("refuses a file the review never saw", judged(&project) == 2);

    fs::remove_file(project.join("new.txt")).unwrap();
    check("accepts the reviewed diff again", judged(&project) == 0);

    sh(&project, &["commit", "-aqm", "feat: reviewed"]);
    fs::write(project.join("a.txt"), "three\n").unwrap();
    check(
        "refuses a review made before the last commit",
        judged(&project) == 2,
    );

    fs::create_dir_all(project.join("deep")).unwrap();
    check(
        "sees the whole work tree from a subdirectory",
        judged(&project.join("deep")) == 2,
    );

    review_done(&hook, &project, "code-review");
    check(
        "lets through the reviewed diff of the next change",
        judged(&project) == 0,
    );
    fs::write(project.join("é.txt"), "x\n").unwrap();
    check("refuses a new non-ASCII file", judged(&project) == 2);
    fs::remove_file(project.join("é.txt")).unwrap();

    let failed_review = serde_json::json!({
        "tool_name": "Skill", "cwd": project, "tool_input": {"skill": "code-review"}, "tool_response": {"success": false}
    });
    fs::write(project.join("f.txt"), "z\n").unwrap();
    run(&hook, "record", &project, &failed_review);
    check("does not record a failed review", judged(&project) == 2);

    for (name, ok) in issues_only_checks(&hook, &project)
        .into_iter()
        .chain(first_commit_checks(&hook, &project))
    {
        check(name, ok);
    }

    for checks in [
        worktree_checks,
        recipe_checks,
        native_checks,
        native_hook_checks,
        native_worktree_checks,
        skipped_hook_checks,
        stage_everything_checks,
        cheap_checks,
        streaming_checks,
        untracked_content_checks,
        close_checks,
    ] {
        for (name, ok) in checks(&hook) {
            check(name, ok);
        }
    }

    let _ = fs::remove_dir_all(&project);
    assert!(!failed, "a check failed: see the FAIL lines");
}

// spec: FEA-36
/// With the issue files off, `issues/` is a directory like any other: a commit of it, or of a close, waits for a review.
#[test]
fn issues_are_code_with_the_issue_files_off() {
    let hook = Path::new(env!("CARGO_BIN_EXE_cardinal")).to_path_buf();
    let off = "[cardinal]\ndisabled = [\"issue-files\"]\n";
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path();
    fs::create_dir_all(project.join("issues")).unwrap();
    fs::create_dir_all(project.join(".claude")).unwrap();
    sh(project, &["init", "-q", "."]);
    fs::write(project.join(".claude/cardinal.toml"), off).unwrap();
    fs::write(project.join("issues/i.md"), "one\n").unwrap();
    fs::write(project.join("code.txt"), "one\n").unwrap();
    sh(project, &["add", "-A"]);
    sh(project, &["commit", "-qm", "init"]);
    fs::write(project.join("code.txt"), "two\n").unwrap();
    review_done(&hook, project, "code-review");
    assert_eq!(
        native_run(&hook, project, &[]).0,
        0,
        "the reviewed diff passes"
    );
    fs::write(project.join("issues/x.md"), "new\n").unwrap();
    let (code, stderr) = native_run(&hook, project, &[]);
    assert!(
        code == 2 && stderr.contains("never saw"),
        "a new file under issues/ is one the review never saw: {stderr}"
    );
    fs::remove_file(project.join("issues/x.md")).unwrap();
    fs::write(project.join("code.txt"), "one\n").unwrap();
    fs::write(project.join("issues/i.md"), "two\n").unwrap();
    assert_eq!(
        native_run(&hook, project, &[]).0,
        2,
        "a diff under issues/ only waits for a review"
    );
    fs::write(project.join("code.txt"), "two\n").unwrap();
    sh(project, &["add", "--", "issues/i.md"]);
    assert_eq!(
        native_run(&hook, project, &[]).0,
        2,
        "an index under issues/ only waits for a review"
    );
    assert!(
        !close_passes(&hook, |p| {
            let config = fs::read_to_string(p.join(".claude/cardinal.toml")).unwrap();
            fs::write(p.join(".claude/cardinal.toml"), format!("{config}{off}")).unwrap();
        }),
        "the close of an issue waits for a review"
    );
}
