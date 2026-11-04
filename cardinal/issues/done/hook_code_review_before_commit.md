# Refuse a commit until the code review has run on the current diff

# Goal
A hook that makes the agent run the code review before it commits, instead of relying on it remembering step 5
of `skill:issue-workflow`.

# Findings
* Found by the instruction audit (counts only): the code review is mentioned in about 40 user messages over 30
  sessions and 5 projects; the user wrote that the agent systematically forgets it and that it is never optional.
* `issue-workflow` already requires it ("mandatory, never skipped") and still gets skipped, so the instruction
  alone does not hold.
* A hook has no memory: it needs a marker (the diff hash at the time of the review) written when the review
  skill ends and compared at `git commit`.

# Approach
Measure first: check which hook events expose the invocation of a skill (`PostToolUse` on `Skill`) and whether
the review result can be seen.
1. A `PostToolUse` hook on `Skill` for the code review writes a marker holding the hash of `git diff HEAD`
   (staged and unstaged) under the project's `.claude/` state, git-ignored.
2. A `PreToolUse` hook on `Bash` for `git commit` recomputes the hash; if it differs from the marker (or no marker
   exists), it exits 2 telling the agent to run the review.
3. Escape hatches to decide at `issue-workflow` steps 1 to 3: a commit of only docs or issues, and an explicit
   user waiver.

# Resources
* `files/.claude/hooks/`, `files/.claude/skills/issue-workflow/SKILL.md` (step 5)
* `seed/.claude/settings.json` (hooks are declared there)

# Hints
* A fix made after the review changes the hash: the rule is "review the final diff", which is what
  `issue-workflow` asks, but the agent re-runs it often; consider accepting fixes after a review that reported
  findings.
* The shared files hold nothing specific to one project.

# Tracking

1. [x] Measure: capture the real `PostToolUse` (`Skill`) and `PreToolUse` (`Bash`) payloads with a temporary git-ignored log hook; record the available fields and whether the review findings are visible
2. [x] Marker and refusal: `require_code_review.rs` (`record` on `PostToolUse` `Skill`, `check` on `PreToolUse` `Bash` for `git commit`): marker = hash of the diff against `HEAD`, reviewed files, `HEAD`; exit 2 without a valid marker; tests in `test_hooks.rs`
3. [x] Escape hatches: files all under `issues/`, user waiver file `.git/claude-review-waived` (fixes on already reviewed files and `HEAD` changed invalidating the marker were done in deliverable 2); one test each
4. [x] Docs: update step 4.5 of `issue-workflow` and the docs, `just sync`, `just check`, `just test-harness`

## Decisions

* Mode: autonomous (validation of steps 4.6 and 4.8 lifted).
* No spec: the hooks do not change what the `harness` tool does.
* Measured (deliverable 1): `PostToolUse` on `Skill` gets `tool_input.skill` (`code-review`), `session_id`, `cwd`,
  `transcript_path` and `tool_response` = `{success, status: "forked", agentId, result}` where `result` is the
  free text of the review, so the findings are visible but unstructured (no machine-readable "no findings").
  The `Bash` calls of the forked review agent also fire `PreToolUse` (with `agent_id` and `agent_type` set).
* The hook does not parse the findings; fixes after a review are accepted when every staged file was already reviewed.
* Follow-up (not this issue): `block_ai_attribution.rs` `writes_message` misses `gh -R owner/repo pr create ...`
  (flag between `gh` and `pr`), reported by the review of this run.
* The marker hashes the staged content (`git diff --cached`), what will actually be committed.
* The waiver is a file only the user creates, never driven by a sentence of the agent.
* Docs must be reviewed; only a commit touching nothing but `issues/` is exempt.
* Deviation (deliverable 2): the marker hashes the working tree against `HEAD` (tracked and untracked), not
  `git diff --cached`: the hook runs before the `git add` of `git add -A && git commit`, when nothing is staged yet.
* The declarations (seed and own `settings.json`) and the architecture doc moved into
  deliverable 2: `test_settings.rs` fails for an undeclared hook, so the commit would be red otherwise. It also
  now takes the `.rs` word of a command as the script, since the hook command carries a mode argument.
* Review of deliverable 2: the marker lives in the git directory (`git rev-parse --git-path claude-code-review.json`),
  not in `.claude/state/`: a consumer that does not ignore that directory would commit it with `git add -A`.
  It is per repository, so `git -C dir commit` reads the marker of `dir`. Hence no `.gitignore` entry, and the
  user waiver of deliverable 3 is a file in the git directory too (`.git/claude-review-waived`).
* Not fixed, by design: a commit with nothing to commit (clean-tree amend) and a machine without `git` are not
  gated; `cd dir && git commit` is not followed; the review is recorded at the end of the forked skill (measured),
  not at load.
* Moved into deliverable 2: the fixes-after-review rule (files subset), needed because the hook guards the commits of
  this very issue. Left for deliverable 3: `issues/`-only exemption, waiver.
* Not fixed (review): quoted text naming the commit command counts as a commit (only refuses a dirty unreviewed
  tree; it refused a shell command of this very session); the `check` hook starts `rust-script` on each `Bash`
  call; a nested repository or dangling symlink among untracked files hashes as empty; hashing via a temporary
  index (add everything, `write-tree`) would be sturdier.

### Deliverable 2 summary

* Done: `require_code_review.rs` (`record`, `check`), its behavior test, declarations in the seed and own settings,
  the `.rs` script lookup of `test_settings.rs`, the architecture doc. Verified: `just check`, `just test-harness`.
* Not verified: the hook in a consumer project; a real refusal followed by a review in a live session beyond this one.
* Code review: four rounds. Fixed: marker inside the hashed content, quoted non-ASCII names (`-z`), subcommand
  parsing and separators, `git -C` (relative to the shell directory), subdirectory cwd, repository without commit,
  failed review recorded, name/length delimiters in the hash, unmoved marker (now in the git directory), the
  fixes-after-review loop. Not fixed: see the lines above.

### Deliverable 3 summary

* Done: a diff under `issues/` only passes (renames listed as deletion plus addition); a waiver file created by
  the user in the git directory lets one commit through and is removed by the hook; any `Bash` command naming
  the waiver file is refused (a deterrent: a `Write` of the file is not caught). Documentation is reviewed (not exempt).
* Verified: `just check`, `just test-harness`. Not verified: the waiver in a linked worktree.
* Decision: the waiver is one-shot and consumed when the check passes, even if the commit then fails.
* Code review: waiver creatable by the agent (fixed by refusing commands that name it), rename into `issues/`
  bypass (fixed with `--no-renames`), wrapping and doc claims (fixed). Not fixed: the waiver can still be written with the `Write` tool (doc says deterrent); waived commits leave no trace;
  consumed before the commit succeeds; worktree test.

### Deliverable 4 summary

* Done: step 4.5 of `issue-workflow` states the gate, the order (tracking written before the review, fixes to
  reviewed files pass, `issues/`-only diffs exempt) and that only the user creates the waiver. The architecture
  doc and the declarations were done in deliverables 2 and 3.
* Verified: `just sync`, `just check`, `just test-harness`.
