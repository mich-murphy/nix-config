use super::State;
use super::apply_delivery::{
    close_delivery, complete, disposition, finish_launch, invalidate_proof, narrow, observe_pr,
    open_delivery, record_ci_deadline, record_proof, repurpose_authority, set_human_review,
    set_sync, settle_operation, settle_review, spend_pair, start_launch, start_operation,
    use_authority, verify, with_task,
};
use super::projection::{
    apply_discovery, apply_refresh, bind, brief, checkpoint, claim, plan, raise_tier, release,
    snapshot,
};
use crate::{budget, event::Event, state::RecordedLesson, sync::Sync, task::Question};

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
                record_ci_deadline(state, task, head, *deadline);
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
        Event::LaunchStarted { launch } => start_launch(state, launch),
        Event::LaunchEnded {
            launch,
            result,
            usage,
        } => finish_launch(state, *launch, result, *usage),
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
        } => set_human_review(state, task, snapshot, receipt),
        Event::LessonRecorded { task, lesson } => state.lessons.push(RecordedLesson {
            task: task.clone(),
            lesson: lesson.clone(),
        }),
        Event::DeliveryOpened { task, delivery } => {
            open_delivery(state, task, (**delivery).clone());
        }
        Event::AuthorityRepurposed {
            task,
            authority,
            grant,
        } => repurpose_authority(state, task, *authority, grant),
        Event::AcceptanceNarrowed {
            task,
            delivery,
            criteria,
            ..
        } => narrow(state, task, *delivery, criteria),
        Event::OperationStarted { operation } => start_operation(state, operation),
        Event::OperationSettled {
            operation, status, ..
        } => settle_operation(state, *operation, *status),
        Event::PrObserved { task, delivery, pr } => observe_pr(state, task, *delivery, pr),
        Event::DeliveryClosed {
            task,
            delivery,
            outcome,
        } => close_delivery(state, task, *delivery, outcome),
        Event::Verified { task, receipt } => verify(state, task, receipt),
        Event::Completed { task } => complete(state, task),
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
