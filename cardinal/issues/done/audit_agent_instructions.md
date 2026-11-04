# Audit the instructions given to the agent to propose new skills and hooks

# Goal
The instructions a user gives an agent over time (CLAUDE.md, AGENTS.md, corrections repeated in session
transcripts, memory files) reveal practices that should be automated: a recurring multi-step procedure is a
skill, a rule the agent must never break is a hook. Today the harness grows only when the user notices such a
need. Add an audit that reads those instructions and proposes the skills and hooks worth adding, for the user
to validate.

# Findings
* The harness ships six skills (`commit-message`, `create-issue`, `issue-workflow`,
  `prioritize-issues`, `update-docs`, `write-spec`) and two hooks (`format_rust.rs`, `clippy_gate.rs`), all
  written by hand.
* Measured on the user's machine (counts only, no quotes): transcripts are in `~/.claude/projects/<project>/*.jsonl`
  (one directory per project, subagent transcripts in `<session>/subagents/`); 8 projects hold sessions,
  about 130 top-level sessions and 1,170 typed user messages (a few hundred KB of text, small enough to read in
  full; counted by `audit_sources.rs`), the largest project holding two thirds of them. A message typed by the user is a `type: "user"`
  line, not `isMeta`, whose text content does not start with `<`; the others are tool results and reminders.
* Most frequent typed messages are validations ("N ok", "oui", "je valide", about 250 of them), "continue",
  "commit" and "code review" (a dozen each): they are the steps `issue-workflow` already automates, so they
  are not candidates. Explicit rule words are rare (about 35 "never", 13 "toujours", 9 "jamais"): keyword
  search alone finds little, the agent must read the messages.
* Memory holds a few files in 8 projects (`feedback_*`, project notes), and the user's global
  `CLAUDE.md` is 7 lines. Instruction files exist in 6 projects.

# Approach
Measure first: list the sources available in a consumer project and what a sample of them yields.

1. Collect the sources: instruction files (`CLAUDE.md`, `AGENTS.md`), the user's session transcripts, memory.
2. Detect candidates: instructions repeated across sessions or corrected more than once, and rules phrased as
   "never" / "always" that a hook could enforce.
3. Classify each candidate: skill (procedure with steps), hook (mechanical check on a tool call), or neither
   (a plain rule stays in the instruction file).
4. Output a report of proposals with the evidence (quotes, counts) for validation; creating them goes through
   `skill:create-issue`, nothing is generated silently.

Delivery (decided): a skill run by the agent, no new binary, backed by a `rust-script` that extracts the typed
user messages and instruction files with counts (tested by `just test-harness`). A skill keeps the shared
files free of project specifics. All projects of `~/.claude/projects/` are read. The report is shown in the
conversation; an optional copy goes outside the repository and outside `/tmp` (wiped at reboot), e.g.
`~/.local/state/`, never committed.

# Specs
* `specs/audit.md` if a `harness` subcommand is chosen; none for a skill alone.

# Resources
* `docs/architecture.md`, `files/.claude/skills/`, `files/.claude/hooks/`
* `skill:create-issue`, `skill:update-config` (hooks are declared in `settings.json`)

# Hints
* Transcripts can hold private content: the report must follow the `confidential` note of
  `.claude/harness.toml` and never be committed with raw quotes from other projects.
* The shared files hold nothing specific to one project.

# Tracking

1. [x] Skill `audit-instructions`: `SKILL.md` (collect, detect, classify, report; confidentiality; creation through `skill:create-issue`) and its script `audit_sources.rs` (typed user messages, instruction files, memory, with counts), tested in `test_audit_sources.rs`
2. [x] Docs: `docs/architecture.md` (the other documents name no skill list; n/a)
3. [x] Real run on the user's transcripts, report validated; filed `hook_block_ai_attribution_in_commits`, `hook_code_review_before_commit` and `issue_workflow_autonomous_mode`. The `/tmp` paths of the other project were already fixed (decision: no issue)

## Decisions

* Delivery is a skill plus an extraction script, no `harness` subcommand, hence no `specs/audit.md`.
* All projects of `~/.claude/projects/` are read.
* The report is shown in the conversation; an optional copy goes outside the repository and `/tmp`
  (e.g. `~/.local/state/`), never committed.
* The script and the skill are one deliverable: `test_skills.rs` requires a `SKILL.md` in every skill directory.
* No ADR: the choice is recorded here.
