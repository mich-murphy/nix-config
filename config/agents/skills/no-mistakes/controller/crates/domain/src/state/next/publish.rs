use crate::{
    acceptance::Snapshot,
    command::{NextAction, PublishStep},
    delivery::{CheckState, Delivery, DeliveryKind, PrState, PullRequest},
    ids::Sha,
    task::Task,
};

/// The next publish step for an open code delivery, or the sole action for
/// a verification delivery (final acceptance, never a PR). Once required
/// checks are pending, a deadline already recorded by a prior
/// `poll-checks --wait` decides between waiting it out and asking the
/// coordinator to start that wait. A merge the run's policy gates behind
/// a receipt asks for that receipt instead of naming the merge.
pub(super) fn publish_action(
    task: &Task,
    delivery: &Delivery,
    autonomous_merge: bool,
) -> Option<NextAction> {
    match &delivery.kind {
        DeliveryKind::Verification { .. } => Some(NextAction::FinalVerify {
            task: task.id.clone(),
            commit: delivery.base.clone(),
        }),
        DeliveryKind::Code { pr: None } => Some(NextAction::Publish {
            task: task.id.clone(),
            step: PublishStep::Create,
        }),
        DeliveryKind::Code { pr: Some(pr) } if pr.state == PrState::Open && pr.draft => {
            Some(NextAction::Publish {
                task: task.id.clone(),
                step: PublishStep::Ready,
            })
        }
        DeliveryKind::Code { pr: Some(pr) } if pr.state == PrState::Open => {
            Some(open_pr_action(task, delivery, pr, autonomous_merge))
        }
        DeliveryKind::Code { pr: Some(_) } => None,
    }
}

fn open_pr_action(
    task: &Task,
    delivery: &Delivery,
    pr: &PullRequest,
    autonomous_merge: bool,
) -> NextAction {
    let pending = pr
        .checks
        .iter()
        .any(|check| check.required && check.state == CheckState::Pending);
    if pending {
        return delivery.check_deadlines.get(&pr.head).map_or(
            NextAction::PollChecks {
                task: task.id.clone(),
                pr: pr.number,
            },
            |deadline| NextAction::AwaitChecks {
                task: task.id.clone(),
                pr: pr.number,
                deadline: *deadline,
            },
        );
    }
    if !autonomous_merge && !has_merge_receipt(task) {
        return NextAction::RequestMergeAuthority {
            task: task.id.clone(),
        };
    }
    NextAction::Publish {
        task: task.id.clone(),
        step: PublishStep::Merge,
    }
}

/// An unused `Grant::Merge` on the task: the receipt a receipt-required
/// run consumes at the merge. A used one never reopens.
fn has_merge_receipt(task: &Task) -> bool {
    task.authorities.iter().any(|authority| {
        matches!(authority.grant, crate::authority::Grant::Merge) && authority.used.whole.is_none()
    })
}

pub(super) fn verification_action(
    task: &Task,
    delivery: &Delivery,
    commit: &Sha,
) -> Option<NextAction> {
    if let Some(action) = unfixed_baseline(task, delivery) {
        return Some(action);
    }
    let missing = delivery
        .criteria
        .iter()
        .filter(|id| !delivery.proof.entries.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(NextAction::RecordProof {
            task: task.id.clone(),
            missing,
        });
    }
    delivery.review.is_none().then(|| NextAction::Review {
        task: task.id.clone(),
        snapshot: Snapshot {
            base: commit.clone(),
            head: commit.clone(),
            requirements: task.spec.requirements.clone(),
        },
    })
}

/// A verification delivery opened without a prior plan (work already on
/// main) has no baseline fixed for its automated criteria yet; proof
/// cannot be measured until `plan` records one.
fn unfixed_baseline(task: &Task, delivery: &Delivery) -> Option<NextAction> {
    let unfixed = delivery.criteria.iter().any(|id| {
        !task.baselines.contains_key(id)
            && task
                .spec
                .criteria
                .iter()
                .any(|criterion| criterion.id == *id && !criterion.human_only)
    });
    unfixed.then(|| NextAction::Plan {
        task: task.id.clone(),
    })
}
