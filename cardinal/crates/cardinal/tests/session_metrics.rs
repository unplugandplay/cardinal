//! Behavior test for `cardinal tool session-metrics` on fixture transcripts: columns, deduplicated assistant messages,
//! sub-agents, typed messages, hook refusals, skills, project, tolerance to unknown lines, no message text.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The columns before `main_context_peak` (EVA-17): an archive written then is upgraded.
const OLD_COLUMNS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";
/// The columns before the advisor columns (EVA-20): an archive written then is upgraded.
const PRE_ADVISOR_COLUMNS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\tmain_context_peak\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";
const COLUMNS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\tmain_context_peak\tadvisor_models\tadvisor_calls\tadvisor_input_tokens\tadvisor_output_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// A transcript line of `type` in `cwd` at `ts`, with extra fields (`,"k":v` form).
fn line(kind: &str, cwd: &Path, ts: &str, extra: &str) -> String {
    format!(
        "{{\"type\":\"{kind}\",\"cwd\":{:?},\"timestamp\":\"{ts}\",\"sessionId\":\"x\"{extra}}}\n",
        cwd.display().to_string()
    )
}

fn typed(cwd: &Path, ts: &str, text: &str) -> String {
    line("user", cwd, ts, &format!(",\"version\":\"2.1.1\",\"gitBranch\":\"main\",\"message\":{{\"role\":\"user\",\"content\":{text:?}}}"))
}

fn assistant(cwd: &Path, ts: &str, id: &str, model: &str, blocks: &str, usage: &str) -> String {
    line("assistant", cwd, ts, &format!(",\"version\":\"2.1.2\",\"gitBranch\":\"my_issue\",\"message\":{{\"id\":\"{id}\",\"model\":\"{model}\",\"content\":[{blocks}]{usage}}}"))
}

fn usage(input: u64, output: u64, thinking: Option<u64>, read: u64, creation: u64) -> String {
    let details = thinking
        .map(|t| format!(",\"output_tokens_details\":{{\"thinking_tokens\":{t}}}"))
        .unwrap_or_default();
    format!(",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":{output},\"cache_read_input_tokens\":{read},\"cache_creation_input_tokens\":{creation}{details}}}")
}

fn refusal(cwd: &Path, ts: &str, script: &str) -> String {
    error_result(cwd, ts, &format!("PreToolUse:Bash hook error: [rust-script \\\"$CLAUDE_PROJECT_DIR\\\"/{script} check]: SECRET refusal text"))
}

fn error_result(cwd: &Path, ts: &str, text: &str) -> String {
    line("user", cwd, ts, &format!(",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"t\",\"is_error\":true,\"content\":\"{text}\"}}]}}"))
}

const SKILL: &str = "{\"type\":\"tool_use\",\"id\":\"a\",\"name\":\"Skill\",\"input\":{\"skill\":\"commit-message\",\"args\":\"SECRET args\"}}";
const BASH: &str = "{\"type\":\"tool_use\",\"id\":\"b\",\"name\":\"Bash\",\"input\":{\"command\":\"echo SECRET\"}}";
const REVIEW: &str =
    "{\"type\":\"tool_use\",\"id\":\"c\",\"name\":\"Skill\",\"input\":{\"skill\":\"code-review\"}}";
const TEXT: &str = "{\"type\":\"text\",\"text\":\"SECRET answer\"}";

/// Rows of the output keyed by session, each a map column -> value; `None` when the header is wrong.
fn rows(out: &str) -> Option<HashMap<String, HashMap<String, String>>> {
    let mut lines = out.lines();
    let header: Vec<&str> = lines.next()?.split('\t').collect();
    if header.join("\t") != COLUMNS {
        return None;
    }
    Some(
        lines
            .map(|l| {
                let row: HashMap<String, String> = header
                    .iter()
                    .map(std::string::ToString::to_string)
                    .zip(l.split('\t').map(str::to_string))
                    .collect();
                (row.get("session").cloned().unwrap_or_default(), row)
            })
            .collect(),
    )
}

/// A git repository at `dir` with one empty commit on `main`.
fn git_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    sh(dir, &["init", "-q", "-b", "main"]);
    sh(
        dir,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
}

/// The main transcript of the session `s1`, in `worktree`.
fn first_session(worktree: &Path) -> String {
    [
        typed(worktree, "2026-10-26T10:00:00.000Z", "please fix SECRET"),
        typed(worktree, "2026-10-26T10:00:05.000Z", "OK!"),
        typed(worktree, "2026-10-26T10:00:06.000Z", "1 a, 2 c"),
        typed(worktree, "2026-10-26T10:00:07.000Z", "c'est bon."),
        typed(worktree, "2026-10-26T10:00:08.000Z", "4k"),
        typed(worktree, "2026-10-26T10:00:09.000Z", "2a"),
        assistant(
            worktree,
            "2026-10-26T10:00:10.000Z",
            "m1",
            "model-a",
            SKILL,
            &usage(10, 20, Some(5), 100, 7),
        ),
        assistant(
            worktree,
            "2026-10-26T10:00:11.000Z",
            "m1",
            "model-a",
            BASH,
            &usage(10, 20, Some(5), 100, 7),
        ),
        assistant(
            worktree,
            "2026-10-26T10:00:12.000Z",
            "m1",
            "model-a",
            TEXT,
            &usage(10, 20, Some(5), 100, 7),
        ),
        assistant(
            worktree,
            "2026-10-26T10:00:20.000Z",
            "m2",
            "model-b",
            BASH,
            &usage(1, 2, None, 3, 4),
        ),
        refusal(
            worktree,
            "2026-10-26T10:00:21.000Z",
            ".claude/hooks/require_code_review.rs",
        ),
        refusal(
            worktree,
            "2026-10-26T10:00:22.000Z",
            ".claude/hooks/require_code_review.rs",
        ),
        refusal(worktree, "2026-10-26T10:00:23.000Z", "nothing"),
        refusal(
            worktree,
            "2026-10-26T10:00:23.500Z",
            "~/.rsync/notes.rst .claude/hooks/block_ai_attribution.rs",
        ),
        error_result(
            worktree,
            "2026-10-26T10:00:23.700Z",
            "PreToolUse:Bash failed: the hook error: was elsewhere",
        ),
        // the hooks became tools of `cardinal run` (`harness run` before the rename): same names as the scripts they
        // were
        error_result(
            worktree,
            "2026-10-26T10:00:23.800Z",
            "PreToolUse:Bash hook error: [cardinal run review-gate check]: SECRET refusal text",
        ),
        error_result(
            worktree,
            "2026-10-26T10:00:23.900Z",
            "PreToolUse:Bash hook error: [harness run some-new-tool]: SECRET refusal text",
        ),
        line("brand-new-type", worktree, "2026-10-26T10:00:24.000Z", ""),
        "{not json\n".to_string(),
        line(
            "assistant",
            worktree,
            "2026-10-26T10:00:25.000Z",
            ",\"message\":{\"id\":\"m9\",\"model\":\"model-a\",\"content\":[]}",
        ),
        line("attachment", worktree, "2026-10-26T11:00:00.000Z", ""),
        "{\"type\":\"ai-title\",\"sessionId\":\"x\"}\n".to_string(),
        // the metadata types of Claude Code since 2026-11, in their real shape: no timestamp, cwd nor gitBranch
        "{\"type\":\"tools-used\",\"tools\":[\"Bash\"],\"sessionId\":\"x\"}\n".to_string(),
        "{\"type\":\"relocated\",\"relocatedCwd\":\"/elsewhere\",\"sessionId\":\"x\"}\n"
            .to_string(),
        "{\"type\":\"worktree-state\",\"worktreeSession\":{},\"sessionId\":\"x\"}\n".to_string(),
        "{\"type\":\"permission-mode\",\"permissionMode\":\"auto\",\"sessionId\":\"x\"}\n"
            .to_string(),
    ]
    .concat()
}

/// The sessions of removed worktrees named apart from their branches, next to `repo` and to a repository `myrepo-docs`.
fn renamed_worktrees(tmp: &Path, repo: &Path, proj: &Path) {
    let docs = tmp.join("myrepo-docs");
    git_repo(&docs);
    std::fs::create_dir_all(repo.join("plain")).unwrap();
    for (id, cwd, branch) in [
        ("s7", tmp.join("myrepo-feat"), "issue-b"),
        ("s8", tmp.join("myrepo-docs-fix"), "docs-fix"),
        ("s9", tmp.join("myrepo-docs-x/crates"), "main"),
        ("s10", repo.join("plain-x"), "main"),
    ] {
        let session = line(
            "user",
            &cwd,
            "2026-10-26T09:00:00.000Z",
            &format!(
                ",\"gitBranch\":\"{branch}\",\"message\":{{\"role\":\"user\",\"content\":\"ok\"}}"
            ),
        );
        std::fs::write(proj.join(format!("{id}.jsonl")), session).unwrap();
    }
}

/// Builds the fixture projects directory; returns (projects dir, main repository).
fn fixture(tmp: &Path) -> (PathBuf, PathBuf) {
    let repo = tmp.join("myrepo");
    git_repo(&repo);
    let worktree = tmp.join("myrepo-my_issue");
    sh(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "my_issue",
            worktree.to_str().unwrap(),
        ],
    );
    let projects = tmp.join("projects");
    let proj = projects.join("-tmp-myrepo");
    std::fs::create_dir_all(proj.join("s1/subagents")).unwrap();
    let s1 = first_session(&worktree);
    std::fs::write(proj.join("s1.jsonl"), s1).unwrap();
    let sub = [
        line("user", &worktree, "2026-10-26T10:00:30.000Z", ",\"isSidechain\":true,\"message\":{\"role\":\"user\",\"content\":\"subagent SECRET prompt\"}"),
        assistant(&worktree, "2026-10-26T10:00:31.000Z", "m3", "model-c", REVIEW, &usage(1000, 0, None, 0, 0)),
        refusal(&worktree, "2026-10-26T10:00:32.000Z", ".claude/hooks/clippy_gate.rs"),
        "garbage\n".to_string(),
    ]
    .concat();
    std::fs::write(proj.join("s1/subagents/agent-x.jsonl"), sub).unwrap();
    let gone = Path::new("/nonexistent/elsewhere");
    let s2 = [typed(gone, "2026-10-26T09:00:00.000Z", "yes")].concat();
    std::fs::create_dir_all(projects.join("-nonexistent-elsewhere")).unwrap();
    std::fs::write(projects.join("-nonexistent-elsewhere/s2.jsonl"), s2).unwrap();
    let lib = tmp.join("lib");
    git_repo(&lib);
    sh(
        &repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            lib.to_str().unwrap(),
            "vendor/lib",
        ],
    );
    let s3 = typed(&repo.join("vendor/lib"), "2026-10-26T09:00:00.000Z", "yes");
    let removed = tmp.join("myrepo-gone_issue");
    let s4 = line(
        "user",
        &removed,
        "2026-10-26T09:00:00.000Z",
        ",\"gitBranch\":\"gone_issue\",\"message\":{\"role\":\"user\",\"content\":\"ok\"}",
    );
    std::fs::write(proj.join("s3.jsonl"), s3).unwrap();
    std::fs::write(proj.join("s4.jsonl"), s4).unwrap();
    let nested = repo.join(".claude/worktrees/gone_nested/crates");
    let s5 = line(
        "user",
        &nested,
        "2026-10-26T09:00:00.000Z",
        ",\"gitBranch\":\"gone_nested\",\"message\":{\"role\":\"user\",\"content\":\"ok\"}",
    );
    std::fs::write(proj.join("s5.jsonl"), s5).unwrap();
    renamed_worktrees(tmp, &repo, &proj);
    let inline_subagent = line(
        "assistant",
        &repo,
        "2026-10-26T09:00:01.000Z",
        &format!(",\"isSidechain\":true,\"message\":{{\"id\":\"side\",\"model\":\"model-a\",\"content\":[]{}}}", usage(9000, 0, None, 0, 0)),
    );
    let s6 = [
        assistant(
            &repo,
            "2026-10-26T09:00:00.000Z",
            "own",
            "model-a",
            TEXT,
            &usage(50, 0, None, 0, 0),
        ),
        inline_subagent,
    ]
    .concat();
    std::fs::write(proj.join("s6.jsonl"), s6).unwrap();
    (projects, repo)
}

// spec: EVA-1, EVA-2, EVA-3, EVA-4, EVA-5, EVA-6, EVA-7, EVA-9, EVA-10, EVA-16
fn row_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let (projects, _) = fixture(tmp);
    let out = tool_command(dir.join("session_metrics.rs"))
        .arg(&projects)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let rows = rows(&stdout).unwrap_or_default();
    let empty = HashMap::new();
    let s1 = rows.get("s1").unwrap_or(&empty);
    let s2 = rows.get("s2").unwrap_or(&empty);
    let s3 = rows.get("s3").unwrap_or(&empty);
    let s4 = rows.get("s4").unwrap_or(&empty);
    let s5 = rows.get("s5").unwrap_or(&empty);
    let s6 = rows.get("s6").unwrap_or(&empty);
    let s7 = rows.get("s7").unwrap_or(&empty);
    let s8 = rows.get("s8").unwrap_or(&empty);
    let s9 = rows.get("s9").unwrap_or(&empty);
    let s10 = rows.get("s10").unwrap_or(&empty);
    let col =
        |r: &HashMap<String, String>, c: &str| r.get(c).cloned().unwrap_or_else(|| "<none>".into());
    vec![
        ("the script succeeds", out.status.success()),
        ("header names every column, one row per session, none for the sub-agent", rows.len() == 10),
        ("a message repeated on 3 lines is one turn; the sub-agent adds one", col(s1, "turns") == "3"),
        ("tool calls count every tool_use block, sub-agent included", col(s1, "tool_calls") == "4"),
        ("tokens are added once per message, sub-agent included", col(s1, "input_tokens") == "1011" && col(s1, "output_tokens") == "22"),
        ("thinking tokens come from output_tokens_details", col(s1, "thinking_tokens") == "5"),
        ("cache tokens are added once per message", col(s1, "cache_read_tokens") == "103" && col(s1, "cache_creation_tokens") == "11"),
        ("the main context peak is the largest main turn, sub-agents excluded", col(s1, "main_context_peak") == "117"),
        ("a sub-agent turn written in the main transcript is not a main turn", col(s6, "main_context_peak") == "50"),
        ("a session with no assistant message has a peak of 0", col(s2, "main_context_peak") == "0"),
        ("models in order of first appearance, sub-agent included", col(s1, "models") == "model-a,model-b,model-c"),
        ("versions in order of first appearance", col(s1, "versions") == "2.1.1,2.1.2"),
        ("issues are the branches other than main", col(s1, "issues") == "my_issue"),
        ("start is the earliest timestamp, duration to the latest", col(s1, "start") == "2026-10-26T10:00:00.000Z" && col(s1, "duration_s") == "3600"),
        ("plain validations and stop answers are told apart from the rest", col(s1, "validations") == "3" && col(s1, "other_messages") == "3"),
        ("a bare yes is a validation", col(s2, "validations") == "1" && col(s2, "other_messages") == "0"),
        ("hook refusals are counted by hook, sorted, sub-agent included", col(s1, "hook_refusals") == "block_ai_attribution=1;clippy_gate=1;require_code_review=3;some_new_tool=1;unknown=1"),
        ("skills are counted by name, sorted", col(s1, "skills") == "code-review=1;commit-message=1"),
        ("a linked worktree counts with its main repository", col(s1, "project") == "myrepo"),
        ("a cwd outside git falls back to its last component", col(s2, "project") == "elsewhere"),
        ("a submodule is its own project", col(s3, "project") == "lib"),
        ("a removed issue worktree counts with its repository", col(s4, "project") == "myrepo"),
        ("a removed worktree under .claude/worktrees counts with its repository", col(s5, "project") == "myrepo"),
        ("a removed worktree named apart from its branches counts with its repository", col(s7, "project") == "myrepo"),
        ("the branch of the session names the repository of a removed worktree first", col(s8, "project") == "myrepo"),
        ("else the longest repository among the prefixes of a removed ancestor", col(s9, "project") == "myrepo-docs"),
        ("a prefix that is not the top level of a work tree is no repository", col(s10, "project") == "plain-x"),
        ("unreadable lines are counted, sub-agent included", col(s1, "unreadable_lines") == "2"),
        ("unknown line types are counted, tools-used, relocated, worktree-state and permission-mode are known", col(s1, "unknown_lines") == "1"),
        ("assistant lines without usage are missing fields, metadata lines without timestamp are not", col(s1, "missing_fields") == "1"),
        ("no text of a message, a tool input or a tool result is printed", !stdout.contains("SECRET")),
    ]
}

/// An assistant line of message `id` with a top-level `advisorModel` (none when empty) and the usage `iterations`.
fn advised(cwd: &Path, ts: &str, id: &str, advisor: &str, block: &str, iterations: &str) -> String {
    let advisor = if advisor.is_empty() {
        String::new()
    } else {
        format!(",\"advisorModel\":\"{advisor}\"")
    };
    line("assistant", cwd, ts, &format!("{advisor},\"message\":{{\"id\":\"{id}\",\"model\":\"claude-sonnet-5-5\",\"content\":[{block}],\"usage\":{{\"input_tokens\":4,\"output_tokens\":68,\"iterations\":[{iterations}]}}}}"))
}

const ADVISOR_CALL: &str =
    "{\"type\":\"server_tool_use\",\"id\":\"srv\",\"name\":\"advisor\",\"input\":{}}";
const ADVISOR_RESULT: &str = "{\"type\":\"advisor_tool_result\",\"tool_use_id\":\"srv\",\"content\":{\"type\":\"advisor_redacted_result\",\"encrypted_content\":\"SECRET advice\"}}";

/// The sessions `advised` (an advisor called in the main transcript and a sub-agent), `idle` (an advisor set, never
/// called), `none` (no advisor set) and `peaked` (an advised message whose top-level usage adds up two
/// contexts) under the returned projects directory.
fn advisor_fixture(tmp: &Path) -> PathBuf {
    let proj = tmp.join("advisor/projects/-p");
    std::fs::create_dir_all(proj.join("advised/subagents")).unwrap();
    let cwd = Path::new("/nonexistent/p");
    let iterations = "{\"type\":\"message\",\"input_tokens\":2,\"output_tokens\":26},{\"type\":\"advisor_message\",\"model\":\"claude-opus-5-5\",\"input_tokens\":21000,\"output_tokens\":479,\"cache_read_input_tokens\":100,\"cache_creation_input_tokens\":10},{\"type\":\"message\",\"input_tokens\":2,\"output_tokens\":42}";
    let opus = "claude-opus-5-5";
    let main = [
        advised(
            cwd,
            "2026-01-01T00:00:00.000Z",
            "m1",
            opus,
            ADVISOR_CALL,
            iterations,
        ),
        advised(
            cwd,
            "2026-01-01T00:00:01.000Z",
            "m1",
            opus,
            ADVISOR_RESULT,
            iterations,
        ),
        advised(
            cwd,
            "2026-01-01T00:00:02.000Z",
            "m1",
            opus,
            TEXT,
            iterations,
        ),
        advised(
            cwd,
            "2026-01-01T00:00:03.000Z",
            "m2",
            "claude-fable-5-1",
            TEXT,
            "",
        ),
    ]
    .concat();
    std::fs::write(proj.join("advised.jsonl"), main).unwrap();
    let sub_iterations = "{\"type\":\"advisor_message\",\"model\":\"claude-opus-5-5\",\"input_tokens\":1000,\"output_tokens\":21}";
    let sub = advised(
        cwd,
        "2026-01-01T00:00:04.000Z",
        "s1",
        "claude-haiku-5-5",
        ADVISOR_CALL,
        sub_iterations,
    );
    std::fs::write(proj.join("advised/subagents/agent-x.jsonl"), sub).unwrap();
    let idle = advised(cwd, "2026-01-01T00:00:00.000Z", "i1", opus, TEXT, "");
    std::fs::write(proj.join("idle.jsonl"), idle).unwrap();
    let none = advised(
        cwd,
        "2026-01-01T00:00:00.000Z",
        "n1",
        "",
        TEXT,
        "{\"type\":\"message\",\"input_tokens\":2,\"output_tokens\":26}",
    );
    std::fs::write(proj.join("none.jsonl"), none).unwrap();
    let peaked = line("assistant", cwd, "2026-01-01T00:00:00.000Z", ",\"advisorModel\":\"claude-opus-5-5\",\"message\":{\"id\":\"p1\",\"model\":\"claude-opus-5-5\",\"content\":[],\"usage\":{\"input_tokens\":4,\"cache_read_input_tokens\":28546,\"cache_creation_input_tokens\":10919,\"output_tokens\":9,\"iterations\":[{\"type\":\"message\",\"input_tokens\":2,\"cache_read_input_tokens\":9012,\"cache_creation_input_tokens\":10522},{\"type\":\"advisor_message\",\"input_tokens\":30000},{\"type\":\"message\",\"input_tokens\":2,\"cache_read_input_tokens\":19534,\"cache_creation_input_tokens\":397}]}}");
    std::fs::write(proj.join("peaked.jsonl"), peaked).unwrap();
    tmp.join("advisor/projects")
}

// spec: EVA-16, EVA-18, EVA-19
fn advisor_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let out = tool_command(dir.join("session_metrics.rs"))
        .arg(advisor_fixture(tmp))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let rows = rows(&stdout).unwrap_or_default();
    let col = |s: &str, c: &str| {
        rows.get(s)
            .and_then(|r| r.get(c))
            .cloned()
            .unwrap_or_else(|| "<none>".into())
    };
    let advisor_cells = |s: &str| {
        [
            "advisor_models",
            "advisor_calls",
            "advisor_input_tokens",
            "advisor_output_tokens",
        ]
        .map(|c| col(s, c))
    };
    vec![
        (
            "advisor models in order of first appearance, sub-agents included",
            col("advised", "advisor_models") == "claude-opus-5-5,claude-fable-5-1,claude-haiku-5-5",
        ),
        (
            "each advisor server_tool_use block is a call, sub-agents included",
            col("advised", "advisor_calls") == "2",
        ),
        (
            "an advisor call is not a tool call",
            col("advised", "tool_calls") == "0",
        ),
        (
            "advisor input tokens sum input and cache once per message, sub-agents included",
            col("advised", "advisor_input_tokens") == "22110",
        ),
        (
            "advisor output tokens are summed once per message",
            col("advised", "advisor_output_tokens") == "500",
        ),
        (
            "the advisor tokens stay out of the session's own tokens",
            col("advised", "input_tokens") == "12" && col("advised", "output_tokens") == "204",
        ),
        (
            "an advisor set and never called gives 0 calls and 0 tokens",
            advisor_cells("idle") == ["claude-opus-5-5", "0", "0", "0"].map(String::from),
        ),
        (
            "no advisor set gives empty advisor cells",
            advisor_cells("none") == ["", "", "", ""].map(String::from),
        ),
        (
            "the peak of an advised message is its largest message iteration",
            col("peaked", "main_context_peak") == "19933",
        ),
        (
            "the billed tokens of an advised message stay the top-level sums",
            col("peaked", "input_tokens") == "4" && col("peaked", "cache_read_tokens") == "28546",
        ),
        (
            "a message with no advisor iteration keeps its top-level peak",
            col("none", "main_context_peak") == "4",
        ),
        ("the advice is never printed", !stdout.contains("SECRET")),
    ]
}

// spec: EVA-12
fn shared_reader_checks(_dir: &Path) -> Vec<(&'static str, bool)> {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools");
    let uses = |name: &str| {
        std::fs::read_to_string(tools.join(name))
            .is_ok_and(|s| s.contains("use super::transcripts;"))
    };
    vec![
        (
            "audit_sources reads the transcripts through the shared module",
            uses("audit_sources.rs"),
        ),
        (
            "session_metrics reads the transcripts through the shared module",
            uses("session_metrics.rs"),
        ),
    ]
}

/// Commits `files` (path, content) in `repo` at `date` (committer and author); returns the commit hash.
fn commit_at(repo: &Path, date: &str, files: &[(&str, &str)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    sh(repo, &["add", "-A", "--force"]);
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            date,
        ])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "commit failed in {}: {}{}",
        repo.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn new_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    sh(dir, &["init", "-q", "-b", "main"]);
    dir.to_path_buf()
}

/// A `.claude/harness.toml` pinning `rev`.
fn toml(rev: &str) -> String {
    format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"  # v1\n")
}

/// A repository pinning `aaa` then `bbb` on `main`, `ddd` merged from a branch, and a linked worktree pinning `ccc`;
/// returns (repository, worktree).
fn pinned_repo(tmp: &Path) -> (PathBuf, PathBuf) {
    let pinned = new_repo(&tmp.join("pinned"));
    commit_at(
        &pinned,
        "2026-01-01T10:00:00Z",
        &[(".claude/harness.toml", &toml("aaa"))],
    );
    commit_at(
        &pinned,
        "2026-01-01T12:00:00Z",
        &[(".claude/harness.toml", &toml("bbb"))],
    );
    let worktree = tmp.join("pinned-branch");
    sh(
        &pinned,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "branch",
            worktree.to_str().unwrap(),
        ],
    );
    commit_at(
        &worktree,
        "2026-01-01T12:30:00Z",
        &[(".claude/harness.toml", &toml("ccc"))],
    );
    sh(&pinned, &["checkout", "-q", "-b", "bump"]);
    commit_at(
        &pinned,
        "2026-01-01T12:15:00Z",
        &[(".claude/harness.toml", &toml("ddd"))],
    );
    sh(&pinned, &["checkout", "-q", "main"]);
    let merge = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge",
            "bump",
        ])
        .env("GIT_AUTHOR_DATE", "2026-01-01T14:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T14:00:00Z")
        .current_dir(&pinned)
        .output()
        .unwrap();
    assert!(merge.status.success(), "merge failed");
    (pinned, worktree)
}

/// A repository pinning `old` in `.claude/harness.toml`, then `new` in `.claude/cardinal.toml`.
fn migrated_repo(tmp: &Path) -> PathBuf {
    let migrated = new_repo(&tmp.join("migrated"));
    commit_at(
        &migrated,
        "2026-01-01T10:00:00Z",
        &[(".claude/harness.toml", &toml("old"))],
    );
    std::fs::remove_file(migrated.join(".claude/harness.toml")).unwrap();
    commit_at(
        &migrated,
        "2026-01-01T12:00:00Z",
        &[(
            ".claude/cardinal.toml",
            &toml("new").replace("[harness]", "[cardinal]"),
        )],
    );
    migrated
}

// spec: EVA-8
fn revision_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tmp.join("revision");
    let (pinned, worktree) = pinned_repo(&tmp);
    let own = new_repo(&tmp.join("own"));
    commit_at(
        &own,
        "2026-01-01T10:00:00Z",
        &[
            (".claude/harness.toml", "[harness]\nsource = \".\"\n"),
            ("files/a", "1"),
        ],
    );
    let files_commit = commit_at(&own, "2026-01-01T11:00:00Z", &[("files/a", "2")]);
    commit_at(&own, "2026-01-01T11:10:00Z", &[("other", "x")]);
    let unpinned = new_repo(&tmp.join("unpinned"));
    commit_at(
        &unpinned,
        "2026-01-01T10:00:00Z",
        &[(
            ".claude/harness.toml",
            "[harness]\nsource = \"https://example.org/harness\"\n",
        )],
    );
    let migrated = migrated_repo(&tmp);
    let projects = tmp.join("projects");
    let sessions: [(&str, &Path, &str); 10] = [
        ("before_migration", &migrated, "2026-01-01T11:00:00.000Z"),
        ("after_migration", &migrated, "2026-01-01T13:00:00.000Z"),
        ("before", &pinned, "2026-01-01T09:00:00.000Z"),
        ("first", &pinned, "2026-01-01T11:00:00.000Z"),
        ("second", &pinned, "2026-01-01T13:00:00.000Z"),
        ("merged", &pinned, "2026-01-01T15:00:00.000Z"),
        ("on_branch", &worktree, "2026-01-01T13:00:00.000Z"),
        ("own", &own, "2026-01-01T11:30:00.000Z"),
        ("unpinned", &unpinned, "2026-01-01T11:00:00.000Z"),
        (
            "gone",
            Path::new("/nonexistent/gone"),
            "2026-01-01T11:00:00.000Z",
        ),
    ];
    for (id, cwd, ts) in sessions {
        let proj = projects.join(format!("-{id}"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{id}.jsonl")), typed(cwd, ts, "ok")).unwrap();
    }
    let out = tool_command(dir.join("session_metrics.rs"))
        .arg(&projects)
        .output()
        .unwrap();
    let rows = rows(&String::from_utf8_lossy(&out.stdout)).unwrap_or_default();
    let rev = |id: &str| {
        rows.get(id)
            .and_then(|r| r.get("revision"))
            .cloned()
            .unwrap_or_else(|| "<none>".into())
    };
    vec![
        ("no revision before the first pin", rev("before").is_empty()),
        (
            "the pin in force at the start of the session",
            rev("first") == "aaa",
        ),
        (
            "a later pin applies to a later session, not one merged later",
            rev("second") == "bbb",
        ),
        (
            "a pin merged into the default branch applies after the merge",
            rev("merged") == "ddd",
        ),
        (
            "a linked worktree reads the pin of the default branch",
            rev("on_branch") == "bbb",
        ),
        (
            "the harness repository itself gives its last commit touching files/",
            rev("own") == format!("self:{files_commit}"),
        ),
        (
            "a harness.toml with no rev is unpinned",
            rev("unpinned") == "unpinned",
        ),
        (
            "an unreadable project gives an empty revision",
            rev("gone").is_empty(),
        ),
        (
            "the old configuration pins before the migration",
            rev("before_migration") == "old",
        ),
        (
            "the new configuration pins after the migration",
            rev("after_migration") == "new",
        ),
    ]
}

/// The sessions `b` (two validations) and `c` (one) of the project `p` under `tmp/projects`.
fn archived_sessions(tmp: &Path) {
    let projects = tmp.join("projects/-p");
    std::fs::create_dir_all(&projects).unwrap();
    let cwd = Path::new("/nonexistent/p");
    std::fs::write(
        projects.join("b.jsonl"),
        [
            typed(cwd, "2026-01-02T00:00:00.000Z", "ok"),
            typed(cwd, "2026-01-02T00:00:01.000Z", "ok"),
        ]
        .concat(),
    )
    .unwrap();
    std::fs::write(
        projects.join("c.jsonl"),
        typed(cwd, "2026-01-01T12:00:00.000Z", "ok"),
    )
    .unwrap();
}

// spec: EVA-11, EVA-14, EVA-17
fn archive_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tmp.join("archive");
    archived_sessions(&tmp);
    let archive = tmp.join("state/deep/sessions.tsv");
    let run = || {
        tool_command(dir.join("session_metrics.rs"))
            .arg(tmp.join("projects"))
            .arg("--archive")
            .arg(&archive)
            .output()
            .unwrap()
    };
    let first = run();
    let created = std::fs::read_to_string(&archive).unwrap_or_default();
    let old_row = |id: &str, start: &str| {
        format!("{id}\tp\t\t\t\t\t{start}\t0\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t\t\t\t\t9\t0\t0\t0\t0\n")
    };
    std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
    let resolved =
        "c\tkeep\t\trrr\t\t\t2026-01-01T12:00:00.000Z\t0\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t\t\t\t\t9\t0\t0\t0\t0\n";
    std::fs::write(
        &archive,
        format!(
            "{COLUMNS}\n{}{}{resolved}",
            old_row("b", "2026-01-02T00:00:00.000Z"),
            old_row("a", "2026-01-01T00:00:00.000Z")
        ),
    )
    .unwrap();
    let merged = run();
    let text = std::fs::read_to_string(&archive).unwrap_or_default();
    let ids: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|l| l.split('\t').next().unwrap_or_default())
        .collect();
    let merged_rows = rows(&text).unwrap_or_default();
    let cell = |id: &str, c: &str| {
        merged_rows
            .get(id)
            .and_then(|r| r.get(c))
            .cloned()
            .unwrap_or_default()
    };
    let b_validations = cell("b", "validations");
    let before_peak = format!(
        "{OLD_COLUMNS}\nold\tp\t\t\t\t\t2026-01-01T00:00:00.000Z\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t9\t0\t0\t0\t0\n"
    );
    std::fs::write(&archive, before_peak).unwrap();
    let upgraded = run();
    let upgraded_text = std::fs::read_to_string(&archive).unwrap_or_default();
    let upgraded_rows = rows(&upgraded_text).unwrap_or_default();
    let old = upgraded_rows.get("old");
    let foreign = "session\tsomething_else\na\tx\n";
    std::fs::write(&archive, foreign).unwrap();
    let refused = run();
    vec![
        (
            "the archive and its missing parent directories are created",
            first.status.success() && created.lines().count() == 3,
        ),
        (
            "the rows are still printed with --archive",
            String::from_utf8_lossy(&merged.stdout).lines().count() == 3,
        ),
        (
            "one header, sessions kept, replaced or added, sorted by start",
            merged.status.success() && text.starts_with(COLUMNS) && ids == ["a", "c", "b"],
        ),
        (
            "an archived session is replaced by its new row",
            b_validations == "2",
        ),
        (
            "a resolved project and revision survive a new row that lost them",
            cell("c", "project") == "keep"
                && cell("c", "revision") == "rrr"
                && cell("c", "validations") == "1",
        ),
        (
            "an archive written before main_context_peak is upgraded, its rows given an empty peak",
            upgraded.status.success()
                && upgraded_rows.len() == 3
                && old
                    .and_then(|r| r.get("main_context_peak"))
                    .is_some_and(std::string::String::is_empty)
                && old
                    .and_then(|r| r.get("validations"))
                    .is_some_and(|v| v == "9"),
        ),
        (
            "an archive with another header is refused and left unchanged",
            !refused.status.success()
                && std::fs::read_to_string(&archive).unwrap_or_default() == foreign,
        ),
    ]
}

// spec: EVA-20
fn advisor_archive_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tmp.join("advisor_archive");
    archived_sessions(&tmp);
    let archive = tmp.join("sessions.tsv");
    let before_advisor = format!(
        "{PRE_ADVISOR_COLUMNS}\nold\tp\t\t\t\t\t2026-01-01T00:00:00.000Z\t0\t0\t0\t0\t0\t0\t0\t0\t7\t\t\t9\t0\t0\t0\t0\n"
    );
    std::fs::write(&archive, before_advisor).unwrap();
    let upgraded = tool_command(dir.join("session_metrics.rs"))
        .arg(tmp.join("projects"))
        .arg("--archive")
        .arg(&archive)
        .output()
        .unwrap();
    let rows = rows(&std::fs::read_to_string(&archive).unwrap_or_default()).unwrap_or_default();
    let old = rows.get("old");
    vec![(
        "an archive written before the advisor columns is upgraded, its rows given empty advisor cells",
        upgraded.status.success()
            && rows.len() == 3
            && old.is_some_and(|r| {
                ["advisor_models", "advisor_calls", "advisor_input_tokens", "advisor_output_tokens"]
                    .iter()
                    .all(|c| r.get(*c).is_some_and(String::is_empty))
                    && r.get("main_context_peak").is_some_and(|v| v == "7")
                    && r.get("validations").is_some_and(|v| v == "9")
            }),
    )]
}

// spec: EVA-15
fn default_root_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let config = tmp.join("default-root/config");
    let home = tmp.join("default-root/home");
    let session = |root: &Path, name: &str| {
        let proj = root.join("projects/-tmp-p");
        std::fs::create_dir_all(&proj).unwrap();
        let text = typed(tmp, "2026-10-26T09:00:00.000Z", "hi");
        std::fs::write(proj.join(format!("{name}.jsonl")), text).unwrap();
    };
    session(&config, "from_config");
    session(&home.join(".claude"), "from_home");
    let sessions = |cmd: &mut std::process::Command| {
        let out = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut names: Vec<String> = rows(&stdout).unwrap_or_default().into_keys().collect();
        names.sort();
        (out.status.success(), names)
    };
    let set = sessions(
        tool_command(dir.join("session_metrics.rs"))
            .env("CLAUDE_CONFIG_DIR", &config)
            .env("HOME", &home),
    );
    let unset = sessions(
        tool_command(dir.join("session_metrics.rs"))
            .env("CLAUDE_CONFIG_DIR", "")
            .env("HOME", &home),
    );
    let neither = tool_command(dir.join("session_metrics.rs"))
        .env_remove("CLAUDE_CONFIG_DIR")
        .env_remove("HOME")
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&neither.stderr).into_owned();
    vec![
        (
            "the default projects directory is the one of CLAUDE_CONFIG_DIR",
            set == (true, vec!["from_config".to_string()]),
        ),
        (
            "an empty CLAUDE_CONFIG_DIR falls back to the home directory",
            unset == (true, vec!["from_home".to_string()]),
        ),
        (
            "with neither variable, the error names both",
            !neither.status.success()
                && error.contains("CLAUDE_CONFIG_DIR")
                && error.contains("HOME"),
        ),
    ]
}

#[test]
fn session_metrics() {
    let dir = std::path::PathBuf::from("/tools");
    let scratch = tempfile::tempdir().unwrap();
    let tmp = scratch.path().to_path_buf();
    let checks: Vec<_> = row_checks(&dir, &tmp)
        .into_iter()
        .chain(revision_checks(&dir, &tmp))
        .chain(archive_checks(&dir, &tmp))
        .chain(advisor_checks(&dir, &tmp))
        .chain(advisor_archive_checks(&dir, &tmp))
        .chain(shared_reader_checks(&dir))
        .chain(default_root_checks(&dir, &tmp))
        .collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
    assert!(!failed, "a check failed: see the FAIL lines");
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
