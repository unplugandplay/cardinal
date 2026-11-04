---
priority: medium
why: 'the release ask rule proved the mechanism; the other irreversible actions still rest on the wording of a skill'
---
# Ask before the irreversible git actions

# Goal
The skills forbid a push, a history rewrite or a deletion "unasked", and nothing but the model enforces it. The
`ask` rules of the release (`Bash(cardinal run release *.*)`, `Bash(just release *.*)`) showed that a permission
rule makes Claude Code prompt the user whatever the model decided, auto mode included. The outcome: the fragment
asks before the irreversible or outward-facing git actions, so the user confirms them in the permission prompt.

# Findings
* `files/.claude/skills/issue-workflow/SKILL.md:147`: "no push, deletion or history rewrite unasked" (autonomous
  mode, never lifted); `files/.claude/skills/issue-workflow/details.md:58`: "Never rebase a pushed branch unasked".
* `entries/.claude/settings.json` holds only the two release forms under `permissions.ask`; `sync` already manages
  the `ask` list and the lock records its lines (SYN-29, SYN-34, format 8), so new rules need no format raise.
* `git commit` must stay unasked: the autonomous mode commits each deliverable without a stop, and a rule cannot
  depend on the mode.

# Approach
1. List the candidate rules and check each against the flow of the skills (none of them may fire on a step the
   workflow runs without a stop): `Bash(git push *)`, `Bash(git push --force *)` (covered by the first),
   `Bash(git rebase *)`, `Bash(git reset --hard *)`, `Bash(git branch -D *)`, `Bash(git tag -d *)`,
   `Bash(git clean *)`, `Bash(git checkout -- *)`, `Bash(git restore *)`.
2. Add the kept ones to `permissions.ask` of `entries/.claude/settings.json`, and to the test of
   `crates/cardinal/tests/settings_entries_cli.rs` that reads the fragment.
3. Optionally declare them as a feature of `features.toml` (a project whose user pushes from the agent turns them
   off), with its entries as `sync` prints them (`ask <rule>`).
4. Reword the "unasked" lines of the skills to name the prompt, so the agent runs the command and lets the user
   answer it instead of stopping to ask in text first.

# Specs
* `specs/sync.md` if the fragment's content is specified there; `specs/features.md` if step 3 adds a feature.

# Milestone
User control

# Resources
* `entries/.claude/settings.json`, `features.toml`, `specs/features.md`
* `issues/done/keep_writing_the_release_the_users.md` (how the release ask rules were added and tested)
* ADR 0005 (the gates catch mistakes, not bypasses), ADR 0013 (sync manages the allow rules too)

# Hints
* A pattern is easy to bypass (`git -C . push`): accepted by ADR 0005, don't chase every spelling.
* Each prompt costs the user an answer: keep to the irreversible or outward-facing actions, or the prompts get
  approved unread.
* Check that an `allow` rule of a consumer (`Bash(git *)`) does not win over the `ask` rule before relying on it.
