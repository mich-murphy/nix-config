use super::State;
use crate::{
    delivery::{DeliveryKind, Outcome},
    event::OperationStatus,
    ids::{FindingId, IssueKey, Sha, TaskId},
    review::Disposition,
    sync::Sync,
    task::{Phase, Receipt, Task},
};

pub(super) fn finish_launch(
    state: &mut State,
    launch: crate::ids::LaunchId,
    result: &crate::event::LaunchOutcome,
) {
    if let Some(item) = state.launches.iter_mut().find(|item| item.id == launch) {
        item.session = match result {
            crate::event::LaunchOutcome::Completed { session, .. } => Some(session.clone()),
            crate::event::LaunchOutcome::Failed { .. } | crate::event::LaunchOutcome::Cancelled => {
                Some(String::new())
            }
        };
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
            for entry in entries {
                item.proof
                    .entries
                    .insert(entry.criterion.clone(), entry.clone());
            }
            item.review = None;
        }
    });
    state.human_reviews.remove(task);
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
            if let Some(work) = item.work.as_mut() {
                work.plan = None;
                work.snapshot = None;
            }
        }
        match &mut value.phase {
            Phase::Planned { work } | Phase::InFlight { work, .. } => {
                work.plan = None;
                work.snapshot = None;
            }
            _ => {}
        }
    });
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
            if let Outcome::Merged { commit, .. } = outcome {
                value.phase = Phase::Merged {
                    delivery,
                    commit: commit.clone(),
                };
            }
        }
    });
}

pub(super) fn verify(state: &mut State, task: &TaskId, commit: &Sha) {
    with_task(state, task, |value| {
        value.phase = Phase::Verified {
            receipt: Receipt::Delivery {
                commit: commit.clone(),
            },
        };
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
