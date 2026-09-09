use super::{PublishStep, schema};
use crate::{
    acceptance::Snapshot,
    ids::{
        CriterionId, FindingId, IssueKey, JiraStatus, LaunchId, OperationId, PrNumber, Sha, SlotId,
        TaskId,
    },
    task::{HoldReason, Question},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionEnvelope {
    pub action: NextAction,
    /// The bare `Command` tag this action performs (`"claim"`,
    /// `"set-status"`, ...). `app` looks this up in the real `Command`
    /// schema to fill in the filled command line and the schema itself
    /// on the way out (`app::schema::AnnotatedAction`); domain only ever
    /// needs to say which one it is.
    pub name: String,
    pub template: schema::Template,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum NextAction {
    Claim {
        task: TaskId,
    },
    RecoverUnfinished {
        tasks: Vec<TaskId>,
    },
    AnswerQuestions {
        questions: Vec<Question>,
    },
    Report {
        remaining: Vec<TaskId>,
    },
    MonitorLaunch {
        launch: LaunchId,
    },
    SettleOperation {
        operation: OperationId,
    },
    SyncStatus {
        task: TaskId,
        issue: IssueKey,
        target: JiraStatus,
    },
    ObserveStatus {
        task: TaskId,
        issue: IssueKey,
    },
    Brief {
        task: TaskId,
    },
    BindSlot {
        task: TaskId,
    },
    Plan {
        task: TaskId,
    },
    Snapshot {
        task: TaskId,
    },
    Implement {
        task: TaskId,
        remaining_turns: u32,
    },
    Checkpoint {
        task: TaskId,
        launch: LaunchId,
    },
    RecordProof {
        task: TaskId,
        missing: Vec<CriterionId>,
    },
    Review {
        task: TaskId,
        snapshot: Snapshot,
    },
    Disposition {
        task: TaskId,
        findings: Vec<FindingId>,
    },
    Repair {
        task: TaskId,
        findings: Vec<FindingId>,
    },
    ResolveGaps {
        task: TaskId,
        gaps: Vec<String>,
    },
    Publish {
        task: TaskId,
        step: PublishStep,
    },
    AwaitChecks {
        task: TaskId,
        pr: PrNumber,
        deadline: crate::Instant,
    },
    PollChecks {
        task: TaskId,
        pr: PrNumber,
    },
    Resume {
        task: TaskId,
    },
    AwaitHumanReview {
        task: TaskId,
        snapshot: Snapshot,
    },
    FinalVerify {
        task: TaskId,
        commit: Sha,
    },
    Complete {
        task: TaskId,
    },
    Cleanup {
        task: TaskId,
        slot: SlotId,
    },
    Hold {
        task: TaskId,
        reason: HoldReason,
    },
    OpenDelivery {
        task: TaskId,
        remaining: Vec<CriterionId>,
    },
}
