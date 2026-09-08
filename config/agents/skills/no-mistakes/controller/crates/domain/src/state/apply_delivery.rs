use super::State;
use crate::{
    acceptance::Snapshot,
    delivery::{DeliveryKind, Outcome},
    event::OperationStatus,
    ids::{FindingId, Sha, TaskId},
    review::{Disposition, Verdict},
    task::{Phase, Receipt, Task},
};
use std::collections::BTreeMap;

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
    launch: crate::ids::LaunchId,
    findings: &[crate::review::Finding],
    gaps: &[String],
    verdict: Verdict,
) {
    let snapshot = state.tasks.get(task).and_then(current_snapshot);
    with_task(state, task, |value| {
        if let (Some(item), Some(snapshot)) = (
            value.deliveries.iter_mut().find(|item| item.id == delivery),
            snapshot,
        ) {
            item.review = Some(crate::review::Review {
                launch,
                session: String::new(),
                snapshot,
                findings: findings.to_vec(),
                evidence_gaps: gaps.to_vec(),
                reviewer_opinion: verdict,
                verdict,
                dispositions: BTreeMap::new(),
            });
        }
    });
}

pub(super) fn current_snapshot(task: &Task) -> Option<Snapshot> {
    match &task.phase {
        Phase::Planned { work } | Phase::InFlight { work, .. } => work.snapshot.clone(),
        _ => None,
    }
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

pub(super) fn with_task(state: &mut State, task: &TaskId, update: impl FnOnce(&mut Task)) {
    if let Some(value) = state.tasks.get_mut(task) {
        update(value);
    }
}
