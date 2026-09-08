use crate::{
    acceptance::Snapshot,
    command::{NextAction, PublishStep},
    delivery::{CheckState, Delivery, DeliveryKind, PrState, PullRequest},
    ids::Sha,
    state::State,
    task::Task,
};

/// The next publish step for an open code delivery, or the sole action for
/// a verification delivery (final acceptance, never a PR). Once required
/// checks are pending, a deadline already recorded by a prior
/// `poll-checks --wait` decides between waiting it out and asking the
/// coordinator to start that wait.
pub(super) fn publish_action(
    state: &State,
    task: &Task,
    delivery: &Delivery,
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
            Some(open_pr_action(state, task, pr))
        }
        DeliveryKind::Code { pr: Some(_) } => None,
    }
}

fn open_pr_action(state: &State, task: &Task, pr: &PullRequest) -> NextAction {
    let pending = pr
        .checks
        .iter()
        .any(|check| check.required && check.state == CheckState::Pending);
    if !pending {
        return NextAction::Publish {
            task: task.id.clone(),
            step: PublishStep::Merge,
        };
    }
    let key = format!("{}:{}", task.id, pr.head);
    state.check_deadlines.get(&key).map_or(
        NextAction::PollChecks {
            task: task.id.clone(),
            pr: pr.number,
        },
        |deadline| NextAction::AwaitChecks {
            task: task.id.clone(),
            pr: pr.number,
            deadline: *deadline,
        },
    )
}

pub(super) fn verification_action(
    task: &Task,
    delivery: &Delivery,
    commit: &Sha,
) -> Option<NextAction> {
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
