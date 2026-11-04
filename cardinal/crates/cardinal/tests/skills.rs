//! Structural check of the project skills: frontmatter is valid, `name` matches the directory,
//! and every skill (`skill:<name>`), repo doc (`` `AGENTS.md` ``, `` `docs/….md` ``, `` `.claude/rules/….md` ``) or section of `AGENTS.md`
//! (`` `AGENTS.md` section "<heading>" ``) a skill references actually exists, as does every file of its own
//! directory it links (`[x](details.md)`); every markdown file of a skill is checked, not only `SKILL.md`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const BUILT_IN_ROOT_DOCS: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// Passages a skill must keep, as (skill, passage, why).
const REQUIRED_PASSAGES: &[(&str, &str, &str)] = &[
    (
        "create-issue",
        "attach it to a milestone",
        "an issue says which milestone of the roadmap it serves",
    ),
    (
        "create-issue",
        "## Cut a shaped milestone into issues",
        "a shaped milestone turns into issues sized for the workflow",
    ),
    (
        "prioritize-issues",
        "never reorder the milestones",
        "the order of the milestones is the user's call",
    ),
    (
        "issue-workflow",
        "`close-issue` sets that milestone `done` itself",
        "the tool, not the agent, says when a milestone is finished (CLS-8)",
    ),
    (
        "create-issue",
        "commit the issue files alone",
        "the user no longer types \"commit\" after each new issue",
    ),
    (
        "create-issue",
        "committed with that deliverable",
        "an issue found inside another deliverable is not committed on its own",
    ),
    (
        "prioritize-issues",
        "commit the re-prioritized issue files alone",
        "the validated order is committed without a further ask",
    ),
    (
        "audit-instructions",
        "rule-breaches --summary",
        "the rules broken often feed the audit as hook candidates",
    ),
    (
        "audit-instructions",
        "skill:evaluate-harness",
        "the audit points to the skill that measures whether the rules paid off",
    ),
    (
        "audit-instructions",
        "always-on rule last",
        "the forms are ordered by what they cost the context, always-on text last",
    ),
    (
        "audit-instructions",
        "its `why` names what it replaces or trims",
        "an always-on rule pays for itself by cutting something else",
    ),
    (
        "audit-instructions",
        "a hint after a recurring error",
        "a hook does more than block",
    ),
    (
        "audit-instructions",
        "proposing nothing is a valid outcome",
        "the audit does not grow the harness without evidence",
    ),
    (
        "audit-instructions",
        "nor what one session only shows",
        "a single occurrence is no evidence",
    ),
    (
        "audit-instructions",
        "## Review the existing instructions",
        "the shipped instructions are reviewed for contradictions, duplicates and stale rules",
    ),
    (
        "audit-instructions",
        "`issues/done/`",
        "a rule kept for a past incident is read with its issue before it goes",
    ),
    (
        "evaluate-harness",
        "no change is a valid conclusion",
        "the measure does not invent a change to make",
    ),
    (
        "evaluate-harness",
        "cardinal run cardinal-report",
        "the tables come from the report script, not from the agent's arithmetic",
    ),
    (
        "evaluate-harness",
        "cardinal run fixed-context",
        "the fixed context of each revision is measured",
    ),
    (
        "evaluate-harness",
        "cardinal run delivered-quality",
        "the quality axis reads each repository with done issues",
    ),
    (
        "evaluate-harness",
        "--anonymous",
        "a report that lands in a shared file names no project",
    ),
    (
        "evaluate-harness",
        "cardinal run cardinal-changes",
        "the recorded changes of the harness get a verdict",
    ),
    (
        "evaluate-harness",
        "`no data` are no conclusion",
        "a verdict where something else changed too is not read as an effect",
    ),
    (
        "evaluate-harness",
        "never a pass or a fail",
        "numbers become targets once shown: trends only",
    ),
    (
        "evaluate-harness",
        "--by setup",
        "the setups (an executor alone, advised, a strong model alone) are compared on the same harness",
    ),
    (
        "evaluate-harness",
        "observational",
        "a setup is chosen per task, so its comparison is no A/B",
    ),
    (
        "issue-workflow",
        "## Autonomous mode",
        "section defining the autonomous mode",
    ),
    (
        "issue-workflow",
        "autonomous (default)",
        "plan validation asks autonomous or per-deliverable validation",
    ),
    (
        "issue-workflow",
        "recorded first in `## Decisions`",
        "mode recorded in the issue so it survives a /clear",
    ),
    (
        "issue-workflow",
        "Plan: proposed",
        "the plan is written in the issue before its validation",
    ),
    (
        "issue-workflow",
        "set `Plan: validated by <name> <<email>> on <YYYY-MM-DD>`",
        "the validation is recorded, with who and when, so a resume skips the stop",
    ),
    (
        "issue-workflow",
        "`Validated-by: <name> <<email>>`",
        "the commit of a deliverable a person validated names that person",
    ),
    (
        "issue-workflow",
        "`Plan: validated`, signed or not",
        "a resume still reads the bare marker of an issue planned before the signature",
    ),
    (
        "issue-workflow",
        "`(autonomous)`",
        "an autonomous deliverable never claims a human validation",
    ),
    (
        "issue-workflow",
        "`1 a, 2 c`",
        "questions answered by number and option letter",
    ),
    (
        "issue-workflow",
        "`Code review: <n> findings`",
        "the count of findings opens the line delivered-quality reads",
    ),
    (
        "issue-workflow",
        "`red: ABC-3, ABC-4: <failure>`",
        "the red line names the traced ids delivered-quality reads",
    ),
    (
        "issue-workflow",
        "## Advisor",
        "the moments where an advisor's advice pays are named",
    ),
    (
        "issue-workflow",
        "When an advisor tool is available",
        "the advisor section stays neutral when none is configured",
    ),
    (
        "issue-workflow",
        "still red after a fix",
        "the advisor is consulted when stuck, not only on the plan",
    ),
    (
        "issue-workflow",
        "never replaces a review or a stop",
        "the advisor never lifts the reviews nor the stop of step 2",
    ),
    (
        "issue-workflow",
        "## Drive a queue",
        "section chaining a few issues from one session",
    ),
    (
        "issue-workflow",
        "3 issues at most",
        "a queue stays small, the context of the session grows with it",
    ),
    (
        "issue-workflow",
        "grouped, never lifted",
        "a queue groups the plan stops of its issues, it never skips them",
    ),
    (
        "issue-workflow",
        "`Rests on:`",
        "each plan of a queue says what it rests on, for the overlap and stale plan checks",
    ),
    (
        "issue-workflow",
        "stale plan check",
        "before each issue of a queue, the work already done is crossed with its plan",
    ),
    (
        "issue-workflow",
        "no orchestrator subagent",
        "the main loop drives the queue, so the reviews, gates and claims work as for one issue (ADR 0026)",
    ),
    (
        "update-docs",
        "skill:commit-message",
        "a commit of the docs goes through the checker of commit-message",
    ),
    (
        "write-spec",
        "just spec-coverage",
        "the bootstrap of a spec starts from the report of the uncovered test files",
    ),
    (
        "write-spec",
        "## Bootstrap the spec of an existing capability",
        "section reading a spec back from the code and the tests",
    ),
];

/// Strings of `key = ["a", "b"]` in the `[project]` table of `.claude/cardinal.toml`; empty when absent.
fn config_list(root: &std::path::Path, key: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(".claude/cardinal.toml")).unwrap_or_default();
    let table: toml::Table = text.parse().unwrap_or_default();
    let list = table
        .get("project")
        .and_then(|p| p.get(key))
        .or_else(|| table.get(key));
    list.and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_doc_path(s: &str, root_docs: &[String]) -> bool {
    let path_chars = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_./-".contains(c));
    path_chars
        && (BUILT_IN_ROOT_DOCS.contains(&s)
            || root_docs.iter().any(|d| d == s)
            || ((s.starts_with("docs/") || s.starts_with(".claude/rules/"))
                && Path::new(s).extension().is_some_and(|e| e == "md")))
}

/// The broken references of the text of the skill `name` in `dir`: unknown skills, missing doc files, broken links.
fn reference_failures(
    name: &str,
    text: &str,
    dir: &Path,
    root: &Path,
    root_docs: &[String],
) -> Vec<String> {
    let mut failures = Vec::new();
    // `skill:<name>` references
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || ":-".contains(c))) {
        if let Some(target) = word.strip_prefix("skill:").filter(|t| !t.is_empty()) {
            if !root
                .join(".claude/skills")
                .join(target)
                .join("SKILL.md")
                .exists()
            {
                failures.push(format!("{name}: references unknown skill '{target}'"));
            }
        }
    }
    // `path` references to repo docs (odd segments between backticks)
    for path in text
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|s| is_doc_path(s, root_docs))
    {
        if !root.join(path).exists() {
            failures.push(format!("{name}: references missing file '{path}'"));
        }
    }
    // relative markdown links outside code spans, resolved in the skill's directory
    let prose = text.split('`').step_by(2).collect::<Vec<_>>().join(" ");
    for link in prose
        .split("](")
        .skip(1)
        .filter_map(|t| t.split(')').next())
    {
        // drop the title and the anchor; a target with a scheme (`https:`, `mailto:`) is not a file
        let target = link.split([' ', '#']).next().unwrap_or_default();
        if !target.is_empty() && !target.contains(':') && !dir.join(target).exists() {
            failures.push(format!("{name}: links the missing file '{target}'"));
        }
    }
    failures
}

#[test]
fn skills() {
    let root = repo_root();
    let skills = root.join(".claude/skills");
    let root_docs = config_list(&root, "root_docs");
    let mut failures = Vec::new();
    let mut visited = Vec::new();
    // headings of `AGENTS.md` outside fenced blocks; `None` when it cannot be read (its citations report that)
    let headings: Option<Vec<String>> =
        fs::read_to_string(root.join("AGENTS.md"))
            .ok()
            .map(|agents| {
                let mut fenced = false;
                agents
                    .lines()
                    .filter(|l| {
                        fenced ^= l.starts_with("```");
                        !fenced && l.starts_with('#')
                    })
                    .map(|l| l.trim_start_matches('#').trim().to_string())
                    .collect()
            });

    let mut dirs: Vec<_> = fs::read_dir(&skills)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    dirs.sort_by_key(fs::DirEntry::file_name);

    for entry in dirs {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(text) = fs::read_to_string(entry.path().join("SKILL.md")) else {
            failures.push(format!("{name}: SKILL.md missing"));
            continue;
        };
        let mut lines = text.lines();
        if lines.next() != Some("---") {
            failures.push(format!("{name}: no frontmatter"));
        }
        let front: Vec<&str> = lines.by_ref().take_while(|l| *l != "---").collect();
        if !front.contains(&format!("name: {name}").as_str()) {
            failures.push(format!("{name}: frontmatter name != directory"));
        }
        let long_description = front.iter().any(|l| {
            l.strip_prefix("description: ")
                .is_some_and(|d| d.len() >= 40)
        });
        if !long_description {
            failures.push(format!("{name}: description missing or too short"));
        }

        // the references below are checked in every markdown file of the skill, not only `SKILL.md`
        let mut extra: Vec<_> = fs::read_dir(entry.path())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md") && !p.ends_with("SKILL.md"))
            .collect();
        extra.sort();
        let text = extra.iter().fold(text, |acc, p| {
            acc + "\n" + &fs::read_to_string(p).unwrap_or_default()
        });

        failures.extend(reference_failures(
            &name,
            &text,
            &entry.path(),
            &root,
            &root_docs,
        ));
        // `AGENTS.md` section "<heading>" references, wherever the text wraps
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for cited in flat.split("`AGENTS.md` section \"").skip(1) {
            let heading = cited.split('"').next().unwrap_or_default();
            if headings
                .as_ref()
                .is_some_and(|h| !h.iter().any(|h| h == heading))
            {
                failures.push(format!(
                    "{name}: cites the missing section '{heading}' of `AGENTS.md`"
                ));
            }
        }
        for (skill, passage, why) in REQUIRED_PASSAGES.iter().filter(|(s, _, _)| *s == name) {
            if !flat.contains(passage) {
                failures.push(format!("{skill}: missing '{passage}' ({why})"));
            }
        }
        visited.push(name.clone());
        println!("checked {name}");
    }

    for (skill, passage, _) in REQUIRED_PASSAGES
        .iter()
        .filter(|(s, _, _)| !visited.iter().any(|v| v == s))
    {
        failures.push(format!("{skill}: skill missing, cannot hold '{passage}'"));
    }

    for f in &failures {
        println!("FAIL - {f}");
    }
    assert!(
        failures.is_empty(),
        "{} failure(s): see the FAIL lines",
        failures.len()
    );
}

/// The rules of `.claude/rules/` whose front matter holds no `paths`: Claude Code loads them in every session.
fn always_on_rules(root: &Path) -> Vec<String> {
    let mut rules: Vec<String> = fs::read_dir(root.join(".claude/rules"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .filter(|p| {
            let text = fs::read_to_string(p).unwrap_or_default();
            let mut lines = text.lines();
            lines.next() != Some("---")
                || !lines
                    .take_while(|l| *l != "---")
                    .any(|l| l.starts_with("paths:"))
        })
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    rules.sort();
    rules
}

fn repo_root() -> std::path::PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// The skill directories of `.claude/skills/`, as (name, path), sorted; a stray file is not a skill.
fn skill_dirs(root: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut dirs: Vec<_> = fs::read_dir(root.join(".claude/skills"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    dirs.sort();
    dirs
}

/// An always-on rule is already in the context of every session: a skill that points to it pays its line for nothing.
#[test]
fn skills_do_not_point_to_an_always_on_rule() {
    let root = repo_root();
    let rules = always_on_rules(&root);
    assert!(!rules.is_empty(), "no always-on rule in .claude/rules/");
    let mut failures = Vec::new();
    for (_, dir) in skill_dirs(&root) {
        for file in fs::read_dir(dir).unwrap().flatten() {
            let text = fs::read_to_string(file.path()).unwrap_or_default();
            for rule in rules
                .iter()
                .filter(|r| text.contains(&format!(".claude/rules/{r}")))
            {
                failures.push(format!("{}: points to {rule}", file.path().display()));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

// spec: FEA-29
/// The language of the conversation is said once, in the always-on rule of no feature, not in each skill nor in the
/// rule of the report style, which a project may turn off.
#[test]
fn the_user_language_is_said_once_in_the_always_on_rule() {
    let root = repo_root();
    let rule = fs::read_to_string(root.join("files/.claude/rules/user-language.md")).unwrap();
    assert!(
        rule.contains("`user_language`"),
        "the rule names no `user_language`"
    );
    let style =
        fs::read_to_string(root.join("files/.claude/rules/reports-to-the-user.md")).unwrap();
    assert!(
        !style.contains("`user_language`"),
        "the report style restates the language"
    );
    let features = fs::read_to_string(root.join("features.toml")).unwrap();
    assert!(
        !features.contains("user-language.md"),
        "a feature holds the language rule"
    );
    let copies: Vec<String> = skill_dirs(&root)
        .into_iter()
        .filter(|(_, dir)| {
            fs::read_to_string(dir.join("SKILL.md"))
                .unwrap_or_default()
                .contains("`user_language`")
        })
        .map(|(name, _)| name)
        .collect();
    assert!(
        copies.is_empty(),
        "skills restating the language: {copies:?}"
    );
}

/// Passages a skill must not hold, as (skill, passage, why): a rule said elsewhere, or one Claude Code's own prompt holds.
const DROPPED_PASSAGES: &[(&str, &str, &str)] = &[
    (
        "issue-workflow",
        "never skipped",
        "step 3.5 says the review is mandatory once, the commit gate enforces it",
    ),
    (
        "issue-workflow",
        "Step 6 cannot start before the review",
        "step 3.5 says the review is mandatory once",
    ),
    (
        "issue-workflow",
        "If you did not invoke the review",
        "step 3.5 says the review is mandatory once",
    ),
    (
        "issue-workflow",
        "(`done`, `check | result`",
        "the autonomous mode points to the summary of step 3.6 instead of listing its fields",
    ),
    (
        "issue-workflow",
        "Anything outward-facing",
        "Claude Code's prompt holds it; the autonomous mode keeps the push ban",
    ),
    (
        "issue-workflow",
        "re-litigate",
        "Claude Code's prompt holds it",
    ),
    (
        "issue-workflow",
        "don't enumerate every option",
        "Claude Code's prompt holds it",
    ),
    (
        "commit-message",
        "Co-Authored-By",
        "`.claude/rules/no-ai-attribution.md` holds the attribution rule, the checker enforces it",
    ),
    (
        "issue-workflow",
        "git add -A",
        "commit-message holds the ban, the callers commit through it",
    ),
    (
        "create-issue",
        "git add -A",
        "commit-message holds the ban, the callers commit through it",
    ),
    (
        "prioritize-issues",
        "git add -A",
        "commit-message holds the ban, the callers commit through it",
    ),
    (
        "create-issue",
        "issues/queue.md",
        "the rank of each issue file replaced the format 4 queue (ADR 0019)",
    ),
    (
        "prioritize-issues",
        "issues/queue.md",
        "the rank of each issue file replaced the format 4 queue (ADR 0019)",
    ),
];

#[test]
fn skills_drop_the_restated_rules() {
    let root = repo_root();
    let mut failures = Vec::new();
    for (skill, passage, why) in DROPPED_PASSAGES {
        let Ok(text) = fs::read_to_string(root.join(".claude/skills").join(skill).join("SKILL.md"))
        else {
            failures.push(format!("{skill}: skill missing, cannot drop '{passage}'"));
            continue;
        };
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.contains(passage) {
            failures.push(format!("{skill}: holds '{passage}' ({why})"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The skills read the `roadmap` documents of the seeded configuration: a new project must get them from `seed/`.
#[test]
fn seeded_roadmap_is_seeded() {
    let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../seed");
    let roadmap = config_list(&seed, "roadmap");
    assert!(
        !roadmap.is_empty(),
        "the seeded configuration names no roadmap"
    );
    for doc in roadmap {
        assert!(
            seed.join(&doc).is_file(),
            "the seeded roadmap '{doc}' is not seeded"
        );
    }
}

// spec: FEA-12, FEA-13
/// The rule against AI attribution lives in the files of the feature `no-ai-attribution`, so a project that turns it off
/// loads it nowhere.
#[test]
fn the_attribution_rule_lives_in_its_feature_alone() {
    let root = repo_root();
    let features: toml::Table = fs::read_to_string(root.join("features.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let files: Vec<&str> = features["no-ai-attribution"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(!files.is_empty(), "{features:?}");
    let holds = |text: &str| text.contains("Co-Authored-By") && text.contains("attribution");
    assert!(files
        .iter()
        .any(|f| holds(&fs::read_to_string(root.join("files").join(f)).unwrap())));
    assert!(!holds(
        &fs::read_to_string(root.join("files/AGENTS.md")).unwrap()
    ));
}

// spec: FEA-20
/// The commit convention lives in the files of the feature `conventional-commits`, not in `AGENTS.md` nor in the
/// procedure of `skill:commit-message` that the other skills call, so a project that turns it off keeps its own.
#[test]
fn the_commit_convention_lives_in_its_feature_alone() {
    let root = repo_root();
    let features: toml::Table = fs::read_to_string(root.join("features.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let files: Vec<&str> = features["conventional-commits"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    let holds = |text: &str| text.contains("Conventional Commits") && text.contains("`refactor`");
    assert!(files
        .iter()
        .any(|f| holds(&fs::read_to_string(root.join("files").join(f)).unwrap())));
    for shared in ["AGENTS.md", ".claude/skills/commit-message/SKILL.md"] {
        assert!(!files.contains(&shared), "{shared}");
        assert!(
            !holds(&fs::read_to_string(root.join("files").join(shared)).unwrap()),
            "{shared}"
        );
    }
}

// spec: FEA-16, FEA-35, RDH-6
/// Each hook-only feature is the hook entries that run its tool, as `sync` prints them, and no file nor `needs`.
#[test]
fn the_hook_only_features_are_their_hook_entries() {
    let root = repo_root();
    let features: toml::Table = fs::read_to_string(root.join("features.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("entries/.claude/settings.json")).unwrap(),
    )
    .unwrap();
    for (feature, commands) in [
        ("lint-gate", &[""][..]),
        ("format-code", &[""]),
        ("language-hint", &[""]),
        ("read-hint", &[""]),
        ("workspace-lock", &[" check", " start", " release"]),
    ] {
        let commands: Vec<String> = commands
            .iter()
            .map(|sub| format!("cardinal run {feature}{sub}"))
            .collect();
        let mut declared = Vec::new();
        for (event, groups) in settings["hooks"].as_object().unwrap() {
            for group in groups.as_array().unwrap() {
                for hook in group["hooks"].as_array().unwrap() {
                    let command = hook["command"].as_str().unwrap_or_default();
                    if commands.iter().any(|c| c == command) {
                        let matcher = group["matcher"]
                            .as_str()
                            .map(|m| format!("{m} "))
                            .unwrap_or_default();
                        declared.push(format!("hook {event} {matcher}{command}"));
                    }
                }
            }
        }
        assert_eq!(declared.len(), commands.len(), "{feature}: {declared:?}");
        let table = features
            .get(feature)
            .and_then(|t| t.as_table())
            .unwrap_or_else(|| panic!("features.toml declares no {feature}"));
        assert!(table.get("files").is_none(), "{feature}: {table:?}");
        assert!(table.get("needs").is_none(), "{feature}: {table:?}");
        let mut entries: Vec<&str> = table["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap())
            .collect();
        entries.sort_unstable();
        declared.sort_unstable();
        assert_eq!(entries, declared, "{feature}");
    }
}

// spec: FEA-27, FEA-30, FEA-31
/// The features made of managed files only are every file of their directories and the files they name, and no entry.
#[test]
fn the_file_only_features_are_their_files() {
    let root = repo_root();
    let features: toml::Table = fs::read_to_string(root.join("features.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let under = |dirs: &[&str]| -> Vec<String> {
        let mut files: Vec<String> = dirs
            .iter()
            .flat_map(|dir| {
                walkdir(&root.join("files").join(dir))
                    .into_iter()
                    .map(move |f| format!("{dir}/{f}"))
            })
            .collect();
        files.sort();
        files
    };
    for (feature, expected) in [
        (
            "harness-measure",
            under(&[
                ".claude/skills/audit-instructions",
                ".claude/skills/evaluate-harness",
            ]),
        ),
        (
            "report-style",
            vec![".claude/rules/reports-to-the-user.md".to_string()],
        ),
        ("github-ci", under(&[".github"])),
        (
            "issue-files",
            [
                under(&[
                    ".claude/skills/create-issue",
                    ".claude/skills/issue-workflow",
                    ".claude/skills/prioritize-issues",
                ]),
                vec!["cardinal-issues.just".to_string()],
            ]
            .concat(),
        ),
        (
            "specs",
            [
                under(&["specs", ".claude/skills/write-spec"]),
                vec!["cardinal-specs.just".to_string()],
            ]
            .concat(),
        ),
    ] {
        let table = features
            .get(feature)
            .and_then(|t| t.as_table())
            .unwrap_or_else(|| panic!("features.toml declares no {feature}"));
        assert!(table.get("entries").is_none(), "{feature}: {table:?}");
        let mut files: Vec<String> = table["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap().to_string())
            .collect();
        files.sort();
        let mut expected = expected;
        expected.sort();
        assert!(!expected.is_empty(), "{feature}");
        assert_eq!(files, expected, "{feature}");
        if matches!(feature, "issue-files" | "specs") {
            assert!(table.get("needs").is_none(), "{feature}: {table:?}");
        }
    }
}

/// The files under `dir`, relative to it, with `/` separators.
fn walkdir(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            files.extend(walkdir(&path).into_iter().map(|f| format!("{name}/{f}")));
        } else {
            files.push(name);
        }
    }
    files
}

// spec: FEA-23
/// The feature `review-gate` is every hook entry running the gate and the deny rules of its waiver, as `sync` prints
/// them, with the recipe of the waiver.
#[test]
fn the_review_gate_feature_is_its_hooks_deny_rules_and_waiver_recipe() {
    let root = repo_root();
    let features: toml::Table = fs::read_to_string(root.join("features.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("entries/.claude/settings.json")).unwrap(),
    )
    .unwrap();
    // the deny rules of the waiver (REV-19), read from the entries so that a new one is declared too
    let mut declared: Vec<String> = settings["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| rule.as_str().unwrap())
        .filter(|rule| rule.contains("waive-review") || rule.contains("claude-review-waived"))
        .map(|rule| format!("deny {rule}"))
        .collect();
    for (event, groups) in settings["hooks"].as_object().unwrap() {
        for group in groups.as_array().unwrap() {
            for hook in group["hooks"].as_array().unwrap() {
                let command = hook["command"].as_str().unwrap();
                if command.starts_with("cardinal run review-gate ") {
                    let matcher = group["matcher"]
                        .as_str()
                        .map(|m| format!("{m} "))
                        .unwrap_or_default();
                    declared.push(format!("hook {event} {matcher}{command}"));
                }
            }
        }
    }
    assert_eq!(declared.len(), 5, "{declared:?}");
    let table = features
        .get("review-gate")
        .and_then(|t| t.as_table())
        .expect("features.toml declares no review-gate");
    let strings = |key: &str| -> Vec<String> {
        let mut list: Vec<String> = table[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        list.sort();
        list
    };
    declared.sort();
    assert_eq!(strings("entries"), declared);
    assert_eq!(strings("files"), ["cardinal-review.just"]);
}

// spec: FEA-33
/// The seeded `check` runs `check-specs` when `just` has it, and succeeds without `cardinal-specs.just`.
#[test]
fn the_seeded_check_runs_check_specs_only_when_present() {
    let root = repo_root();
    let check = |specs: Option<&str>| {
        let dir = tempfile::tempdir().unwrap();
        fs::copy(root.join("seed/justfile"), dir.path().join("justfile")).unwrap();
        fs::copy(
            root.join("files/cardinal.just"),
            dir.path().join("cardinal.just"),
        )
        .unwrap();
        if let Some(recipe) = specs {
            fs::write(dir.path().join("cardinal-specs.just"), recipe).unwrap();
        }
        std::process::Command::new("just")
            .arg("check")
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(check(None), "without cardinal-specs.just");
    assert!(check(Some("check-specs:\n    true\n")));
    assert!(!check(Some("check-specs:\n    false\n")));
}

// spec: FEA-32
/// The CI step of the specs runs `just check-specs` only when the project has `cardinal-specs.just`.
#[test]
fn the_ci_checks_the_specs_only_when_the_recipe_exists() {
    let workflow =
        fs::read_to_string(repo_root().join("files/.github/workflows/cardinal.yml")).unwrap();
    let line = workflow
        .lines()
        .find(|l| l.contains("just check-specs"))
        .expect("the CI runs just check-specs");
    let guard = "if [ ! -f cardinal-specs.just ]; then echo";
    let (before, after) = line.split_once(guard).expect(line);
    assert!(!before.contains("just check-specs"), "{line}");
    assert!(
        after.contains("elif") && after.contains("then just check-specs"),
        "{line}"
    );
}

// spec: FEA-34
/// `issue-workflow` keys its spec steps, writing one and tracing a test to it, on `specs/README.md`, which the
/// feature `specs` leaves out.
#[test]
fn the_issue_workflow_writes_specs_only_with_the_specs_readme() {
    let skill =
        fs::read_to_string(repo_root().join("files/.claude/skills/issue-workflow/SKILL.md"))
            .unwrap();
    let flat = skill.split_whitespace().collect::<Vec<_>>().join(" ");
    for passage in [
        "**Spec**, when the issue changes what a capability does, in a project that has `specs/README.md`",
        "When the capability has a spec and the project has `specs/README.md`, the test proves",
    ] {
        assert!(flat.contains(passage), "issue-workflow lacks: {passage}");
    }
}

// spec: FEA-19
/// The recipes of each part a feature may leave out live in a file of their own, which `cardinal.just` imports
/// optionally: `just` loads every recipe with all the files, and the others with any one missing.
#[test]
fn the_recipes_of_each_part_live_in_a_file_imported_optionally() {
    const PARTS: [(&str, &[&str]); 5] = [
        (
            "cardinal.just",
            &[
                "open-worktree",
                "close-worktree",
                "release",
                "check-cardinal",
            ],
        ),
        ("cardinal-commits.just", &["check-commit", "check-commits"]),
        (
            "cardinal-issues.just",
            &["next-issue", "close-issue", "check-issue"],
        ),
        ("cardinal-specs.just", &["check-specs", "spec-coverage"]),
        ("cardinal-review.just", &["waive-review"]),
    ];
    let files = repo_root().join("files");
    let core = fs::read_to_string(files.join("cardinal.just")).unwrap();
    let recipes = |missing: Option<&str>| {
        let dir = tempfile::tempdir().unwrap();
        for (file, _) in PARTS.iter().filter(|(f, _)| Some(*f) != missing) {
            fs::copy(files.join(file), dir.path().join(file)).unwrap();
        }
        fs::write(dir.path().join("justfile"), "import 'cardinal.just'\n").unwrap();
        let out = std::process::Command::new("just")
            .arg("--summary")
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{missing:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
    };
    let of = |parts: &[(&str, &[&str])]| -> BTreeSet<String> {
        parts
            .iter()
            .flat_map(|(_, r)| r.iter().map(ToString::to_string))
            .collect()
    };
    assert_eq!(recipes(None), of(&PARTS));
    for (file, _) in &PARTS[1..] {
        assert!(core.contains(&format!("import? '{file}'")), "{file}");
        let left: Vec<_> = PARTS.iter().copied().filter(|(f, _)| f != file).collect();
        assert_eq!(recipes(Some(file)), of(&left), "{file}");
    }
}
