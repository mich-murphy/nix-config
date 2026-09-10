use crate::{
    acceptance::{ProofEntry, Snapshot},
    authority::AuthorityReceipt,
    delivery::DeliveryKind,
    ids::{
        AuthorityId, CriterionId, DeliveryId, Digest, FindingId, IssueKey, JiraStatus, LaunchId,
        OperationId, PrNumber, Sha, SlotId, TaskId, TransitionId,
    },
    ports::MergeMethod,
    review::{Disposition, ReviewReport},
    risk::{Profile, Tier},
    task::{Criterion, HoldReason, Plan, TaskSpec},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "command", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Command {
    Init {
        config: Box<RunConfig>,
        profile: Box<Profile>,
    },
    Status,
    Next,
    UsageReport,
    Discover {
        tasks: Vec<DiscoveredTask>,
        planning_order: Option<Vec<TaskId>>,
    },
    Refresh {
        changed: Vec<DiscoveredTask>,
        removed: Vec<TaskId>,
    },
    Claim {
        task: TaskId,
    },
    Hold {
        task: TaskId,
        reason: HoldReason,
    },
    Resume {
        task: TaskId,
    },
    Brief {
        task: TaskId,
        criteria: Vec<Criterion>,
    },
    Plan {
        task: TaskId,
        plan: Plan,
    },
    Checkpoint {
        task: TaskId,
        advanced: bool,
        observation: String,
        next: String,
        outside_paths: Vec<String>,
        scope_reason: Option<String>,
    },
    Snapshot {
        task: TaskId,
    },
    SubtaskRecord {
        task: TaskId,
        issue: IssueKey,
        criteria: Vec<CriterionId>,
        owned: bool,
        was_terminal: bool,
        status: JiraStatus,
    },
    EscalateTier {
        task: TaskId,
        to: Tier,
        reason: String,
    },
    BindSlot {
        task: TaskId,
        slot: SlotId,
        branch: String,
        authority: Option<AuthorityId>,
    },
    Cleanup {
        task: TaskId,
        delete: bool,
    },
    RecordProof {
        task: TaskId,
        delivery: DeliveryId,
        entries: Vec<ProofEntry>,
    },
    RunCheck {
        task: TaskId,
        criteria: Vec<CriterionId>,
        argv: Vec<String>,
        cwd: PathBuf,
        timeout_seconds: u64,
        implementation: Vec<String>,
    },
    RunAgent {
        task: TaskId,
        role: AgentRole,
        prompt: PathBuf,
        fallback: Option<Fallback>,
    },
    ReviewSchema,
    ValidateReview {
        task: TaskId,
        report: ReviewReport,
    },
    Disposition {
        task: TaskId,
        finding: FindingId,
        disposition: Disposition,
    },
    HumanReview {
        task: TaskId,
        snapshot: Snapshot,
        receipt: Digest,
    },
    LessonRecord {
        task: TaskId,
        lesson: Lesson,
    },
    Publish {
        task: TaskId,
        step: PublishStep,
        title: Option<String>,
        body: Option<PathBuf>,
        method: Option<MergeMethod>,
    },
    ObservePr {
        task: TaskId,
        pr: PrNumber,
    },
    PollChecks {
        task: TaskId,
        wait: bool,
    },
    FinalVerify {
        task: TaskId,
        commit: Sha,
        evidence: PathBuf,
    },
    Complete {
        task: TaskId,
    },
    Judge {
        task: TaskId,
    },
    SetStatus {
        task: TaskId,
        issue: IssueKey,
        current: JiraStatus,
        target: JiraStatus,
        transitions: Vec<Transition>,
    },
    ObserveStatus {
        task: TaskId,
        issue: IssueKey,
        status: JiraStatus,
        evidence: String,
    },
    Grant {
        task: TaskId,
        receipt: AuthorityReceipt,
    },
    OpenDelivery {
        task: TaskId,
        kind: DeliveryKind,
        /// `None` opens a task's first delivery as verification of a
        /// commit already on main, which needs no authority because it
        /// writes no code; every other delivery carries a receipt.
        receipt: Option<AuthorityReceipt>,
        jira: JiraRead,
    },
    NarrowAcceptance {
        task: TaskId,
        criteria: Vec<CriterionId>,
        receipt: AuthorityReceipt,
    },
    RecoverOperation {
        target: RecoveryTarget,
        terminate: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    pub repo: PathBuf,
    pub github_repo: String,
    pub epic: TaskId,
    pub review_mode: ReviewMode,
    pub max_open_prs: u32,
    pub feedback_file: Option<PathBuf>,
    pub jira: JiraConfig,
    pub risk: RiskConfig,
    #[serde(default)]
    pub caps: BTreeMap<Tier, crate::budget::Limits>,
    /// Code deliveries per task that may reopen after a needs-human hold
    /// without a receipt: the run config is the user's standing
    /// authorization for that many follow-ups. Verification deliveries
    /// never count, since they write no code.
    #[serde(default)]
    pub follow_up_deliveries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JiraConfig {
    pub statuses: StatusMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusMap {
    pub todo: JiraStatus,
    pub progress: JiraStatus,
    pub review: JiraStatus,
    pub done: JiraStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RiskConfig {
    pub sensitive: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    Autonomous,
    HumanReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveredTask {
    pub id: TaskId,
    pub spec: TaskSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub id: TransitionId,
    pub to: JiraStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JiraRead {
    pub member: bool,
    pub resolved: bool,
    pub ownership_clear: bool,
    pub requirements: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fallback {
    pub model: crate::ids::ModelId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub text: String,
    pub families: Vec<String>,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRole {
    Implementer,
    Reviewer,
    Escalation,
    /// The post-hoc grader: launched by `judge`, never by `run-agent`, and
    /// exempt from budgets, pairs and the active-launch rule.
    Judge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PublishStep {
    Create,
    Ready,
    Merge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RecoveryTarget {
    Launch { launch: LaunchId },
    Operation { operation: OperationId },
}

mod action;
pub mod schema;

pub use action::{ActionEnvelope, NextAction};
