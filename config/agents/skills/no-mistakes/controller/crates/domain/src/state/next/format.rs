use crate::{
    command::{NextAction, serde_placeholder::Schema},
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
        NextAction::Hold { task, .. } => (
            input("hold", task),
            vec![("reason", "hold reason shown in the action")],
        ),
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
        required,
        properties,
    }
}
