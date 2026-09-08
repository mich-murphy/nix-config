use crate::{
    command::{
        NextAction,
        schema::{Schema, Template},
    },
    ids::TaskId,
};
use std::collections::BTreeMap;

pub(super) fn command_for(action: &NextAction) -> (String, Schema) {
    let (command, fields) = match action {
        NextAction::Claim { task } => (scalar("claim", task), vec![]),
        NextAction::RecoverUnfinished { .. }
        | NextAction::Report { .. }
        | NextAction::AnswerQuestions { .. } => (plain("status"), vec![]),
        NextAction::MonitorLaunch { launch } => (
            format!(
                "controller recover-operation {} --kind launch --run <run-directory>",
                launch.0
            ),
            vec![("terminate", "boolean after process observation")],
        ),
        NextAction::SettleOperation { operation } => (
            format!(
                "controller recover-operation {} --kind operation --run <run-directory>",
                operation.0
            ),
            vec![("terminate", "boolean after process observation")],
        ),
        NextAction::SyncStatus { task, .. } => (
            input("set-status", task),
            vec![
                ("issue", "freshly read issue key"),
                ("current", "fresh Jira status ID"),
                ("target", "status ID shown in the action"),
                ("transitions", "fresh array of {id,to}"),
            ],
        ),
        NextAction::ObserveStatus { task, .. } => (
            input("observe-status", task),
            vec![
                ("issue", "issue key shown in the action"),
                ("status", "fresh Jira status ID"),
                ("evidence", "connector read receipt"),
            ],
        ),
        NextAction::Brief { task } => (
            input("brief", task),
            vec![("criteria", "complete criterion array")],
        ),
        NextAction::BindSlot { task } => (
            scalar("bind-slot", task),
            vec![("slot", "worker slot"), ("branch", "fresh task branch")],
        ),
        NextAction::Plan { task } => (
            input("plan", task),
            vec![("plan", "baseline, deliverable, components and examples")],
        ),
        NextAction::Snapshot { task } => (scalar("snapshot", task), vec![]),
        NextAction::Implement { task, .. } => agent(task, "implementer"),
        NextAction::Review { task, .. } => (
            input("run-agent", task),
            vec![
                ("role", "reviewer"),
                ("prompt", "absolute reviewer prompt path"),
                ("snapshot", "exact snapshot shown in the action"),
            ],
        ),
        NextAction::Repair { task, .. } => (
            input("run-agent", task),
            vec![
                ("role", "implementer"),
                ("prompt", "absolute repair prompt path"),
                ("findings", "finding IDs shown in the action"),
            ],
        ),
        NextAction::ResolveGaps { task, .. } => (
            input("run-agent", task),
            vec![
                ("role", "reviewer"),
                ("prompt", "absolute evidence-gap prompt path"),
                ("gaps", "evidence gaps shown in the action"),
            ],
        ),
        NextAction::Checkpoint { task, .. } => (
            input("checkpoint", task),
            vec![
                ("launch", "launch ID shown in the action"),
                ("advanced", "boolean"),
                ("observation", "new measured observation"),
                ("next", "next bounded action"),
                ("outside_paths", "array"),
                ("scope_reason", "required for changed code"),
            ],
        ),
        NextAction::RecordProof { task, .. } => (
            input("record-proof", task),
            vec![
                ("delivery", "current delivery ID"),
                ("entries", "proof entries for each missing criterion"),
            ],
        ),
        NextAction::Disposition { task, .. } => (
            input("disposition", task),
            vec![
                ("finding", "one finding ID shown in the action"),
                ("disposition", "decision and concrete evidence"),
            ],
        ),
        NextAction::Publish { task, .. } => (
            input("publish", task),
            vec![("step", "create, ready, or merge as shown in the action")],
        ),
        NextAction::AwaitChecks { task, .. } => (
            scalar("poll-checks", task),
            vec![("wait", "true to wait within the recorded deadline")],
        ),
        NextAction::PollChecks { task, .. } => (
            scalar("poll-checks", task),
            vec![(
                "wait",
                "true to begin the bounded wait and record a deadline",
            )],
        ),
        NextAction::Resume { task } => (
            scalar("resume", task),
            vec![(
                "final_revisit",
                "boolean, true only for the terminal revisit",
            )],
        ),
        NextAction::AwaitHumanReview { task, .. } => (
            input("human-review", task),
            vec![
                ("snapshot", "exact snapshot shown in the action"),
                ("receipt", "digest of actual human decision"),
            ],
        ),
        NextAction::FinalVerify { task, .. } => (
            scalar("final-verify", task),
            vec![
                ("commit", "exact commit shown in the action"),
                ("evidence", "absolute final verification artifact"),
            ],
        ),
        NextAction::Complete { task } => (scalar("complete", task), vec![]),
        NextAction::Cleanup { task, .. } => (
            scalar("cleanup", task),
            vec![("delete", "boolean, false resets for reuse")],
        ),
        // The task is already held (budget exhausted, PR superseded, or Jira
        // sync failed twice); `hold` would be rejected as a repeat of an
        // already-recorded hold. Report the standing hold instead.
        NextAction::Hold { .. } => (plain("status"), vec![]),
        NextAction::OpenDelivery { task, .. } => (
            input("open-delivery", task),
            vec![
                ("kind", "code or verification"),
                ("authority", "current user receipt and delivery grant"),
                ("jira", "fresh membership and ownership read"),
                ("remaining", "criteria shown in the action"),
            ],
        ),
    };
    (command, schema(fields))
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
        NextAction::Resume { .. } => insert(&mut values, "final_revisit", "false"),
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

fn agent(task: &TaskId, role: &'static str) -> (String, Vec<(&'static str, &'static str)>) {
    (
        input("run-agent", task),
        vec![("role", role), ("prompt", "absolute prompt path")],
    )
}

fn plain(command: &str) -> String {
    format!("controller {command} --run <run-directory>")
}

fn scalar(command: &str, task: &TaskId) -> String {
    format!("controller {command} {task} --run <run-directory>")
}

fn input(command: &str, task: &TaskId) -> String {
    format!("controller {command} --run <run-directory> --input <request.json> # task={task}")
}

fn schema(fields: Vec<(&str, &str)>) -> Schema {
    let mut required = vec!["run".to_owned()];
    let mut properties = BTreeMap::from([(
        "run".to_owned(),
        "absolute ignored run directory".to_owned(),
    )]);
    for (name, description) in fields {
        required.push(name.to_owned());
        properties.insert(name.to_owned(), description.to_owned());
    }
    Schema {
        kind: "object".into(),
        additional_properties: false,
        required,
        properties,
    }
}
