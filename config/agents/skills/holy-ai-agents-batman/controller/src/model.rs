use crate::{
    authority::Authority,
    ids::{Digest, Sha, SlotId, TaskId},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Delivery {
    Queued,
    Active,
    Deferred,
    Merged,
    AlreadySatisfied,
    NeedsHuman,
    Excluded,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Sync {
    Pending,
    Unknown,
    Confirmed,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Routine,
    Complex,
    HighRisk,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Implementer,
    Reviewer,
    Escalation,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    ChangesRequired,
    Blocked,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStatus {
    Passed,
    Failed,
    Skipped,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    Command,
    Inspection,
    Human,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OpStatus {
    Prepared,
    Unknown,
    Confirmed,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GhAction {
    CreatePr,
    Ready,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusMap {
    pub todo: String,
    pub progress: String,
    pub review: String,
    pub done: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub key: String,
    pub code: bool,
    pub verified: bool,
    pub evidence: String,
    pub main_commit: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub key: String,
    pub parent: String,
    #[serde(default)]
    pub subtasks: Vec<String>,
    pub priority: u32,
    pub due: Option<u64>,
    pub rank: String,
    pub not_before: Option<u64>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    pub member: bool,
    pub ownership_clear: bool,
    pub ownership_evidence: String,
    pub jira_status: String,
    pub resolved: bool,
    #[serde(default)]
    pub blocker: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: String,
    pub text: String,
    pub human_only: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub base: Sha,
    pub head: Sha,
    pub requirements: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceInput {
    pub criterion: String,
    pub status: EvidenceStatus,
    pub kind: EvidenceKind,
    pub artifact: PathBuf,
    pub implementation: Vec<String>,
    pub description: String,
    pub human_source: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub input: EvidenceInput,
    pub digest: Digest,
    pub snapshot: Snapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub impact: String,
    pub category: String,
    pub location: String,
    pub trigger: String,
    pub consequence: String,
    pub correction: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewOutput {
    pub snapshot: Snapshot,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub evidence_gaps: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub launch: u64,
    pub output: ReviewOutput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskState {
    pub status: String,
    pub resolved: bool,
    pub was_terminal: bool,
    pub owned: bool,
    pub criteria: Vec<String>,
    pub source: String,
    pub sync: Sync,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_source: Option<crate::usage_capture::Source>,
    #[serde(default)]
    pub context: serde_json::Value,
    pub id: u64,
    pub key: String,
    pub role: Role,
    pub model: String,
    pub effort: String,
    pub snapshot: Option<Snapshot>,
    pub session: Option<String>,
    pub started: u64,
    pub ended: Option<u64>,
    pub outcome: Option<String>,
    pub reason: String,
    pub usage: Option<serde_json::Value>,
    #[serde(default)]
    pub process: Option<ProcessIdentity>,
    #[serde(default)]
    pub managed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start: String,
    pub group: u32,
    pub terminal: bool,
    #[serde(default)]
    pub marker: String,
    #[serde(default)]
    pub exit_success: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: u64,
    pub key: String,
    pub kind: String,
    pub status: OpStatus,
    pub created: u64,
    pub snapshot: Option<Snapshot>,
    pub attempts: u32,
    pub details: serde_json::Value,
    pub observation: Option<serde_json::Value>,
    #[serde(default)]
    pub process: Option<ProcessIdentity>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub authorities: Vec<Authority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_slot_reuse: Option<crate::followup::HistoricalSlotReuseAuthorization>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exercise: Option<crate::superseded::Exercise>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<crate::stages::Stage>,
    #[serde(default)]
    pub delivery_history: Vec<crate::followup::ArchivedDelivery>,
    #[serde(default)]
    pub followup: Option<crate::followup::Authorization>,
    #[serde(default)]
    pub loop_state: crate::loops::TaskLoop,
    pub spec: TaskSpec,
    pub delivery: Delivery,
    pub sync: Sync,
    pub started_by_run: bool,
    pub was_terminal: bool,
    pub criteria: Vec<Criterion>,
    pub requirements: Option<String>,
    pub tier: Option<Tier>,
    pub tier_reason: String,
    pub slot: Option<String>,
    pub branch: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub evidence: BTreeMap<String, Evidence>,
    pub review: Option<Review>,
    pub findings: BTreeMap<String, Finding>,
    pub dispositions: BTreeMap<String, String>,
    pub reviews: u32,
    pub repairs: u32,
    pub ci_repairs: u32,
    pub escalations: u32,
    pub implementer_session: Option<String>,
    pub reviewer_session: Option<String>,
    pub implementer_model: Option<String>,
    pub pr: Option<u64>,
    pub pr_open: bool,
    pub draft: bool,
    pub merged_commit: Option<Sha>,
    pub main_verified: bool,
    pub final_verified: bool,
    pub deadlines: BTreeMap<String, u64>,
    pub checks: Vec<serde_json::Value>,
    pub checks_head: Option<String>,
    pub next_action: String,
    pub revisit_used: bool,
    #[serde(default)]
    pub subtasks: BTreeMap<String, SubtaskState>,
    #[serde(default)]
    pub last_repair_review: Option<u32>,
}
impl Task {
    pub fn new(spec: TaskSpec, done: &str) -> Self {
        let was_terminal = spec.resolved || spec.jira_status == done;
        Self {
            spec,
            authorities: Vec::new(),
            historical_slot_reuse: None,
            exercise: None,
            stages: vec![],
            delivery_history: Vec::new(),
            followup: None,
            loop_state: Default::default(),
            delivery: Delivery::Queued,
            sync: Sync::Pending,
            started_by_run: false,
            was_terminal,
            criteria: vec![],
            requirements: None,
            tier: None,
            tier_reason: String::new(),
            slot: None,
            branch: None,
            snapshot: None,
            evidence: BTreeMap::new(),
            review: None,
            findings: BTreeMap::new(),
            dispositions: BTreeMap::new(),
            reviews: 0,
            repairs: 0,
            ci_repairs: 0,
            escalations: 0,
            implementer_session: None,
            reviewer_session: None,
            implementer_model: None,
            pr: None,
            pr_open: false,
            draft: true,
            merged_commit: None,
            main_verified: false,
            final_verified: false,
            deadlines: BTreeMap::new(),
            checks: vec![],
            checks_head: None,
            next_action: String::new(),
            revisit_used: false,
            subtasks: BTreeMap::new(),
            last_repair_review: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub usage_imports: Vec<crate::telemetry::UsageReceipt>,
    #[serde(default)]
    pub lessons: BTreeMap<String, crate::loops::LearnedLesson>,
    #[serde(default)]
    pub loop_policy: Option<crate::loops::LoopPolicy>,
    pub version: u32,
    pub run_id: String,
    pub repo: PathBuf,
    pub github_repo: String,
    pub epic: String,
    pub statuses: StatusMap,
    pub coordinator_model: String,
    pub coordinator_effort: String,
    pub preflight: String,
    pub tasks: BTreeMap<String, Task>,
    pub frozen: bool,
    pub order: Vec<String>,
    pub refreshed: u64,
    pub refresh_generation: u64,
    pub selection_generation: u64,
    pub active: Option<String>,
    pub launches: Vec<Launch>,
    pub operations: Vec<Operation>,
    pub slots: BTreeMap<String, String>,
    pub serial: u64,
    pub followups: Vec<String>,
}
impl State {
    pub fn id(&mut self) -> u64 {
        self.serial += 1;
        self.serial
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Input {
    Init {
        repo: PathBuf,
        github_repo: String,
        epic: String,
        statuses: StatusMap,
        coordinator_model: String,
        coordinator_effort: String,
        preflight: String,
        #[serde(default)]
        loop_policy: crate::loops::LoopPolicy,
    },
    AuthorizeBudgetAdjustment {
        key: TaskId,
        mode: crate::budget::AdjustmentMode,
        source: String,
        artifact: PathBuf,
        scope: String,
        #[serde(default)]
        paths: Vec<String>,
    },
    EvidenceBatch {
        key: TaskId,
        snapshot: Snapshot,
        results: Vec<crate::batch::CriterionResult>,
    },
    UsageImport {
        request: crate::telemetry::ImportRequest,
    },
    UsageReport,
    UsageBind {
        launch: u64,
        source: crate::usage_capture::Source,
    },
    UsageSync,
    ReviewReadiness {
        key: TaskId,
    },
    ReconcilePlan {
        key: Option<TaskId>,
    },
    BeginStage {
        request: crate::stages::Request,
    },
    EndStage {
        key: TaskId,
    },
    LessonRecord {
        key: TaskId,
        lesson: crate::loops::Lesson,
    },
    ConfigureLoop {
        policy: crate::loops::LoopPolicy,
    },
    LoopPlan {
        key: TaskId,
        plan: crate::loops::PlanInput,
    },
    Measure {
        key: TaskId,
        criterion: String,
        observed: String,
        satisfied: bool,
        artifact: PathBuf,
    },
    Checkpoint {
        key: TaskId,
        advanced: bool,
        finding: String,
        next_step: String,
        artifact: PathBuf,
        #[serde(default)]
        scope_reason: String,
    },
    HumanReview {
        key: TaskId,
        snapshot: Snapshot,
        source: String,
        artifact: PathBuf,
    },
    Discover {
        tasks: Vec<TaskSpec>,
        #[serde(default)]
        planning_order: Vec<String>,
    },
    Refresh {
        tasks: Vec<TaskSpec>,
    },
    Claim {
        key: TaskId,
    },
    Brief {
        key: TaskId,
        criteria: Vec<Criterion>,
        tier: Tier,
        reason: String,
    },
    ReserveSlot {
        slot: SlotId,
    },
    Bind {
        key: TaskId,
        slot: SlotId,
        #[serde(default)]
        historical_slot_reuse: Option<crate::followup::HistoricalSlotReuseRequest>,
    },
    Snapshot {
        key: TaskId,
    },
    Evidence {
        key: TaskId,
        evidence: EvidenceInput,
    },
    SubtaskRecord {
        key: TaskId,
        issue: String,
        current: String,
        resolved: bool,
        criteria: Vec<String>,
        ownership_evidence: String,
    },
    AgentBegin {
        #[serde(default)]
        allow_evidence_gaps: bool,
        key: TaskId,
        role: Role,
        model: String,
        effort: String,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        ci_repair: bool,
    },
    AgentFinish {
        launch: u64,
        session: String,
        outcome: String,
        review: Option<ReviewOutput>,
        usage: Option<serde_json::Value>,
    },
    Disposition {
        key: TaskId,
        finding: String,
        decision: String,
        evidence: String,
    },
    Hold {
        key: TaskId,
        outcome: Delivery,
        reason: String,
    },
    Resume {
        key: TaskId,
        #[serde(default)]
        final_revisit: bool,
    },
    JiraPrepare {
        key: TaskId,
        issue: String,
        current: String,
        resolved: bool,
        target: String,
        transitions: Vec<Transition>,
    },
    Dispatch {
        operation: u64,
    },
    JiraObserve {
        operation: u64,
        current: String,
        resolved: bool,
        evidence: String,
    },
    JiraRetry {
        operation: u64,
        current: String,
        resolved: bool,
        transitions: Vec<Transition>,
    },
    SyncFailed {
        operation: u64,
        reason: String,
    },
    GhPrepare {
        key: TaskId,
        action: GhAction,
        title: Option<String>,
        body: Option<PathBuf>,
        method: Option<String>,
    },
    FinalVerify {
        key: TaskId,
        main_commit: Sha,
        evidence: PathBuf,
    },
    Complete {
        key: TaskId,
    },
    CleanupCheck {
        key: TaskId,
    },
    Status,
    Next,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub id: String,
    pub to: String,
}
