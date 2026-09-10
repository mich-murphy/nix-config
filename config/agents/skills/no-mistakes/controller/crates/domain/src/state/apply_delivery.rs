use super::State;
use crate::{
    acceptance::Snapshot,
    authority::Grant,
    delivery::{DeliveryKind, Outcome},
    event::OperationStatus,
    ids::{AuthorityId, Digest, FindingId, IssueKey, TaskId},
    review::Disposition,
    sync::Sync,
    task::{Phase, Receipt, Task},
};

/// Records the `poll-checks --wait` deadline for `head` on the task's
/// current delivery, the first time a `CiPending` hold observes it.
pub(super) fn record_ci_deadline(
    state: &mut State,
    task: &TaskId,
    head: &crate::ids::Sha,
    deadline: crate::Instant,
) {
    with_task(state, task, |value| attach_deadline(value, head, deadline));
}

fn attach_deadline(task: &mut Task, head: &crate::ids::Sha, deadline: crate::Instant) {
    if let Some(delivery) = task.deliveries.last_mut() {
        delivery.check_deadlines.insert(head.clone(), deadline);
    }
}

/// Pushes a launch onto both the task-wide launch log and the delivery it
/// belongs to, so `validate_reuse`'s "unstarted delivery" check and "every
/// operation belongs to a delivery" hold structurally.
pub(super) fn start_launch(state: &mut State, launch: &crate::event::Launch) {
    with_task(state, &launch.task, |value| attach_launch(value, launch));
    state.launches.push(launch.clone());
}

fn attach_launch(task: &mut Task, launch: &crate::event::Launch) {
    if let Some(delivery) = task
        .deliveries
        .iter_mut()
        .find(|item| item.id == launch.delivery)
    {
        delivery.launches.push(launch.id);
    }
}

/// The `Operation` counterpart of `start_launch`.
pub(super) fn start_operation(state: &mut State, operation: &crate::event::Operation) {
    with_task(state, &operation.task, |value| {
        attach_operation(value, operation);
    });
    state.operations.push(operation.clone());
}

fn attach_operation(task: &mut Task, operation: &crate::event::Operation) {
    if let Some(delivery) = task
        .deliveries
        .iter_mut()
        .find(|item| item.id == operation.delivery)
    {
        delivery.operations.push(operation.id);
    }
}

/// Pushes the delivery and moves the phase on: `Claimed` for a new code
/// delivery, `Merged` (at the commit under verification) for a
/// `Verification` one. A first code delivery moves no phase; `bind-slot`
/// already set `Planned` for it. A first verification delivery (work
/// already on main) has no slot, so it moves to `Merged` here.
pub(super) fn open_delivery(state: &mut State, task: &TaskId, delivery: crate::delivery::Delivery) {
    with_task(state, task, |value| {
        let first_code =
            value.deliveries.is_empty() && matches!(delivery.kind, DeliveryKind::Code { .. });
        value.deliveries.push(delivery.clone());
        if !first_code {
            value.phase = opened_phase(&delivery);
        }
    });
}

fn opened_phase(delivery: &crate::delivery::Delivery) -> Phase {
    match &delivery.kind {
        DeliveryKind::Code { .. } => Phase::Claimed,
        DeliveryKind::Verification { of } => Phase::Merged {
            delivery: delivery.id,
            commit: of.clone(),
        },
    }
}

pub(super) fn finish_launch(
    state: &mut State,
    launch: crate::ids::LaunchId,
    result: &crate::event::LaunchOutcome,
    usage: Option<crate::ports::Tokens>,
) {
    if let Some(item) = state.launches.iter_mut().find(|item| item.id == launch) {
        item.outcome = Some(result.clone());
        item.usage = usage;
    }
}

pub(super) fn record_proof(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    entries: &[crate::acceptance::ProofEntry],
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            add_proof(item, entries);
        }
    });
}

fn add_proof(delivery: &mut crate::delivery::Delivery, entries: &[crate::acceptance::ProofEntry]) {
    for entry in entries {
        delivery
            .proof
            .entries
            .insert(entry.criterion.clone(), entry.clone());
    }
    if delivery.review.is_some() {
        delivery.prior_review = delivery.review.take();
    }
    delivery.human_review = None;
}

pub(super) fn invalidate_proof(state: &mut State, task: &TaskId, delivery: crate::ids::DeliveryId) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            item.proof.entries.clear();
            item.review = None;
        }
    });
}

pub(super) fn settle_review(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    review: &crate::review::Review,
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            item.review = Some(review.clone());
        }
    });
}

pub(super) fn disposition(
    state: &mut State,
    task: &TaskId,
    finding: &FindingId,
    disposition: &Disposition,
) {
    with_task(state, task, |value| {
        if let Some(review) = value
            .deliveries
            .last_mut()
            .and_then(|delivery| delivery.review.as_mut())
        {
            review
                .dispositions
                .insert(finding.clone(), disposition.clone());
        }
    });
}

pub(super) fn narrow(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    criteria: &[crate::ids::CriterionId],
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            item.criteria = criteria.to_vec();
            item.proof.entries.clear();
            item.review = None;
            item.human_review = None;
            clear_delivery_plan(item);
        }
    });
}

fn clear_delivery_plan(delivery: &mut crate::delivery::Delivery) {
    if let Some(work) = delivery.work.as_mut() {
        work.plan = None;
        work.snapshot = None;
    }
}

pub(super) fn settle_operation(
    state: &mut State,
    operation: crate::ids::OperationId,
    status: OperationStatus,
) {
    if let Some(item) = state
        .operations
        .iter_mut()
        .find(|item| item.id == operation)
    {
        item.status = status;
    }
}

pub(super) fn observe_pr(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    pr: &crate::delivery::PullRequest,
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery)
            && let DeliveryKind::Code { pr: current } = &mut item.kind
        {
            *current = Some(pr.clone());
        }
    });
}

pub(super) fn close_delivery(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    outcome: &Outcome,
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            item.outcome = outcome.clone();
        }
        if let Outcome::Merged { commit, .. } = outcome {
            value.phase = Phase::Merged {
                delivery,
                commit: commit.clone(),
            };
        }
    });
}

pub(super) fn verify(state: &mut State, task: &TaskId, receipt: &Receipt) {
    with_task(state, task, |value| {
        value.phase = Phase::Verified {
            receipt: receipt.clone(),
        };
    });
}

/// `Completed` carries forward the receipt `Verified` already recorded:
/// the fact a task was verified does not change when the run confirms it,
/// so `Phase::Completed` simply keeps it rather than asking the handler
/// to resupply the same value.
pub(super) fn complete(state: &mut State, task: &TaskId) {
    with_task(state, task, |value| {
        if let Phase::Verified { receipt } = value.phase.clone() {
            value.phase = Phase::Completed { receipt };
        }
    });
    // A task that never took a slot has no cleanup to wait for, so it
    // stops being the active task here rather than at `SlotReleased`.
    let unowned = state
        .tasks
        .get(task)
        .is_some_and(|value| value.current_work().is_none());
    if unowned && state.active.as_ref() == Some(task) {
        state.active = None;
    }
}

pub(super) fn judge(state: &mut State, task: &TaskId, judgment: &crate::judge::Judgment) {
    with_task(state, task, |value| value.judgment = Some(judgment.clone()));
}

pub(super) fn set_human_review(
    state: &mut State,
    task: &TaskId,
    snapshot: &Snapshot,
    receipt: &Digest,
) {
    with_task(state, task, |value| {
        if let Some(delivery) = value.deliveries.last_mut() {
            delivery.human_review = Some(crate::delivery::HumanReceipt {
                snapshot: snapshot.clone(),
                receipt: receipt.clone(),
            });
        }
    });
}

/// The explicit event side of repurposing an unused `Pair` grant as the
/// next delivery's grant (design Section 5): the handler already decided
/// this is safe; `apply` only records the resulting `grant`.
pub(super) fn repurpose_authority(
    state: &mut State,
    task: &TaskId,
    authority: AuthorityId,
    grant: &Grant,
) {
    with_task(state, task, |value| {
        if let Some(entry) = value
            .authorities
            .iter_mut()
            .find(|item| item.id == authority)
        {
            entry.grant = grant.clone();
        }
    });
}

pub(super) fn spend_pair(
    state: &mut State,
    task: &TaskId,
    authority: crate::ids::AuthorityId,
    launch: crate::ids::LaunchId,
    implementation: bool,
) {
    with_task(state, task, |value| {
        if let Some(item) = value
            .authorities
            .iter_mut()
            .find(|item| item.id == authority)
        {
            let _result = crate::authority::spend_pair(item, implementation, launch);
        }
    });
}

pub(super) fn use_authority(
    state: &mut State,
    task: &TaskId,
    authority: crate::ids::AuthorityId,
    by: crate::ids::UseId,
) {
    with_task(state, task, |value| {
        if let Some(item) = value
            .authorities
            .iter_mut()
            .find(|item| item.id == authority)
        {
            item.used.whole = Some(by);
        }
    });
}

pub(super) fn set_sync(state: &mut State, task: &TaskId, issue: &IssueKey, sync: Sync) {
    with_task(state, task, |value| {
        if IssueKey::from(value.id.clone()) == *issue {
            value.sync = sync;
        } else if let Some(subtask) = value.subtasks.get_mut(issue) {
            subtask.sync = sync;
        }
    });
}

pub(super) fn with_task(state: &mut State, task: &TaskId, update: impl FnOnce(&mut Task)) {
    if let Some(value) = state.tasks.get_mut(task) {
        update(value);
    }
}
