use crate::{
    command::NextAction,
    delivery::{Delivery, DeliveryKind, PrState},
    ids::{IssueKey, JiraStatus, TaskId},
    sync::Sync,
    task::{HoldReason, Task},
};

/// One step of Jira synchronisation toward `target` for `issue`. Returns
/// `None` when `sync` already confirms `target`, meaning the caller should
/// proceed past this checkpoint. An unknown intent yields observation
/// first; an intent that has already failed three times (the bound `sync::intend`
/// enforces) yields a hold instead of looping on another `SyncStatus`.
pub(super) fn sync_step(
    task: &TaskId,
    issue: &IssueKey,
    sync: &Sync,
    target: &JiraStatus,
) -> Option<NextAction> {
    match sync {
        Sync::Confirmed(receipt) if &receipt.status == target => None,
        Sync::Unknown(_) => Some(NextAction::ObserveStatus {
            task: task.clone(),
            issue: issue.clone(),
        }),
        Sync::Failed(failure) if failure.intent.attempts >= 3 => Some(NextAction::Hold {
            task: task.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "Jira synchronisation failed three times".into(),
                remaining: Vec::new(),
            },
        }),
        Sync::Pending | Sync::Confirmed(_) | Sync::Failed(_) => Some(NextAction::SyncStatus {
            task: task.clone(),
            issue: issue.clone(),
            target: target.clone(),
        }),
    }
}

impl super::super::State {
    /// Once the current code delivery has a PR (draft or ready), Jira must
    /// reflect Review before any further publish step (design Section 11).
    pub(super) fn review_sync_gate(&self, task: &Task, delivery: &Delivery) -> Option<NextAction> {
        let DeliveryKind::Code { pr: Some(pr) } = &delivery.kind else {
            return None;
        };
        if pr.state != PrState::Open {
            return None;
        }
        let review = self.config.as_ref()?.jira.as_ref()?.statuses.review.clone();
        let issue = IssueKey::from(task.id.clone());
        sync_step(&task.id, &issue, &task.sync, &review)
    }

    /// After `Verified`, the external tracker (when the run has one)
    /// must confirm Done for the parent and every recorded subtask
    /// before `Complete` (design Section 11). Without one, verified
    /// delivery is itself the completion bar.
    pub(super) fn verified_action(&self, task: &Task) -> Option<NextAction> {
        let Some(done) = self
            .config
            .as_ref()?
            .jira
            .as_ref()
            .map(|jira| jira.statuses.done.clone())
        else {
            return Some(NextAction::Complete {
                task: task.id.clone(),
            });
        };
        let issue = IssueKey::from(task.id.clone());
        if let Some(action) = sync_step(&task.id, &issue, &task.sync, &done) {
            return Some(action);
        }
        if let Some(action) = subtasks_sync_step(task, &done) {
            return Some(action);
        }
        Some(NextAction::Complete {
            task: task.id.clone(),
        })
    }
}

fn subtasks_sync_step(task: &Task, target: &crate::ids::JiraStatus) -> Option<NextAction> {
    task.subtasks
        .iter()
        .find_map(|(issue, subtask)| sync_step(&task.id, issue, &subtask.sync, target))
}
