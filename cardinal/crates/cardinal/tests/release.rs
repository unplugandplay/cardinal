//! Behavior test for `cardinal tool release`, on temporary git repositories.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn git_out(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = git_out(dir, args);
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository on `main` with one first commit.
fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main", "."]);
    commit(dir.path(), "chore: start", "start.txt");
    dir
}

/// Commit a change of the file `file` with the message `message`.
fn commit(dir: &Path, message: &str, file: &str) {
    let path = dir.join(file);
    let before = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::write(&path, format!("{before}{message}\n")).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", message, "--no-verify"]);
}

fn tag(dir: &Path, name: &str) {
    git(dir, &["tag", "-a", name, "-m", name]);
}

/// Run the tool with the identity and signing settings of `git_out`, which a CI runner lacks.
fn release(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cardinal"))
        .args(["tool", "release"])
        .args(args)
        .env("GIT_CONFIG_COUNT", "4")
        .env("GIT_CONFIG_KEY_0", "user.email")
        .env("GIT_CONFIG_VALUE_0", "t@t")
        .env("GIT_CONFIG_KEY_1", "user.name")
        .env("GIT_CONFIG_VALUE_1", "t")
        .env("GIT_CONFIG_KEY_2", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_2", "false")
        .env("GIT_CONFIG_KEY_3", "tag.gpgsign")
        .env("GIT_CONFIG_VALUE_3", "false")
        .current_dir(dir)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn changelog(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("CHANGELOG.md")).unwrap_or_default()
}

fn config(dir: &Path, text: &str) {
    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::write(dir.join(".claude/cardinal.toml"), text).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "chore: configure", "--no-verify"]);
}

/// The proposed version printed by `release` with no argument.
fn proposed(dir: &Path) -> String {
    let out = release(dir, &[]);
    assert!(out.status.success(), "{out:?}");
    stdout(&out).lines().next().unwrap_or_default().to_string()
}

// spec: REL-1
#[test]
fn proposes_the_next_version_from_the_commits_since_the_last_tag() {
    let dir = repo();
    assert_eq!(proposed(dir.path()), "0.1.0", "no version tag yet");
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "fix: a", "a.txt");
    commit(dir.path(), "feat(x): b", "b.txt");
    assert_eq!(proposed(dir.path()), "0.3.0");

    let dir = repo();
    tag(dir.path(), "v1.4.2");
    commit(dir.path(), "docs: a", "a.txt");
    commit(dir.path(), "fix: b", "b.txt");
    assert_eq!(proposed(dir.path()), "1.4.3");
}

// spec: REL-2
#[test]
fn a_marked_commit_a_footer_or_a_listed_file_is_breaking() {
    for (message, file) in [
        ("feat!: drop a flag", "a.txt"),
        ("fix(x)!: change a default", "a.txt"),
        (
            "fix: change a default\n\nBREAKING CHANGE: the default moved",
            "a.txt",
        ),
        (
            "fix: change a default\n\nBREAKING-CHANGE: the default moved",
            "a.txt",
        ),
        ("feat: raise the format", "cardinal-format"),
        ("raise the format", "cardinal-format"),
        (
            "change a default\n\nBREAKING CHANGE: the default moved",
            "a.txt",
        ),
    ] {
        let dir = repo();
        config(
            dir.path(),
            "[project]\nrelease_breaking_files = [\"cardinal-format\"]\n",
        );
        tag(dir.path(), "v0.2.0");
        commit(dir.path(), message, file);
        assert_eq!(proposed(dir.path()), "1.0.0", "{message}");
        assert!(
            !release(dir.path(), &["0.3.0"]).status.success(),
            "{message}"
        );
    }
}

// spec: REL-3
#[test]
fn refuses_a_malformed_taken_written_or_too_low_version() {
    let dir = repo();
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: b", "b.txt");
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# Changelog\n\n## [0.4.0] - 2026-01-01\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "docs: changelog", "--no-verify"],
    );
    let before = changelog(dir.path());
    for version in [
        "1.2", "0.3.x", "0.03.0", "v0.2.0", "0.2.0", "0.2.1", "0.4.0",
    ] {
        let out = release(dir.path(), &[version]);
        assert!(!out.status.success(), "{version} accepted: {out:?}");
        assert_eq!(
            changelog(dir.path()),
            before,
            "{version} changed the changelog"
        );
    }
    assert!(release(dir.path(), &["v0.3.0"]).status.success());

    let dir = repo();
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: b", "b.txt");
    assert!(
        release(dir.path(), &["1.0.0"]).status.success(),
        "a higher version is accepted"
    );
}

// spec: REL-4
#[test]
fn writes_the_section_of_the_version_above_the_previous_ones() {
    let dir = repo();
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# Changelog\n\nIntro.\n\n## [0.2.0] - 2026-01-01\n\nFirst numbered release.\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "docs: changelog", "--no-verify"],
    );
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat!: drop the old flag", "a.txt");
    commit(dir.path(), "feat: add a flag", "b.txt");
    commit(dir.path(), "fix(x): repair a link", "c.txt");
    commit(dir.path(), "chore: tidy", "d.txt");
    let out = release(dir.path(), &["1.0.0"]);
    assert!(out.status.success(), "{out:?}");
    let text = changelog(dir.path());
    let today = chrono::Local::now().format("%Y-%m-%d");
    let expected = format!(
        "# Changelog\n\nIntro.\n\n## [1.0.0] - {today}\n\n### Breaking\n\n- drop the old flag\n\n### Added\n\n- add a flag\n\n### Fixed\n\n- repair a link\n\n## [0.2.0] - 2026-01-01\n\nFirst numbered release.\n"
    );
    assert_eq!(text, expected);
}

// spec: REL-4
#[test]
fn the_first_release_creates_the_changelog_with_a_single_line() {
    let dir = repo();
    commit(dir.path(), "feat: a", "a.txt");
    let out = release(dir.path(), &["0.2.0"]);
    assert!(out.status.success(), "{out:?}");
    let text = changelog(dir.path());
    assert!(text.starts_with("# Changelog\n"), "{text}");
    let today = chrono::Local::now().format("%Y-%m-%d");
    assert!(
        text.ends_with(&format!(
            "\n## [0.2.0] - {today}\n\nFirst numbered release.\n"
        )),
        "{text}"
    );
}

// spec: REL-5
#[test]
fn runs_the_version_command_of_the_project_with_the_version() {
    let dir = repo();
    config(
        dir.path(),
        "[project]\nrelease_version_command = \"printf %s >version.txt\"\n",
    );
    assert!(release(dir.path(), &["0.2.0"]).status.success());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("version.txt")).unwrap(),
        "0.2.0"
    );

    let dir = repo();
    config(
        dir.path(),
        "[project]\nrelease_version_command = \"false\"\n",
    );
    assert!(!release(dir.path(), &["0.2.0"]).status.success());
    assert!(
        !dir.path().join("CHANGELOG.md").exists(),
        "the changelog is left as it was"
    );
}

// spec: REL-6, REL-7, REL-8
#[test]
fn prepares_then_tags_locally_and_never_pushes() {
    let dir = repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "-q", "--bare", "."]);
    git(
        dir.path(),
        &["remote", "add", "origin", &remote.path().to_string_lossy()],
    );
    git(dir.path(), &["push", "-q", "origin", "main"]);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: add a flag", "a.txt");
    let head = git(dir.path(), &["rev-parse", "HEAD"]);

    let out = release(dir.path(), &["0.3.0"]);
    assert!(out.status.success(), "{out:?}");
    assert!(stdout(&out).contains("--tag"), "{}", stdout(&out));
    assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), head, "no commit");
    assert_eq!(git(dir.path(), &["tag", "-l", "v0.3.0"]), "", "no tag");

    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "chore: release v0.3.0", "--no-verify"],
    );
    let out = release(dir.path(), &["0.3.0", "--tag"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout(&out).contains("git push origin v0.3.0"),
        "{}",
        stdout(&out)
    );
    assert_eq!(git(dir.path(), &["cat-file", "-t", "v0.3.0"]), "tag");
    assert_eq!(
        git(dir.path(), &["rev-parse", "v0.3.0^{commit}"]),
        git(dir.path(), &["rev-parse", "HEAD"])
    );
    let message = git(dir.path(), &["tag", "-l", "--format=%(contents)", "v0.3.0"]);
    assert!(
        message.starts_with("## [0.3.0] - ") && message.contains("### Added\n\n- add a flag"),
        "{message}"
    );
    assert_eq!(git(remote.path(), &["tag", "-l"]), "", "nothing pushed");
}

// spec: REL-10
#[test]
fn refuses_to_tag_an_uncommitted_missing_or_tagged_release() {
    let dir = repo();
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: add a flag", "a.txt");
    assert!(
        !release(dir.path(), &["0.3.0", "--tag"]).status.success(),
        "no section"
    );
    assert!(release(dir.path(), &["0.3.0"]).status.success());
    assert!(
        !release(dir.path(), &["0.3.0", "--tag"]).status.success(),
        "uncommitted"
    );
    assert_eq!(git(dir.path(), &["tag", "-l", "v0.3.0"]), "");
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "chore: release v0.3.0", "--no-verify"],
    );
    assert!(release(dir.path(), &["0.3.0", "--tag"]).status.success());
    assert!(
        !release(dir.path(), &["0.3.0", "--tag"]).status.success(),
        "tag exists"
    );
}

// spec: REL-9
#[test]
fn lists_and_marks_the_done_milestones_not_yet_released() {
    let dir = repo();
    std::fs::write(
        dir.path().join("ROADMAP.md"),
        "# Roadmap\n\n## Old\n\n- Goal: a\n- State: done\n- Released: v0.2.0\n\n## New\n\n- Goal: b\n- State: done\n- Issues:\n  - x\n\n## Later\n\n- Goal: c\n- State: in issues\n",
    )
    .unwrap();
    config(dir.path(), "[project]\nroadmap = [\"ROADMAP.md\"]\n");
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: add a flag", "a.txt");
    let out = release(dir.path(), &["0.3.0"]);
    assert!(out.status.success(), "{out:?}");
    let today = chrono::Local::now().format("%Y-%m-%d");
    assert!(
        changelog(dir.path()).contains(&format!(
            "## [0.3.0] - {today}\n\n### Milestones\n\n- New\n\n### Added\n\n- add a flag\n"
        )),
        "{}",
        changelog(dir.path())
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ROADMAP.md")).unwrap(),
        "# Roadmap\n\n## Old\n\n- Goal: a\n- State: done\n- Released: v0.2.0\n\n## New\n\n- Goal: b\n- State: done\n- Released: v0.3.0\n- Issues:\n  - x\n\n## Later\n\n- Goal: c\n- State: in issues\n"
    );
}

// spec: REL-9
/// A `## ` line or a `State:` line inside a fenced block or an HTML comment is no milestone of its own.
#[test]
fn skips_the_milestones_of_fences_and_comments() {
    let dir = repo();
    let roadmap = "# Roadmap\n\n## Real\n\n- Goal: a\n\n```markdown\n## Fake\n\n- State: done\n```\n\n- State: in issues\n\n<!--\n## Template\n\n- State: done\n-->\n";
    std::fs::write(dir.path().join("ROADMAP.md"), roadmap).unwrap();
    config(dir.path(), "[project]\nroadmap = [\"ROADMAP.md\"]\n");
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: add a flag", "a.txt");
    let out = release(dir.path(), &["0.3.0"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        !changelog(dir.path()).contains("### Milestones"),
        "{}",
        changelog(dir.path())
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ROADMAP.md")).unwrap(),
        roadmap
    );
}

// spec: REL-3
#[test]
fn refuses_a_release_with_no_commit_since_the_last_tag() {
    let dir = repo();
    tag(dir.path(), "v0.3.0");
    let out = release(dir.path(), &["0.3.1"]);
    assert!(!out.status.success(), "{out:?}");
    assert!(!dir.path().join("CHANGELOG.md").exists());
}

// spec: REL-4
#[test]
fn the_section_goes_below_the_unreleased_one() {
    let dir = repo();
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n- soon\n\n## [0.2.0] - 2026-01-01\n\nFirst numbered release.\n",
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "docs: changelog", "--no-verify"],
    );
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "fix: a", "a.txt");
    assert!(release(dir.path(), &["0.2.1"]).status.success());
    let today = chrono::Local::now().format("%Y-%m-%d");
    assert_eq!(
        changelog(dir.path()),
        format!("# Changelog\n\n## [Unreleased]\n\n- soon\n\n## [0.2.1] - {today}\n\n### Fixed\n\n- a\n\n## [0.2.0] - 2026-01-01\n\nFirst numbered release.\n")
    );
}

// spec: REL-5
#[test]
fn a_failing_version_command_leaves_the_roadmap_as_it_was() {
    let dir = repo();
    let roadmap = "# Roadmap\n\n## M\n\n- State: done\n";
    std::fs::write(dir.path().join("ROADMAP.md"), roadmap).unwrap();
    config(
        dir.path(),
        "[project]\nroadmap = [\"ROADMAP.md\"]\nrelease_version_command = \"false\"\n",
    );
    assert!(!release(dir.path(), &["0.2.0"]).status.success());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ROADMAP.md")).unwrap(),
        roadmap
    );
}

// spec: REL-10
#[test]
fn refuses_to_tag_with_an_untracked_file() {
    let dir = repo();
    commit(dir.path(), "feat: a", "a.txt");
    assert!(release(dir.path(), &["0.2.0"]).status.success());
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &["commit", "-qm", "chore: release v0.2.0", "--no-verify"],
    );
    std::fs::write(dir.path().join("VERSION"), "0.2.0\n").unwrap();
    assert!(!release(dir.path(), &["0.2.0", "--tag"]).status.success());
    assert_eq!(git(dir.path(), &["tag", "-l", "v0.2.0"]), "");
}

const OFF: &str = "[cardinal]\ndisabled = [\"conventional-commits\"]\n\n[project]\nrelease_breaking_files = [\"format\"]\n";
const WARNING: &str =
    "release: conventional-commits is off: the version is proposed from release_breaking_files only";

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

// spec: REL-11
#[test]
fn without_the_convention_only_a_listed_file_is_breaking_and_release_warns() {
    let dir = repo();
    config(dir.path(), OFF);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat: a", "a.txt");
    commit(dir.path(), "feat!: drop b", "b.txt");
    commit(dir.path(), "Change c\n\nBREAKING CHANGE: c moved", "c.txt");
    let out = release(dir.path(), &[]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(stdout(&out), "0.2.1\n", "no type read: a patch");
    assert!(stderr(&out).contains(WARNING), "{out:?}");
    let out = release(dir.path(), &["0.2.1"]);
    assert!(out.status.success(), "{out:?}");
    assert!(stderr(&out).contains(WARNING), "{out:?}");

    let dir = repo();
    config(dir.path(), OFF);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "Raise the format", "format");
    assert_eq!(proposed(dir.path()), "1.0.0");
}

// spec: REL-12
#[test]
fn without_the_convention_the_section_lists_the_whole_subjects_under_changes() {
    let dir = repo();
    config(dir.path(), OFF);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "Raise the format", "format");
    commit(dir.path(), "Add a flag", "a.txt");
    commit(dir.path(), "fix: repair a link", "b.txt");
    let out = release(dir.path(), &["1.0.0"]);
    assert!(out.status.success(), "{out:?}");
    let today = chrono::Local::now().format("%Y-%m-%d");
    assert!(
        changelog(dir.path()).ends_with(&format!(
            "## [1.0.0] - {today}\n\n### Breaking\n\n- Raise the format\n\n### Changed\n\n- Add a flag\n- fix: repair a link\n"
        )),
        "{}",
        changelog(dir.path())
    );
}

const ON: &str = "[project]\nrelease_breaking_files = [\"format\"]\n";

// spec: REL-11
#[test]
fn each_commit_is_read_with_the_convention_of_its_revision() {
    let dir = repo();
    config(dir.path(), ON);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat!: drop the old API", "a.txt");
    config(dir.path(), OFF);
    commit(dir.path(), "Add b", "b.txt");
    let out = release(dir.path(), &[]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        stdout(&out),
        "1.0.0\n",
        "the breaking mark written under the convention counts"
    );
    assert!(stderr(&out).contains(WARNING), "{out:?}");

    let dir = repo();
    config(dir.path(), OFF);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "Add x", "x.txt");
    config(dir.path(), ON);
    commit(dir.path(), "feat: add c", "c.txt");
    let out = release(dir.path(), &[]);
    assert_eq!(stdout(&out), "0.3.0\n", "{out:?}");
    assert!(
        stderr(&out).contains(WARNING),
        "a commit written with the convention off: {out:?}"
    );
}

// spec: REL-11
#[test]
fn the_warning_says_when_no_breaking_file_is_listed() {
    let second = "release: release_breaking_files lists no file: the commits written with conventional-commits off are a patch";
    let dir = repo();
    config(
        dir.path(),
        "[cardinal]\ndisabled = [\"conventional-commits\"]\n",
    );
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "Add a", "a.txt");
    let out = release(dir.path(), &[]);
    assert!(
        stderr(&out).contains(&format!("{WARNING}\n{second}")),
        "{out:?}"
    );

    let dir = repo();
    config(dir.path(), OFF);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "Add a", "a.txt");
    let out = release(dir.path(), &[]);
    assert!(!stderr(&out).contains(second), "{out:?}");
}

// spec: REL-12
#[test]
fn a_release_across_the_turn_off_lists_each_commit_by_its_convention() {
    let dir = repo();
    config(dir.path(), ON);
    tag(dir.path(), "v0.2.0");
    commit(dir.path(), "feat!: drop the old API", "a.txt");
    commit(dir.path(), "feat: add a", "b.txt");
    std::fs::write(dir.path().join(".claude/cardinal.toml"), OFF).unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &[
            "commit",
            "-qm",
            "chore: turn the convention off",
            "--no-verify",
        ],
    );
    commit(dir.path(), "Add b", "c.txt");
    let out = release(dir.path(), &["1.0.0"]);
    assert!(out.status.success(), "{out:?}");
    let today = chrono::Local::now().format("%Y-%m-%d");
    assert!(
        changelog(dir.path()).ends_with(&format!(
            "## [1.0.0] - {today}\n\n### Breaking\n\n- drop the old API\n\n### Added\n\n- add a\n\n### Changed\n\n- chore: turn the convention off\n- Add b\n"
        )),
        "{}",
        changelog(dir.path())
    );
}
