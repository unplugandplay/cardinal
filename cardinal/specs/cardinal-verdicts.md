---
id: VRD
status: accepted
adrs: [0002, 0010]
---

# Verdicts on the recorded changes of Cardinal

## Intent

A commit of the Cardinal source that changes the shared files records the effect it is expected to have, as a
metric of `cardinal-report` and a direction; `cardinal run cardinal-changes` lists these expectations with the
revisions in use that hold each change, and `cardinal-report --changes` gives each one a verdict from the groups
before and after it, `confounded` whenever something else changed too, so `skill:evaluate-harness` ties a trend to
the change that should have caused it, or says it cannot.

## Requirements

- **VRD-1** (deterministic) An expectation must be a line of a commit message of the form `Expected-effect: <metric> <direction>`, `<direction>` being `up` or `down` and `<metric>` one of `turns`, `tool_calls`, `output_tokens`, `input_tokens`, `validations`, `other_messages` (REP-5), `cache_read_share` (REP-12), `main_context_peak` (REP-9), `advisor_calls`, `advisor_input_tokens`, `advisor_output_tokens` (REP-11), `fixes_after_per_issue`, `findings_per_review`, `doc_lines_per_spec_line`, `input_tokens_per_issue`, `output_tokens_per_issue` (REP-6), `total_bytes` (REP-7), `bytes_per_session:<rule>` (VRD-13), or `breached_share:<rule>`, `<rule>` a name of lowercase letters, digits and `-`, as the rules of the `rule-breaches` summary (BRE-3), and the line holding nothing else.
- **VRD-2** (deterministic) `commit-check` must refuse, with a message naming the line, a commit message holding a line that starts with `Expected-effect:` and is not an expectation of VRD-1.
- **VRD-3** (deterministic) `cardinal-changes [repo] --sessions <file>` must print a header line then one tab-separated row per expectation of the non-merge commits of the default branch of `repo`, those of a merged branch included, (default the current directory; the default branch as EVA-8 picks it) that change a path under `files/`, with the columns `change` (the full hash of the commit), `metric`, `direction` and `revisions`, in the order `git log --reverse` lists the commits (oldest first), then in the order of the lines of a message.
- **VRD-4** (deterministic) `revisions` must list the distinct `revision` values of the sessions file, in the order of the earliest `start` of their sessions and joined by `,`, whose commit (the value itself, or what follows `self:`) is a hexadecimal hash of 7 to 40 digits that exists in `repo` and has the change as an ancestor or is the change; a revision git cannot resolve in `repo`, a branch or tag name, `unpinned` and the empty revision are never listed.
- **VRD-5** (deterministic) With `--changes <file>` (the rows of VRD-3, any other header refused as REP-3), `cardinal-report` must print a `## Verdicts` table, after the other tables, with one row per change and metric, in the order of the file, and the columns `change` (its first 7 hexadecimal digits), `metric`, `expected` (the direction), `before`, `after` (group names of REP-4), `before_value`, `after_value` and `verdict`; the after group is the first group (REP-4) holding a revision of the row's `revisions`, the before group the group just before it, and the values the cells of the metric in the table VRD-1 takes it from (efficiency for REP-5, quality for REP-6, fixed context for `total_bytes`, rules for `breached_share` and `bytes_per_session`, of the rule's row and as a number) of these groups (computed for a `too few` group too), empty when the table is not printed, the group is missing or the cell is empty.
- **VRD-6** (deterministic) `verdict` must be `no data` when a group or a value of the row is missing, else `confounded` when a group is `too few`, when the after group holds a revision absent from the row's `revisions`, when the most frequent name of `models`, of `versions` or of `projects` (REP-5, before `--anonymous` hides the projects) differs between the two groups, or when another change of the file has the same after group; else `helped` when the after value moved from the before value in the expected direction, `hurt` when it moved the other way and `no observed effect` when they are equal.
- **VRD-9** (deterministic) For the metric `main_context_peak`, a group whose `peak_sessions` (REP-10) is fewer than `--min-sessions` must count as `too few` in VRD-6, the before group included: the verdict is then `confounded`, or `no data` when no session of a group has a peak (its value is missing).
- **VRD-10** (deterministic) For the metrics `advisor_calls`, `advisor_input_tokens` and `advisor_output_tokens`, a group whose `advisor_sessions` (REP-11) is fewer than `--min-sessions` must count as `too few` in VRD-6, the before group included, as VRD-9 does for `main_context_peak`, and a change of the most frequent name of `advisor_models` (REP-11) between the two groups as a change of the mix of VRD-6.
- **VRD-11** (deterministic) For the metric `fixes_after_per_issue`, a group whose `issues` (REP-6) is fewer than `--min-issues`, and for the metrics `input_tokens_per_issue` and `output_tokens_per_issue`, a group whose `token_issues` (REP-6) is fewer than `--min-issues`, must count as `too few` in VRD-6, the before group included, as VRD-9 does for `main_context_peak`; the verdict is then `confounded`, or `no data` when the value is missing.
- **VRD-12** (deterministic) For the metrics `findings_per_review` and `doc_lines_per_spec_line`, a group whose `issues` (REP-6) is fewer than `--min-issues`, and for `findings_per_review` also a group whose `reviews` (REP-6) is fewer than `--min-issues`, must count as `too few` in VRD-6, the before group included, as VRD-11 does for the per-issue metrics; the verdict is then `confounded`, or `no data` when the value is missing.
- **VRD-13** (deterministic) The metric `bytes_per_session:<rule>`, `<rule>` a waste rule of `rule-breaches` (BRE-20), must be accepted as a metric of VRD-1 by `commit-check` and `cardinal-changes`, and its values (VRD-5) be the cells `bytes_per_session` (REP-13) of the rule's row in the rules table.
- **VRD-7** (deterministic) The output of `cardinal-changes` and of the `## Verdicts` table must hold no text of a commit message other than the metric and the direction of its expectations.
- **VRD-8** (deterministic) `cardinal-changes` must refuse with an error and a non-zero exit a missing `--sessions`, a sessions file whose header is not the one of EVA-1, an unknown option, or a `repo` that is not a git repository.

## Scenarios

- (VRD-3) Given a commit that changes only `docs/` with an `Expected-effect: turns down` line, when the rows are printed, then it has none.
- (VRD-3) Given a commit with an expectation on a branch merged into the default branch by a merge commit, when the rows are printed, then it has its row.
- (VRD-4) Given a session whose revision is `main`, when the rows are printed, then `main` is never listed.
- (VRD-4) Given sessions of revisions `r1`, `self:r2` and `r3`, the change being `r2` and `r3` a descendant of it, when the rows are printed, then `revisions` is `self:r2,r3`.
- (VRD-5) Given a change held by the revisions of the second group only, when the report is printed, then `before` is the first group and `after` the second.
- (VRD-5) Given a change held by a revision of the first group, when the report is printed, then `before` is empty and `verdict` is `no data`.
- (VRD-6) Given groups `a` and `b` of 10 sessions each with the same models, versions and projects, the change held by every revision of `b` and none of `a`, `findings_per_review` 1.50 in `a` and 0.80 in `b`, when the expectation is `findings_per_review down`, then `verdict` is `helped`.
- (VRD-6) Given the same groups where the after group also holds a session of a project pinned to a revision without the change, then `verdict` is `confounded`.
- (VRD-9) Given groups `a` and `b` of 10 sessions each, 8 sessions of `a` archived before `main_context_peak`, when the expectation is `main_context_peak down`, then `verdict` is `confounded`, whatever the two medians; with all 10 sessions of `a` archived before the column, it is `no data`.
- (VRD-10) Given groups `a` and `b` of 10 sessions each, 3 sessions of `a` with an advisor set, when the expectation is `advisor_calls up`, then `verdict` is `confounded`, whatever the two medians.
- (VRD-11) Given groups `a` and `b` of 10 sessions each, `a` closing 2 issues and `b` 6, when the expectation is `fixes_after_per_issue down`, then `verdict` is `confounded`, whatever the two ratios.
- (VRD-12) Given groups `a` and `b` of 10 sessions each, `a` closing 6 issues with 2 reviews and `b` 6 issues with 12 reviews, when the expectation is `findings_per_review down`, then `verdict` is `confounded`, whatever the two ratios; with `a` holding 12 reviews and `b` closing 3 issues, `findings_per_review down` and `doc_lines_per_spec_line down` are `confounded` too.
- (VRD-2) Given a message with `Expected-effect: speed up`, when it is checked, then `commit-check` refuses it.
- (VRD-13) Given groups `a` and `b` of 10 sessions each with the same mix, the change held by every revision of `b` and none of `a`, `oversized-result` with `bytes_per_session` 30000 in `a` and 12000 in `b`, when the expectation is `bytes_per_session:oversized-result down`, then `verdict` is `helped`, whatever its `breached_share`.
- (VRD-1) Given a commit of `files/` with `Expected-effect: doc_lines_per_spec_line up`, when it is checked, then `commit-check` accepts it.

## Non-goals

- A threshold on the agent's behaviour: the expectation names a direction, never a value to reach.
- A significance test: a verdict compares two groups; the evaluation reads `confounded` and `too few` as no
  conclusion, and a trend over two groups or more (`skill:evaluate-harness`).
- Replaying a task under two revisions: the evaluation stays on real sessions of real projects.
- Expectations recorded outside the commit messages (an issue section, a file under `docs/`), or changed after the
  commit: the history is the record.
- The changes of a consumer's own files: only the Cardinal source is read.
