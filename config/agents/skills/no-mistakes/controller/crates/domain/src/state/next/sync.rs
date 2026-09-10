use crate::{
    command::NextAction,
    ids::{IssueKey, JiraStatus, TaskId},
    sync::Sync,
    task::HoldReason,
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
