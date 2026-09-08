use crate::{
    acceptance::{ProofEntry, Snapshot},
    authority::Authority,
    delivery::DeliveryKind,
    ids::{
        AuthorityId, CriterionId, DeliveryId, Digest, FindingId, IssueKey, JiraStatus, LaunchId,
        OperationId, PrNumber, Sha, SlotId, TaskId, TransitionId,
    },
    ports::MergeMethod,
    review::{Disposition, Review},
    risk::{Profile, Tier},
    task::{Criterion, HoldReason, Plan, Question, TaskSpec},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        final_revisit: bool,
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
        review: Review,
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
        authority: Box<Authority>,
    },
    OpenDelivery {
        task: TaskId,
        kind: DeliveryKind,
        authority: AuthorityId,
        jira: JiraRead,
    },
    NarrowAcceptance {
        task: TaskId,
        criteria: Vec<CriterionId>,
        authority: AuthorityId,
    },
    RecoverOperation {
        operation: OperationId,
        terminate: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JiraConfig {
    pub statuses: StatusMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMap {
    pub todo: JiraStatus,
    pub progress: JiraStatus,
    pub review: JiraStatus,
    pub done: JiraStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskConfig {
    pub sensitive: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    Autonomous,
    HumanReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTask {
    pub id: TaskId,
    pub spec: TaskSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: TransitionId,
    pub to: JiraStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraRead {
    pub member: bool,
    pub resolved: bool,
    pub ownership_clear: bool,
    pub requirements: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fallback {
    pub model: crate::ids::ModelId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    pub text: String,
    pub families: Vec<String>,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRole {
    Implementer,
    Reviewer,
    Escalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublishStep {
    Create,
    Ready,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub action: NextAction,
    pub command: String,
    pub schema: serde_placeholder::Schema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub mod serde_placeholder {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Schema {
        pub required: Vec<String>,
        pub properties: BTreeMap<String, String>,
    }
}
