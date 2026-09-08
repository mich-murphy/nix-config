use super::State;
use super::apply_delivery::{
    close_delivery, disposition, finish_launch, invalidate_proof, narrow, observe_pr, record_proof,
    set_sync, settle_operation, settle_review, spend_pair, use_authority, verify, with_task,
};
use super::projection::{
    apply_discovery, apply_refresh, bind, brief, checkpoint, claim, plan, raise_tier, release,
    snapshot,
};
use crate::{
    budget,
    event::Event,
    sync::Sync,
    task::{Phase, Question},
};

pub fn apply(state: &mut State, event: &Event) {
    match event {
        Event::RunInitialized {
            config,
            profile,
            capabilities,
        } => {
            state.config = Some((**config).clone());
            state.profile = Some((**profile).clone());
            state.capabilities = Some(*capabilities);
        }
        Event::QueueDiscovered { tasks, order, .. } => apply_discovery(state, tasks, order),
        Event::QueueRefreshed { changed, removed } => apply_refresh(state, changed, removed),
        Event::QuestionRaised { task, text } => state.questions.push(Question {
            task: task.clone(),
            text: text.clone(),
        }),
        Event::Claimed { task, .. } => claim(state, task),
        Event::Held { task, reason, at } => {
            if let crate::task::HoldReason::CiPending { head, deadline, .. } = reason {
                state
                    .check_deadlines
                    .insert(format!("{task}:{head}"), *deadline);
            }
            with_task(state, task, |value| {
                value.hold = Some(crate::task::Hold {
                    since: *at,
                    reason: reason.clone(),
                });
            });
            if state.active.as_ref() == Some(task) {
                state.active = None;
            }
        }
        Event::Resumed { task } => {
            state.active = Some(task.clone());
            with_task(state, task, |value| value.hold = None);
        }
        Event::Briefed {
            task,
            criteria,
            requirements,
            tier,
            provisional,
        } => brief(state, task, criteria, requirements, *tier, *provisional),
        Event::TierRaised {
            task,
            to,
            reason,
            at,
        } => raise_tier(state, task, *to, reason, *at),
        Event::SlotBound {
            task,
            binding,
            branch,
            base,
        } => bind(state, task, binding, branch, base),
        Event::SlotReleased { task, .. } => release(state, task),
        Event::Planned {
            task,
            plan: value,
            feedback,
        } => plan(state, task, value, feedback),
        Event::Snapshotted {
            task,
            delivery,
            snapshot: value,
            covers,
            ..
        } => snapshot(state, task, *delivery, value, *covers),
        Event::Checkpointed {
            task,
            launch,
            advanced,
            stalled,
            ..
        } => checkpoint(state, task, *launch, *advanced, *stalled),
        Event::SubtaskRecorded {
            task,
            issue,
            subtask,
        } => with_task(state, task, |value| {
            value.subtasks.insert(issue.clone(), subtask.clone());
        }),
        Event::Excluded { task, phase } => {
            with_task(state, task, |value| value.phase = phase.clone())
        }
        Event::LaunchStarted { launch } => state.launches.push(launch.clone()),
        Event::LaunchEnded {
            launch,
            result,
            usage,
        } => {
            state.usage.insert(*launch, *usage);
            finish_launch(state, *launch, result);
        }
        Event::ProofRecorded {
            task,
            delivery,
            entries,
        } => record_proof(state, task, *delivery, entries),
        Event::ProofInvalidated { task, delivery, .. } => invalidate_proof(state, task, *delivery),
        Event::ReviewSettled {
            task,
            delivery,
            review,
        } => settle_review(state, task, *delivery, review),
        Event::Dispositioned {
            task,
            finding,
            disposition: value,
        } => disposition(state, task, finding, value),
        Event::HumanReviewed {
            task,
            snapshot,
            receipt,
        } => {
            state
                .human_reviews
                .insert(task.clone(), (snapshot.clone(), receipt.clone()));
        }
        Event::LessonRecorded { task, lesson } => {
            state.lessons.push((task.clone(), lesson.clone()))
        }
        Event::DeliveryOpened { task, delivery } => with_task(state, task, |value| {
            let first = value.deliveries.is_empty();
            if let Some(authority_id) = delivery.authority
                && let Some(authority) = value
                    .authorities
                    .iter_mut()
                    .find(|entry| entry.id == authority_id)
                && crate::authority::unused_pair(authority)
            {
                authority.grant = crate::authority::Grant::Delivery {
                    kind: delivery.kind.clone(),
                    criteria: delivery.criteria.clone(),
                    paths: delivery.paths.clone(),
                };
            }
            value.deliveries.push((**delivery).clone());
            if !first {
                value.phase = match &delivery.kind {
                    crate::delivery::DeliveryKind::Code { .. } => Phase::Claimed,
                    crate::delivery::DeliveryKind::Verification { of } => Phase::Merged {
                        delivery: delivery.id,
                        commit: of.clone(),
                    },
                };
            }
        }),
        Event::AcceptanceNarrowed {
            task,
            delivery,
            criteria,
            ..
        } => narrow(state, task, *delivery, criteria),
        Event::OperationStarted { operation } => state.operations.push(operation.clone()),
        Event::OperationSettled {
            operation, status, ..
        } => settle_operation(state, *operation, *status),
        Event::PrObserved { task, delivery, pr } => observe_pr(state, task, *delivery, pr),
        Event::DeliveryClosed {
            task,
            delivery,
            outcome,
        } => close_delivery(state, task, *delivery, outcome),
        Event::Verified { task, commit } => verify(state, task, commit),
        Event::Completed { task } => {
            state.completed.insert(task.clone());
        }
        Event::StatusIntended {
            task,
            issue,
            current,
            target,
            transition,
            operation,
            attempts,
            at,
        } => set_sync(
            state,
            task,
            issue,
            Sync::Unknown(crate::sync::StatusIntent {
                operation: *operation,
                from: current.clone(),
                target: target.clone(),
                transition: transition.clone(),
                attempts: *attempts,
                at: *at,
            }),
        ),
        Event::StatusObserved {
            task, issue, sync, ..
        } => set_sync(state, task, issue, sync.clone()),
        Event::AuthorityRegistered { task, authority } => with_task(state, task, |value| {
            value.authorities.push((**authority).clone())
        }),
        Event::GrantUsed {
            task,
            authority,
            by,
        } => use_authority(state, task, *authority, *by),
        Event::PairSpent {
            task,
            authority,
            launch,
            implementation,
        } => spend_pair(state, task, *authority, *launch, *implementation),
        Event::BudgetSpent {
            task,
            budget,
            counted,
        } => with_task(state, task, |value| {
            budget::spend(&mut value.budgets, *budget, *counted)
        }),
    }
}
