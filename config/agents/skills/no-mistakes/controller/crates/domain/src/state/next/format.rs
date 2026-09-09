use crate::{
    command::{NextAction, schema::Template},
    ids::TaskId,
};
use std::collections::BTreeMap;

/// The bare `Command` tag this action performs. `app` looks this up in
/// the real `Command` schema to fill in the action's own schema and its
/// filled command line; domain only ever needs to say which one it is.
pub(super) fn command_for(action: &NextAction) -> String {
    match action {
        NextAction::Claim { .. } => "claim",
        NextAction::RecoverUnfinished { .. }
        | NextAction::Report { .. }
        | NextAction::AnswerQuestions { .. }
        | NextAction::Hold { .. } => "status",
        NextAction::MonitorLaunch { .. } | NextAction::SettleOperation { .. } => {
            "recover-operation"
        }
        NextAction::SyncStatus { .. } => "set-status",
        NextAction::ObserveStatus { .. } => "observe-status",
        NextAction::Brief { .. } => "brief",
        NextAction::BindSlot { .. } => "bind-slot",
        NextAction::Plan { .. } => "plan",
        NextAction::Snapshot { .. } => "snapshot",
        NextAction::Implement { .. }
        | NextAction::Review { .. }
        | NextAction::Repair { .. }
        | NextAction::ResolveGaps { .. } => "run-agent",
        NextAction::Checkpoint { .. } => "checkpoint",
        NextAction::RecordProof { .. } => "record-proof",
        NextAction::Disposition { .. } => "disposition",
        NextAction::Publish { .. } => "publish",
        NextAction::AwaitChecks { .. } | NextAction::PollChecks { .. } => "poll-checks",
        NextAction::Resume { .. } => "resume",
        NextAction::AwaitHumanReview { .. } => "human-review",
        NextAction::FinalVerify { .. } => "final-verify",
        NextAction::Complete { .. } => "complete",
        NextAction::Cleanup { .. } => "cleanup",
        NextAction::OpenDelivery { .. } => "open-delivery",
    }
    .to_owned()
}

pub(super) fn template_for(action: &NextAction) -> Template {
    let mut values = BTreeMap::from([("run".into(), "<absolute-ignored-run-directory>".into())]);
    if let Some(task) = action_task(action) {
        values.insert("task".into(), task.to_string());
    }
    match action {
        NextAction::Implement { .. } => insert(&mut values, "role", "implementer"),
        NextAction::Review { snapshot, .. } => {
            insert(&mut values, "role", "reviewer");
            insert(&mut values, "snapshot.base", snapshot.base.as_ref());
            insert(&mut values, "snapshot.head", snapshot.head.as_ref());
            insert(
                &mut values,
                "snapshot.requirements",
                snapshot.requirements.as_ref(),
            );
        }
        NextAction::Repair { findings, .. } | NextAction::Disposition { findings, .. } => {
            insert(&mut values, "findings", &join(findings));
        }
        NextAction::ResolveGaps { gaps, .. } => insert(&mut values, "gaps", &gaps.join(" | ")),
        NextAction::RecordProof { missing, .. }
        | NextAction::OpenDelivery {
            remaining: missing, ..
        } => {
            insert(&mut values, "criteria", &join(missing));
        }
        NextAction::Publish { step, .. } => {
            insert(&mut values, "step", &format!("{step:?}").to_lowercase())
        }
        NextAction::AwaitChecks { pr, deadline, .. } => {
            insert(&mut values, "pr", &pr.0.to_string());
            insert(&mut values, "deadline", &deadline.to_string());
            insert(&mut values, "wait", "true");
        }
        NextAction::PollChecks { pr, .. } => {
            insert(&mut values, "pr", &pr.0.to_string());
            insert(&mut values, "wait", "true");
        }
        NextAction::FinalVerify { commit, .. } => insert(&mut values, "commit", commit.as_ref()),
        NextAction::Cleanup { slot, .. } => insert(&mut values, "slot", slot.as_ref()),
        NextAction::SyncStatus { issue, target, .. } => {
            insert(&mut values, "issue", issue.as_ref());
            insert(&mut values, "target", target.as_ref());
        }
        NextAction::ObserveStatus { issue, .. } => insert(&mut values, "issue", issue.as_ref()),
        NextAction::MonitorLaunch { launch } => {
            insert(&mut values, "launch", &launch.0.to_string())
        }
        NextAction::SettleOperation { operation } => {
            insert(&mut values, "operation", &operation.0.to_string())
        }
        NextAction::Checkpoint { launch, .. } => {
            insert(&mut values, "launch", &launch.0.to_string())
        }
        NextAction::Hold { reason, .. } => insert(&mut values, "reason", &format!("{reason:?}")),
        NextAction::Claim { .. }
        | NextAction::RecoverUnfinished { .. }
        | NextAction::AnswerQuestions { .. }
        | NextAction::Report { .. }
        | NextAction::Brief { .. }
        | NextAction::BindSlot { .. }
        | NextAction::Plan { .. }
        | NextAction::Snapshot { .. }
        | NextAction::Resume { .. }
        | NextAction::AwaitHumanReview { .. }
        | NextAction::Complete { .. } => {}
    }
    Template { values }
}

fn action_task(action: &NextAction) -> Option<&TaskId> {
    match action {
        NextAction::Claim { task }
        | NextAction::SyncStatus { task, .. }
        | NextAction::ObserveStatus { task, .. }
        | NextAction::Brief { task }
        | NextAction::BindSlot { task }
        | NextAction::Plan { task }
        | NextAction::Snapshot { task }
        | NextAction::Implement { task, .. }
        | NextAction::Checkpoint { task, .. }
        | NextAction::RecordProof { task, .. }
        | NextAction::Review { task, .. }
        | NextAction::Disposition { task, .. }
        | NextAction::Repair { task, .. }
        | NextAction::ResolveGaps { task, .. }
        | NextAction::Publish { task, .. }
        | NextAction::AwaitChecks { task, .. }
        | NextAction::PollChecks { task, .. }
        | NextAction::Resume { task }
        | NextAction::AwaitHumanReview { task, .. }
        | NextAction::FinalVerify { task, .. }
        | NextAction::Complete { task }
        | NextAction::Cleanup { task, .. }
        | NextAction::Hold { task, .. }
        | NextAction::OpenDelivery { task, .. } => Some(task),
        NextAction::RecoverUnfinished { .. }
        | NextAction::AnswerQuestions { .. }
        | NextAction::Report { .. }
        | NextAction::MonitorLaunch { .. }
        | NextAction::SettleOperation { .. } => None,
    }
}

fn insert(values: &mut BTreeMap<String, String>, name: &str, value: &str) {
    values.insert(name.into(), value.into());
}

fn join<T: ToString>(values: &[T]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
