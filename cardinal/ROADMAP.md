# Roadmap

The planned work of Cardinal, grouped into milestones, in the order they are meant to land. A milestone becomes
issues once it is shaped; the `priority` and `depends_on` of each issue file order those issues within a milestone (`just next-issue`). States: `idea`, `shaped`, `in issues`, `done` (set by `just close-issue`);
`just release` adds `- Released: v<version>` to a `done` one.
The work done before this file existed is in `issues/done/`.

## Plan and release

- Goal: plan the work above the issues, and ship numbered releases with a changelog that consumers can pin.
- Why: what is planned but not yet an issue has no home in the repository, and the consumers pin commits with no
  word of what changed.
- State: done
- Released: v0.2.0
- Issues:
  - [plan_the_milestones_in_a_roadmap_before_they_become_issues](issues/done/plan_the_milestones_in_a_roadmap_before_they_become_issues.md)
  - [release_numbered_versions_with_a_changelog](issues/done/release_numbered_versions_with_a_changelog.md)

## Team use

- Goal: let several people share a project driven by Cardinal without stepping on each other.
- Why: no project uses Cardinal in a team yet; these are the blockers an audit found.
- State: done
- Released: v1.0.0
- Issues:
  - [read_the_user_language_from_a_personal_file](issues/done/read_the_user_language_from_a_personal_file.md)
  - [rank_the_issues_without_a_shared_queue_file](issues/done/rank_the_issues_without_a_shared_queue_file.md)

## Advisor

- Goal: let a session consult a stronger advisor model at the moments where advice pays, and measure what it brings.
- Why: an advisor is configured but no call is seen in the transcripts, no skill says when to consult it, and the
  evaluation cannot tell an advised session from a solo one.
- State: done
- Released: v1.2.0
- Issues:
  - [count_the_advisor_calls_of_each_session](issues/done/count_the_advisor_calls_of_each_session.md)
  - [measure_the_main_context_peak_of_an_advised_message](issues/done/measure_the_main_context_peak_of_an_advised_message.md)
  - [tell_the_issue_workflow_when_to_consult_the_advisor](issues/done/tell_the_issue_workflow_when_to_consult_the_advisor.md)
  - [compare_an_advised_executor_with_a_solo_model](issues/done/compare_an_advised_executor_with_a_solo_model.md)

## Optional features

- Goal: let a project turn off the parts of the harness that do not fit its flow, at no cost in tokens or time.
- Why: every consumer gets the whole harness; a user whose flow differs (an AI co-author trailer, another commit
  convention, no parallel agents, issues tracked elsewhere) cannot drop a part without a conflict at each sync.
- State: done
- Released: v3.0.0
- Issues:
  - [let_a_project_turn_off_a_feature_of_the_harness](issues/done/let_a_project_turn_off_a_feature_of_the_harness.md)
  - [declare_the_hook_only_features](issues/done/declare_the_hook_only_features.md)
  - [declare_the_file_only_features](issues/done/declare_the_file_only_features.md)
  - [let_a_feature_need_another](issues/done/let_a_feature_need_another.md)
  - [split_the_managed_recipes_by_feature](issues/done/split_the_managed_recipes_by_feature.md)
  - [turn_off_the_review_gate](issues/done/turn_off_the_review_gate.md)
  - [quiet_the_review_rules_of_rule_breaches_when_the_gate_is_off](issues/done/quiet_the_review_rules_of_rule_breaches_when_the_gate_is_off.md)
  - [turn_off_the_workspace_lock](issues/done/turn_off_the_workspace_lock.md)
  - [turn_off_conventional_commits](issues/done/turn_off_conventional_commits.md)
  - [turn_off_the_issue_files](issues/done/turn_off_the_issue_files.md)
  - [turn_off_the_specs](issues/done/turn_off_the_specs.md)
  - [find_the_root_of_a_done_issue](issues/done/find_the_root_of_a_done_issue.md)
  - [release_a_project_without_the_commit_convention](issues/done/release_a_project_without_the_commit_convention.md)
  - [leave_the_issues_directory_alone_with_the_issue_files_off](issues/done/leave_the_issues_directory_alone_with_the_issue_files_off.md)
  - [release_across_the_turn_off_of_the_commit_convention](issues/done/release_across_the_turn_off_of_the_commit_convention.md)

## Longer autonomy

- Goal: let one session drive several issues in a row, stopping the user once for the plans of the batch.
- Why: one session per issue works today; chaining them saves the user the restarts.
- State: done
- Released: v3.0.0
- Issues:
  - [let_the_close_of_an_issue_commit_without_a_review](issues/done/let_the_close_of_an_issue_commit_without_a_review.md)
  - [drive_several_issues_in_a_row_from_one_session](issues/done/drive_several_issues_in_a_row_from_one_session.md)
  - [order_the_issues_by_priority_and_dependencies](issues/done/order_the_issues_by_priority_and_dependencies.md)
  - [link_the_next_issue_when_closing_one](issues/done/link_the_next_issue_when_closing_one.md)
  - [warn_when_starting_a_blocked_issue](issues/done/warn_when_starting_a_blocked_issue.md)
  - [claim_an_issue_from_the_session_that_works_it](issues/done/claim_an_issue_from_the_session_that_works_it.md)
  - [reuse_a_worktree_across_issues](issues/done/reuse_a_worktree_across_issues.md)
  - [count_the_sessions_of_a_closed_worktree_with_its_repository](issues/done/count_the_sessions_of_a_closed_worktree_with_its_repository.md)
  - [hand_over_the_worktree_lock_of_a_clean_session](issues/done/hand_over_the_worktree_lock_of_a_clean_session.md)
  - [set_the_milestone_done_when_its_last_issue_closes](issues/done/set_the_milestone_done_when_its_last_issue_closes.md)
  - [allow_the_release_proposal_in_auto_mode](issues/done/allow_the_release_proposal_in_auto_mode.md)
  - [share_one_roadmap_grammar_between_the_tools](issues/done/share_one_roadmap_grammar_between_the_tools.md)
  - [keep_writing_the_release_the_users](issues/done/keep_writing_the_release_the_users.md)

## Lean agent loop

- Goal: keep one agent loop driven by skills and tools, with a subagent only where a measure justifies it, and judge
  each change of the instructions before it spreads.
- Why: the one subagent of the workflow never got its verdict, a skill missed when due is not counted, the
  instructions grew one issue at a time, and every verdict is observational.
- State: done
- Released: v3.1.0
- Issues:
  - [show_the_cache_read_share_in_the_efficiency_table](issues/done/show_the_cache_read_share_in_the_efficiency_table.md)
  - [report_the_tokens_spent_per_closed_issue](issues/done/report_the_tokens_spent_per_closed_issue.md)
  - [require_enough_issues_for_a_per_issue_verdict](issues/done/require_enough_issues_for_a_per_issue_verdict.md)
  - [judge_the_delegated_exploration_before_keeping_it](issues/done/judge_the_delegated_exploration_before_keeping_it.md)
  - [require_enough_reviews_and_spec_lines_for_a_ratio_verdict](issues/done/require_enough_reviews_and_spec_lines_for_a_ratio_verdict.md)
  - [count_the_skills_a_session_should_have_loaded](issues/done/count_the_skills_a_session_should_have_loaded.md)
  - [detect_token_waste_in_the_transcripts](issues/done/detect_token_waste_in_the_transcripts.md)
  - [route_the_update_commit_to_the_message_skill](issues/done/route_the_update_commit_to_the_message_skill.md)
  - [weigh_the_token_waste_per_revision](issues/done/weigh_the_token_waste_per_revision.md)
  - [audit_the_shared_instructions_for_conflicts_and_stale_guidance](issues/done/audit_the_shared_instructions_for_conflicts_and_stale_guidance.md)
  - [compute_the_size_of_a_deliverable_with_a_tool](issues/done/compute_the_size_of_a_deliverable_with_a_tool.md)
  - [refuse_staging_every_file_before_a_commit](issues/done/refuse_staging_every_file_before_a_commit.md)
  - [replay_a_reference_task_set_to_compare_two_harness_variants](issues/done/replay_a_reference_task_set_to_compare_two_harness_variants.md)
  - [tell_the_needed_oversized_results_from_the_waste](issues/done/tell_the_needed_oversized_results_from_the_waste.md)
  - [find_why_commits_skip_the_commit_message_skill](issues/done/find_why_commits_skip_the_commit_message_skill.md)
  - [count_only_the_fixes_of_the_lines_an_issue_changed](issues/done/count_only_the_fixes_of_the_lines_an_issue_changed.md)
  - [find_why_the_advisor_is_never_consulted](issues/done/find_why_the_advisor_is_never_consulted.md)
  - [know_the_new_line_types_of_the_transcripts](issues/done/know_the_new_line_types_of_the_transcripts.md)
  - [leave_the_issue_file_and_the_synced_copies_out_of_the_issue_review](issues/done/leave_the_issue_file_and_the_synced_copies_out_of_the_issue_review.md)
  - [hint_before_a_whole_read_of_a_large_file](issues/done/hint_before_a_whole_read_of_a_large_file.md)

## User control

- Goal: let the user confirm each irreversible action and answer each question of the agent in a prompt of Claude
  Code, rather than rely on the model stopping to ask in text.
- Why: the ask rules of the release proved that a prompt holds whatever the model decides, auto mode included,
  while the other confirmations and the questions still rest on the wording of the skills and are easy to miss.
- State: in issues
- Issues:
  - [ask_before_the_irreversible_git_actions](issues/ask_before_the_irreversible_git_actions.md)
  - [ask_the_user_through_the_question_tool](issues/ask_the_user_through_the_question_tool.md)
