# Behaviour inventory

Status: step 2 of `redesign.md` Section 13.
Date: 2026-09-08.
Source: 110 tests across `tests/guardrails.rs`, `tests/improvements/*`,
`tests/documentation_budget/*` and `xtask/tests/quality.rs`, read by body, not
by name.

One line per rule. `Test` is the current test that pins it; several tests pin
more than one rule and appear more than once. `Name` is the new test name under
Section 17 rule 5. The port of a module's lines is that module's exit criterion.

Decision key: **keep** restate at the new boundary; **rewrite** the rule
changes under the design; **adapter** becomes a contract test against a fake
process; **drop** representation, migration or a deleted mechanism.

## Decisions taken

Two rules were found only by reading bodies and were not settled in
`redesign.md`. Both are now recorded there (Section 14).

1. **One unused pair at a time.** `ordinary_exception_preserves_documentation_schema_marker`
   authorised a documentation pair and a source pair while both were unused.
   The coexistence was incidental to a schema-marker test; Section 5 stands.
2. **Retroactive documentation credit is dropped.** `reconcile-documentation-extra`
   refunded a consumed source pair whose commits were documentation only. It
   existed because a hard cap of 3 was hit by documentation repairs; with
   `full` at 5 and warnings not consuming a cycle, the case does not recur.
   The five rules marked *refund* below are deleted, not ported.

## domain::queue

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Membership freezes at discovery; a second discover is rejected | `freezes_membership_and_respects_priority` | `discover_freezes_membership` | keep |
| Claim follows priority order | same | `claim_respects_priority` | keep |
| One active task at a time | `does_not_overlap_tasks` | `claim_rejects_second_active` | keep |
| Code dependency needs a main commit, not Jira Done | `blocks_code_dependency_without_delivery_on_main` | `dependency_requires_main_commit` | keep |
| Another run holding the issue blocks claim | `cross_run_issue_claims_are_exclusive` | `claim_excludes_other_runs` | keep; reservation moves to `adapters::git` |
| Open-PR cap blocks new claims but not resume | `unfinished_pr_limit_stops_claims_but_allows_existing_work_to_resume` | `pr_cap_blocks_claim_not_resume` | keep |
| Refresh delta replaces full list; no TTL | new, Section 10 | `refresh_accepts_delta` | new |
| Blocked and needs-input partition at discovery | new, Section 10 | `discover_partitions_blocked_and_needs_input` | new |

## domain::worktree

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Cannot clean a slot with unfinished work | `cannot_clean_unfinished_or_unknown_slot` | `cleanup_rejects_unfinished_slot` | keep |
| Existing directory of unknown ownership is occupied | same | `reserve_rejects_unknown_directory` | keep |
| Uncertain provision confirms only the slot the task already holds | `uncertain_provision_confirms_the_same_already_bound_slot` | `uncertain_provision_confirms_same_slot` | drop; bind-slot is synchronous through Vcs (revision 3) |
| Uncertain provision cannot confirm a different slot | `uncertain_provision_cannot_confirm_a_different_already_bound_slot` | `uncertain_provision_rejects_other_slot` | drop; bind-slot is synchronous through Vcs (revision 3) |
| A slot with a second unfinished owner is not bindable | `same_slot_replay_cannot_bypass_a_second_unfinished_owner` | `bind_rejects_second_owner` | keep |
| Missing checkout owned by a held task blocks provisioning before the helper runs | `provision_rejects_missing_checkout_owned_by_unfinished_task_before_helper_runs` | `provision_checks_ownership_first` | keep |
| Collision settles failed, preserves both tasks, allows another slot | `legacy_provision_collision_settles_failed_and_allows_a_different_slot` | `collision_settles_failed` | drop; bind-slot is synchronous through Vcs (revision 3) |
| Reconciling a provision never bypasses the active-task guard | `legacy_collision_does_not_bypass_active_task_guard` | `reconcile_respects_active_guard` | drop; bind-slot is synchronous through Vcs (revision 3) |
| No delivery binds a slot any closed delivery used | `followup_preserves_dirty_old_worker_and_refuses_historical_or_foreign_replacement`, `followup_rejects_symlink_foreign_dirty_and_other_task_slot_without_effects` | `bind_rejects_historical_slot` | keep |
| Symlink, dirty or foreign checkout rejected without effects | same | `symlink_slot_rejected`, `foreign_slot_rejected`, `dirty_slot_rejected` | keep |
| Slot reuse grant binds a historical slot only for an unstarted delivery whose historical owner completed there | `explicit_historical_slot_reuse_refreshes_only_unstarted_delivery_state`, `historical_slot_exception_rejects_unfinished_owner_or_started_prepared_work` | `slot_reuse_requires_unstarted_delivery`, `slot_reuse_requires_completed_owner` | keep |
| Slot reuse refreshes the delivery base to current main | `explicit_historical_slot_reuse_refreshes_only_unstarted_delivery_state` | `slot_reuse_refreshes_base` | keep |
| Completed delivery is stable across worker reset, branch reuse and removal | `completed_delivery_stays_stable_after_worker_reset_reuse_and_removal` | `closed_delivery_ignores_worktree_changes` | keep, one case |

## domain::budget

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Cancelled reviews consume review budget | `cancellations_consume_all_three_review_attempts_across_reopen` | `cancelled_review_counts` | keep; cap becomes tier-dependent |
| Continued implementer turns after one review use one repair cycle | `continued_repair_turns_use_one_cycle` | `continued_turns_use_one_repair` | keep |
| Escalation review spends review and escalation budget | `astra_review_consumes_both_budgets` | `escalation_spends_both_budgets` | keep; model literal becomes tier |
| Escalation needs a completed capable review first | same | `escalation_requires_prior_review` | keep |
| Rejected launch writes nothing and spends nothing | `rollback_does_not_consume_attempt_on_rejected_model`, `malformed_commands_fail_closed` | `rejected_launch_spends_nothing` | keep |
| Requirements change invalidates evidence, keeps counters | `requirements_change_invalidates_evidence_without_resetting_budget` | `brief_change_keeps_counters` | keep |
| Two stalled checkpoints exhaust implementation; replan and resume do not reset | `stalled_implementation_is_bounded_across_reopen_and_replan` | `stall_limit_survives_replan` | keep |
| Implementation limit counts cancelled turns; validation still allowed | `implementation_limit_counts_cancelled_attempts_and_still_allows_validation` | `turn_limit_counts_cancelled`, `exhausted_turns_allow_review` | keep |
| New evidence resets stall streak; duplicate artifact does not | `new_evidence_resets_stall_streak_but_duplicate_evidence_does_not` | `stall_resets_on_new_artifact` | keep |
| Review budget spans deliveries | `staged_last_review_and_extra_stage_cycle_recover_with_fresh_final_review_authority`, `followup_missing_worker_runs_one_pair_and_publishes_without_reusing_history` | `review_budget_spans_deliveries` | keep |
| Pair grant adds one implementer and one reviewer; failed halves are spent | `human_exception_allows_one_cycle_without_resetting_counters`, `additional_adjustments_archive_history_and_reject_replayed_authority`, `followup_failed_pair_consumes_authority_and_next_pair_requires_fresh_receipt` | `pair_grants_one_of_each`, `failed_pair_is_spent` | keep |
| Pair grant requires the task to be held | `human_exception_allows_one_cycle_without_resetting_counters`, `exception_does_not_authorize_other_tasks_or_early_budget_growth` | `pair_requires_hold` | keep |
| Pair grant applies to one task only | `exception_does_not_authorize_other_tasks_or_early_budget_growth` | `pair_is_task_scoped` | keep |
| Path-scoped pair launches do not count against any budget | `docs_round_preserves_history_and_excludes_budget_across_restart`, `docs_turn_does_not_charge_or_reset_exhausted_implementation_budget`, `operational_infrastructure_readme_is_documentation_without_source_allowance` | `scoped_pair_is_uncounted` | keep |
| Path-scoped pair does not reset exhausted budgets | `docs_turn_does_not_charge_or_reset_exhausted_implementation_budget` | `scoped_pair_keeps_exhaustion` | keep |
| Path-scoped pair does not manufacture acceptance | `docs_round_preserves_history_and_excludes_budget_across_restart`, `explicit_root_documentation_is_exempt_but_still_needs_acceptance` | `scoped_pair_needs_acceptance` | keep |
| Verification pair spends only its review half | `staged_last_review_and_extra_stage_cycle_recover_with_fresh_final_review_authority` | `verification_pair_spends_review_only` | keep |
| Budgets by tier: trivial 1, lite 3, full 5 | new, Section 10 | `review_cap_follows_tier` | new |
| *refund*: refund lowers reviews and repairs by one, keeps launches | `historical_docs_credit_preserves_history_and_only_one_source_round` | | drop; mechanism deleted |
| *refund*: refund applies once | same | | drop; mechanism deleted |
| *refund*: pair with any source commit, reverted or not, is not refundable | `reverted_source_work_in_historical_pair_is_not_documentation`, `reconciliation_rejects_source_scope_and_changed_counting_receipt` | | drop; mechanism deleted |
| *refund*: later unlaunched source snapshot blocks refund | `unlaunched_later_source_snapshot_does_not_receive_credit`, `later_docs_round_is_allowed_but_cannot_launder_prior_source_drift` | | drop; mechanism deleted |
| *refund*: schema marker preserved | `subsequent_documentation_authorization_preserves_reconciliation_schema` | | drop; representation |

## domain::authority

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| A digest registers once | `additional_adjustments_archive_history_and_reject_replayed_authority`, `followup_requires_unchanged_historical_pr_and_rejects_reused_authorization`, `docs_round_preserves_history_and_excludes_budget_across_restart` | `digest_registers_once` | keep |
| Changed receipt blocks every use of its grant | `human_exception_allows_one_cycle_without_resetting_counters`, `docs_round_rejects_wrong_scope_and_changed_authorization`, `followup_preparation_does_not_grant_launches_and_changed_receipt_blocks_launch`, `changed_stage_authority_and_pending_stage_block_followup_and_launch` | `changed_receipt_blocks_grant` | keep |
| Receipt path must be absolute and readable | `followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation` | `receipt_must_be_absolute` | keep |
| Grant requirements digest must match current criteria | `followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation`, `recovery_rejects_changed_receipts_partial_pairs_and_wrong_replacement_without_effects` | `grant_requires_current_criteria` | keep |
| Unused pair may be re-registered as the next delivery's grant | `closed_recovery_preserves_history_replacement_and_only_transfers_unused_pair` | `unused_pair_repurposes` | rewrite as Section 5 rule |
| Partly used pair cannot be repurposed | `recovery_rejects_changed_receipts_partial_pairs_and_wrong_replacement_without_effects` | `partial_pair_stays_put` | keep |
| Path scope must be non-empty, relative, existing, and documentation only | `docs_round_rejects_wrong_scope_and_changed_authorization`, `prospective_documentation_rejects_source_changes_even_when_reverted`, `operational_infrastructure_readme_is_documentation_without_source_allowance` | `scope_requires_markdown_paths` | keep |
| Grant cannot be issued while the task has a live launch | `followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation` | `grant_requires_idle` | keep |
| One unused pair at a time | `ordinary_exception_preserves_documentation_schema_marker` | `one_unused_pair` | keep |
| Preparation and pair share one receipt | `followup_preparation_does_not_grant_launches_and_changed_receipt_blocks_launch` | | rewrite: delivery grant includes its pair |

## domain::delivery

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Open-delivery requires hold, run ownership, no prior terminal state | `followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation`, `recovery_rejects_changed_receipts_partial_pairs_and_wrong_replacement_without_effects` | `open_requires_hold`, `open_requires_jira_ownership`, `open_rejects_terminal_task` | keep |
| Open-delivery requires prior delivery closed | `followup_requires_unchanged_historical_pr_and_rejects_reused_authorization` | `open_requires_closed_prior` | keep |
| Prior `Merged` commit must be on main | `followup_requires_unchanged_historical_pr_and_rejects_reused_authorization`, `followup_final_verify_uses_current_merge_and_full_cumulative_criteria` | `history_merges_stay_on_main` | keep |
| Prior `Replaced` PR must still be merged at the recorded head and merge commit | `historical_reopen_and_external_merge_drift_do_not_mutate_current_delivery`, `recovery_rejects_changed_receipts_partial_pairs_and_wrong_replacement_without_effects` | `replacement_identity_is_fixed` | keep |
| Replacement must be a distinct PR, not this or any prior delivery's | `followup_preserves_dirty_old_worker_and_refuses_historical_or_foreign_replacement` | `replacement_must_be_foreign` | keep |
| Observing a closed delivery never mutates the current one | `historical_reopen_and_external_merge_drift_do_not_mutate_current_delivery`, `completed_delivery_rejects_open_and_closed_remote_pr_without_mutation`, `completed_delivery_rejects_remote_head_and_merge_identity_drift` | `closed_delivery_is_immutable` | keep; structural, one test |
| Operations of a closed delivery are not dispatchable | `followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation` | `closed_operations_not_dispatchable` | keep; structural |
| New delivery starts from current `origin/main` with fresh sessions | `followup_missing_worker_runs_one_pair_and_publishes_without_reusing_history` | `open_starts_from_main` | keep |
| Task is Verified only when the latest closed delivery has full criteria and PASS | `followup_final_verify_uses_current_merge_and_full_cumulative_criteria`, `followup_merge_does_not_complete_operational_or_cleanup_acceptance`, `stage_pass_cannot_complete_full_task_and_end_invalidates_stage_evidence` | `verified_requires_full_delivery` | keep |
| Narrowing requires an unmerged in-flight Code delivery and distinct criterion IDs | `stage_pass_cannot_complete_full_task_and_end_invalidates_stage_evidence` | `narrowing_requires_open_code_delivery` | keep |
| Narrowing invalidates plan, evidence and review | same | `narrowing_invalidates_proof` | keep |
| Narrowed merge leaves acceptance open and archives its proof | same, `staged_github_delivery_merges_only_the_reviewed_stage_and_keeps_full_acceptance_open` | `narrowed_merge_keeps_acceptance_open` | keep |
| Narrowed delivery blocks open-delivery until merged | `changed_stage_authority_and_pending_stage_block_followup_and_launch` | `narrowing_blocks_open_delivery` | keep |
| Verification delivery admits no implementer | `staged_last_review_and_extra_stage_cycle_recover_with_fresh_final_review_authority` | `verification_rejects_implementer` | keep; structural |
| `next` on a Verification delivery is final acceptance, never a PR | `already_satisfied_work_routes_to_final_verification_without_empty_pr` | `verification_routes_to_acceptance` | keep |
| External merge without acceptance holds `NeedsHuman`, keeps counters | `premature_external_merge_without_acceptance_is_held` | `external_merge_holds` | keep |
| Every commit in `base..head` stays inside `paths`, reverted or not | `cleanup_rejects_unrelated_intermediate_change_even_when_reverted`, `prospective_documentation_rejects_source_changes_even_when_reverted`, `docs_round_rejects_wrong_scope_and_changed_authorization` | `paths_bound_every_commit` | keep; one rule for both |
| Single-approval exercise: preparation then cleanup | `same_approval_prepares_cleanup_once_after_stage_and_keeps_full_acceptance_binding`, `failed_exercise_can_remove_controls_under_frozen_cleanup_stage_without_claiming_full_proof` | | drop; mechanism deleted |

## domain::acceptance

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| PASS requires evidence for every criterion | `pass_requires_every_criterion_and_immutable_artifacts` | `pass_requires_all_criteria` | keep |
| Evidence artifacts are digest-pinned; a changed artifact fails acceptance | same, `parent_done_revalidates_evidence_at_prepare_and_dispatch` | `changed_artifact_fails_acceptance` | keep |
| Human-only criterion cannot be satisfied by command or inspection evidence | `human_acceptance_cannot_be_relabelled_as_test`, `batch_cannot_satisfy_human_criteria_with_automated_evidence` | `human_criterion_rejects_automation` | keep |
| Passing command is not acceptance without a measurement against baseline | `measurements_are_required_even_when_a_command_passed` | `acceptance_requires_measurement` | keep |
| Weakened baseline artifact fails acceptance | same | `changed_baseline_fails_acceptance` | keep |
| Proof batch is atomic; a bad entry writes nothing | `batch_is_atomic_and_invalidates_review_only_on_success` | `proof_batch_is_atomic` | keep |
| New proof invalidates the existing review, keeps count | same | `new_proof_invalidates_review` | keep |
| Head change invalidates evidence and review | `rejects_stale_review_after_head_changes` | `head_change_invalidates_proof` | keep |
| Final verification checks the merge is on main at the exact head | `complete_delivery_requires_merge_review_jira_and_allows_cleanup_check` | `final_verify_checks_main_head` | keep |
| Jira Done revalidates evidence digests | `parent_done_revalidates_evidence_at_prepare_and_dispatch` | `done_revalidates_evidence` | keep; one point, not two |

## domain::review

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Review settles only against the exact snapshot it was launched on | `rejects_stale_review_after_head_changes` | `review_requires_same_snapshot` | keep |
| Reviewer session cannot equal the implementer session | `reviewer_cannot_reuse_implementer_session` | `reviewer_session_is_separate` | keep |
| Malformed review output rejected without consuming a launch | `review_prevalidation_rejects_malformed_results_without_consuming_launches` | `malformed_review_is_free` | keep |
| Evidence gaps cannot accompany PASS | same | `pass_rejects_evidence_gaps` | keep; becomes computed verdict |
| Readiness failure spends no review budget | `readiness_rejects_missing_evidence_without_spending_review_budget` | `readiness_is_free` | keep |
| Native PASS receipt survives a failed settlement; same launch retries | `native_pass_receipt_survives_failed_settlement_and_same_launch_can_retry` | `settlement_failure_is_retryable` | keep |
| Verdict is computed from severity; Critical blocks | new, Section 9 | `critical_finding_blocks` | new |
| Reviewer-declared PASS with a critical finding is not PASS | new, Section 9 | `verdict_ignores_reviewer_claim` | new |
| Every finding must be dispositioned before merge | `engine::reviewed` invariant, no direct test | `merge_requires_dispositions` | new |
| Human-review mode needs a receipt for the exact snapshot; changes void it | `human_review_mode_requires_a_current_receipt_and_invalidates_it_on_changes`, `human_mode_guards_merge_preparation_and_execution` | `human_receipt_pins_snapshot`, `human_receipt_gates_merge` | keep |

## domain::publish

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| CI hold rejected before the per-head deadline | `cannot_defer_ci_before_deadline` | `ci_hold_requires_deadline` | keep |
| Deadline survives hold and resume | `deadline_survives_hold_and_resume` | `ci_deadline_survives_hold` | keep |
| Merge requires PASS, green required checks, exact head match | `complete_delivery_requires_merge_review_jira_and_allows_cleanup_check` | `merge_requires_pass_checks_head` | keep |
| Merge records `Merged` only from a verified GitHub MERGED observation | same | `merged_requires_observation` | keep |
| Narrowed merge merges only the reviewed head | `staged_github_delivery_merges_only_the_reviewed_stage_and_keeps_full_acceptance_open` | `merge_uses_reviewed_head` | keep |
| Complete requires Merged, Verified and confirmed Jira Done | `complete_delivery_requires_merge_review_jira_and_allows_cleanup_check` | `complete_requires_verified_and_done` | keep |
| Reconciliation reuses confirmed PR receipts and re-observes unknown ones | `reconciliation_reuses_complete_prs_but_selects_uncertain_operations` | `reconcile_reobserves_unknown_only` | keep |

## domain::sync

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Unknown transition blocks a second intent and a hold until observed | `unknown_jira_outcome_blocks_replay_and_reconciles_success` | `unknown_status_blocks_replay` | keep |
| Observation of the target status confirms without another attempt | same | `observe_confirms_success` | keep |
| Retry is bounded to one | `jira_retry_is_bounded_and_does_not_reopen_external_done` | `status_retry_is_bounded` | keep |
| Older read cannot overwrite a confirmed status | `old_jira_receipt_cannot_overwrite_confirmed_state` | `stale_read_cannot_overwrite` | keep |
| Subtask Done needs a recorded subtask | `cannot_mark_unrecorded_subtask_done` | `subtask_done_requires_record` | keep |
| Missing or ambiguous transition to target is rejected | `transition_for`, no direct test | `ambiguous_transition_rejected` | new |

## domain::progress

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Replan cannot change a baseline target | `replanning_cannot_rewrite_the_starting_condition` | `replan_keeps_baseline` | keep |
| Out-of-plan paths need a scope reason; size is reported | `scope_checkpoint_reports_outside_paths_and_requires_reassessment` | `checkpoint_requires_scope_reason` | keep |
| Integration snapshot can be checkpointed without a launch | same | `checkpoint_allows_integration` | keep |
| Checkpoint required after each implementer turn before the next | `stalled_implementation_is_bounded_across_reopen_and_replan` | `turn_requires_checkpoint` | keep |
| Feedback is pinned to committed `origin/main` at plan time | `scoped_feedback_is_pinned_and_injected_into_native_and_managed_launches` | `feedback_pins_to_main` | keep |
| Only accepted lessons in matching families reach the plan | same, `additional_lesson_tags_select_only_accepted_relevant_guidance` | `plan_selects_accepted_lessons` | keep |
| Lesson acceptance requires a Verified delivery; proposals are excluded | `proposed_lessons_require_verified_delivery_before_reuse` | `lesson_acceptance_requires_verified` | keep |
| Accepted lesson registers once | same | `lesson_registers_once` | keep |
| Legacy plan wire shape | `empty_tags_preserve_the_legacy_plan_wire_contract_after_state_rewrite` | | drop |
| Legacy state without policy needs configuration | `legacy_state_requires_configuration_without_resetting_budgets` | | drop; no migration |
| Tier computed from signals; escalation only upward | new, Section 9 | `tier_never_lowers` | new |

## app

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Concurrent launches of the same task have one winner | `concurrent_launch_claims_have_one_winner` | `launch_has_one_winner` | keep; identity via `adapters` |
| Unknown fields and unknown roles rejected with no writes | `malformed_commands_fail_closed` | `unknown_field_fails_closed` | keep |
| Unknown operation before its gate settles failed without claiming execution | `startup_crash_before_gate_is_recoverable_without_claiming_execution` | `pre_gate_crash_settles_failed` | keep |
| Surviving descendant can be terminated and the operation settled | `surviving_owned_descendant_can_be_stopped_after_leader_exits` | `orphan_process_is_stoppable` | keep |
| Publish failure settles unknown, then failed on fresh evidence, then retries without duplicate | `gh_wrapper_reconciles_create_and_definite_failure_without_duplicate` | `publish_failure_no_duplicate` | keep |
| Check with missing executable is failed, not unknown | `missing_check_executable_is_failed_not_permanently_unknown` | `missing_check_binary_fails` | keep |
| Check records real exit status and evidence | `runtime_check_executes_and_records_real_exit_status` | `check_records_exit_status` | keep |
| `--check` runs decide and writes nothing | new, Section 15 | `check_flag_writes_nothing` | new |
| Every command returns the events it wrote | new, Section 15 | `command_returns_events` | new |
| Projection equals fold of events | new, Section 8 | `status_matches_fold` | new |
| Old schema is not migrated | `followup_old_schema_read_does_not_migrate_and_new_schema_rejects_future_versions` | | drop |

## adapters

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Codex argv carries model, effort, sandbox, session resume | `codex_runner_controls_model_effort_session_and_counts_usage` | `codex_argv_matches_launch` | adapter |
| Codex stream yields session id and usage | same | `codex_stream_yields_usage` | adapter |
| Pi argv carries model, thinking, tools, session, approve | new, Section 14 | `pi_argv_matches_launch` | adapter |
| Pi final message comes from `agent_end`; usage from `message_update` | new | `pi_stream_yields_final_and_usage` | adapter |
| Launch context is injected unchanged into the prompt | `scoped_feedback_is_pinned_and_injected_into_native_and_managed_launches` | `harness_injects_context` | adapter |
| Missing usage does not block settlement; report marks unknown | `missing_usage_does_not_block_settlement_and_empty_report_marks_unknown`, `absent_cache_counts_remain_unknown_and_malformed_usage_can_be_recovered` | `missing_usage_is_unknown` | keep, domain |
| Absent cache count stays null, never zero | `absent_cache_counts_remain_unknown_and_malformed_usage_can_be_recovered` | `cache_count_stays_null` | keep, domain |
| Malformed usage is warned and dropped, not fabricated | same | `malformed_usage_is_dropped` | keep, domain |
| Report never infers cost | `usage_import_subtracts_cumulative_counters_and_counts_each_launch_once` | `report_has_no_cost` | keep, domain |
| Cumulative log import, line intervals, session matching | `usage_import_*`, `usage_rejects_*`, `bound_usage_imports_on_finish_and_sync_never_counts_later_turns` | | drop; capture is automatic |
| GitHub observation parses state, head, merge commit, checks | `complete_delivery_*`, `premature_external_merge_*` | `github_observation_parses_pr` | adapter |
| Cross-run claim reservation via git | `cross_run_issue_claims_are_exclusive` | `git_reservation_is_exclusive` | adapter |

## xtask

| Rule | Test | Name | Decision |
| --- | --- | --- | --- |
| Comments and strings are not branches | `comments_and_strings_do_not_count_as_branches` | same | keep |
| Threshold is inclusive | `threshold_is_inclusive` | same | keep |
| Function over threshold fails | `excessive_function_complexity_fails` | same | keep |
| Closure complexity counts in its parent | `excessive_closure_complexity_cannot_hide_in_parent` | same | keep |
| Malformed Rust and empty scope fail | `malformed_rust_fails`, `empty_scope_fails` | same | keep |
| `std::process` outside `adapters` fails | new, Section 12 | `process_outside_adapters_fails` | new |
| File over 400 lines fails | new, Section 12 | `long_file_fails` | new |

## Totals

| Decision | Rules |
| --- | --- |
| keep | 97 |
| new | 17 |
| adapter | 7 |
| rewrite | 3 |
| drop | 13 |
