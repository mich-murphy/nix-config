use crate::{
    Instant,
    acceptance::{ProofEntry, Snapshot},
    authority::Authority,
    budget::BudgetKind,
    command::{AgentRole, DiscoveredTask, Lesson, RunConfig},
    delivery::{Delivery, Outcome, PullRequest},
    ids::{
        AuthorityId, CriterionId, DeliveryId, FindingId, IssueKey, JiraStatus, LaunchId,
        OperationId, Sha, SlotId, TaskId, TransitionId, UseId,
    },
    ports::Tokens,
    review::Disposition,
    risk::{Profile, Tier},
    sync::Sync,
    task::{HoldReason, Phase, Plan, SlotBinding, Subtask},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub sequence: u64,
    pub at: Instant,
    pub actor: Actor,
    pub event: Event,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Actor {
    Coordinator,
    Harness,
    Adapter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    RunInitialized {
        config: Box<RunConfig>,
        profile: Box<Profile>,
        capabilities: crate::ports::Capabilities,
    },
    QueueDiscovered {
        tasks: Vec<DiscoveredTask>,
        order: Vec<TaskId>,
        frozen: Instant,
    },
    QueueRefreshed {
        changed: Vec<DiscoveredTask>,
        removed: Vec<TaskId>,
    },
    QuestionRaised {
        task: Option<TaskId>,
        text: String,
    },
    Claimed {
        task: TaskId,
        at: Instant,
    },
    Held {
        task: TaskId,
        reason: HoldReason,
        at: Instant,
    },
    Resumed {
        task: TaskId,
    },
    Briefed {
        task: TaskId,
        criteria: Vec<crate::task::Criterion>,
        requirements: crate::ids::Digest,
        tier: Tier,
        provisional: bool,
    },
    TierRaised {
        task: TaskId,
        to: Tier,
        reason: String,
        at: Instant,
    },
    SlotBound {
        task: TaskId,
        binding: SlotBinding,
        branch: String,
        base: Sha,
    },
    SlotReleased {
        task: TaskId,
        slot: SlotId,
    },
    Planned {
        task: TaskId,
        plan: Plan,
    },
    Snapshotted {
        task: TaskId,
        delivery: DeliveryId,
        snapshot: Snapshot,
        lines: u32,
        files: u32,
    },
    Checkpointed {
        task: TaskId,
        advanced: bool,
        stalled: u32,
        observation: String,
        next: String,
    },
    SubtaskRecorded {
        task: TaskId,
        issue: IssueKey,
        subtask: Subtask,
    },
    Excluded {
        task: TaskId,
        phase: Phase,
    },
    LaunchStarted {
        launch: Launch,
    },
    LaunchEnded {
        launch: LaunchId,
        result: LaunchOutcome,
        usage: Option<Tokens>,
    },
    ProofRecorded {
        task: TaskId,
        delivery: DeliveryId,
        entries: Vec<ProofEntry>,
    },
    ProofInvalidated {
        task: TaskId,
        delivery: DeliveryId,
        cause: String,
    },
    ReviewSettled {
        task: TaskId,
        delivery: DeliveryId,
        review: Box<crate::review::Review>,
    },
    Dispositioned {
        task: TaskId,
        finding: FindingId,
        disposition: Disposition,
    },
    HumanReviewed {
        task: TaskId,
        snapshot: Snapshot,
        receipt: crate::ids::Digest,
    },
    LessonRecorded {
        task: TaskId,
        lesson: Lesson,
    },
    DeliveryOpened {
        task: TaskId,
        delivery: Box<Delivery>,
    },
    AcceptanceNarrowed {
        task: TaskId,
        delivery: DeliveryId,
        criteria: Vec<CriterionId>,
        authority: AuthorityId,
    },
    OperationStarted {
        operation: Operation,
    },
    OperationSettled {
        operation: OperationId,
        status: OperationStatus,
        observation: Observation,
    },
    PrObserved {
        task: TaskId,
        delivery: DeliveryId,
        pr: PullRequest,
    },
    DeliveryClosed {
        task: TaskId,
        delivery: DeliveryId,
        outcome: Outcome,
    },
    Verified {
        task: TaskId,
        commit: Sha,
    },
    Completed {
        task: TaskId,
    },
    StatusIntended {
        task: TaskId,
        issue: IssueKey,
        current: JiraStatus,
        target: JiraStatus,
        transition: TransitionId,
        operation: OperationId,
        attempts: u8,
        at: Instant,
    },
    StatusObserved {
        task: TaskId,
        issue: IssueKey,
        read: JiraStatus,
        sync: Sync,
    },
    AuthorityRegistered {
        task: TaskId,
        authority: Box<Authority>,
    },
    GrantUsed {
        task: TaskId,
        authority: AuthorityId,
        by: UseId,
    },
    PairSpent {
        task: TaskId,
        authority: AuthorityId,
        launch: LaunchId,
        implementation: bool,
    },
    BudgetSpent {
        task: TaskId,
        budget: BudgetKind,
        counted: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launch {
    pub id: LaunchId,
    pub task: TaskId,
    pub delivery: DeliveryId,
    pub role: AgentRole,
    pub prompt: PathBuf,
    pub session: Option<String>,
    pub counted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchOutcome {
    Completed { session: String, output: String },
    Failed { reason: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub task: TaskId,
    pub delivery: DeliveryId,
    pub action: GitHubAction,
    pub status: OperationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitHubAction {
    Create {
        title: String,
        body: PathBuf,
        head: Sha,
    },
    Ready {
        pr: crate::ids::PrNumber,
        head: Sha,
    },
    Merge {
        pr: crate::ids::PrNumber,
        head: Sha,
        method: crate::ports::MergeMethod,
    },
    Observe {
        pr: crate::ids::PrNumber,
    },
    Check {
        argv: Vec<String>,
        cwd: PathBuf,
    },
    BindSlot {
        slot: SlotId,
        branch: String,
    },
    Cleanup {
        slot: SlotId,
        delete: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationStatus {
    Running,
    Unknown,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Observation {
    PullRequest(PullRequest),
    Check { exit: i32, artifact: PathBuf },
    Worktree { head: Sha, clean: bool },
    Failure { reason: String },
}
