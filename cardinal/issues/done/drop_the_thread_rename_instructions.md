# Drop the thread rename instructions from the skills

# Goal
`skill:issue-workflow` tells the agent to name the thread after the issue with `/rename <title>`, or to hand the
user the line to paste, at the start and on every resume. It is not needed and only adds noise to the agent output.
Remove the instruction so the agent no longer mentions renaming.

# Findings
* `files/.claude/skills/issue-workflow/SKILL.md`, step 0: asks for `/rename <title>`, and for the line to be given to
  the user when the agent cannot type it (a skill cannot type a slash command), once, and again on resume.
* `files/.claude/skills/create-issue/SKILL.md`, template: the title comment says "it names the thread via /rename".
* No spec, ADR or doc found that mentions `/rename` (`grep -rn "/rename" specs docs crates` is empty).

# Approach
1. Delete the naming paragraph from step 0 of `issue-workflow`; keep the resume/start logic (Tracking section,
   spec check) untouched.
2. Reword the title line of the `create-issue` template without the `/rename` mention.
3. Run `just sync` so this repository's own copies follow, then `just check-harness`.

# Tracking
1. [x] Drop the `/rename` instructions from both skills, sync the copies, check.

## Decisions
* Mode: autonomous. No spec (no behaviour change); verified by `grep -rn "/rename" files .claude` being empty.

## Summary
Removed the naming paragraph of step 0 in `issue-workflow` and the `/rename` mention in the `create-issue` template;
copies synced. Verified: grep empty, `just check`, `just test-harness`, `just check-harness`. Not verified: nothing else.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`
* `files/.claude/skills/create-issue/SKILL.md`
* `docs/architecture.md` (managed files, sync)

# Hints
* Edit `files/`, never the copies under `.claude/`; `just check-harness` fails on drift.
* Keep the issue title rule itself (first `# ` heading, imperative): `check_issue.rs` and the queue still use it.
