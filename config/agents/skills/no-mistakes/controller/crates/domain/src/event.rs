use crate::{
    Instant,
    acceptance::{ProofEntry, Snapshot},
    authority::{Authority, Grant},
    budget::BudgetKind,
    command::{AgentRole, DiscoveredTask, Lesson, RunConfig},
    delivery::{Delivery, Outcome, PullRequest},
    ids::{
        AuthorityId, CriterionId, DeliveryId, FindingId, IssueKey, JiraStatus, LaunchId,
        OperationId, Sha, SlotId, TaskId, TransitionId, UseId,
    },
    ports::{ProcessIdentity, Tokens},
    review::Disposition,
    risk::{Profile, Tier},
    sync::Sync,
    task::{HoldReason, Plan, Receipt, SlotBinding, Subtask},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventRecord {
    pub sequence: u64,
    pub at: Instant,
    pub actor: Actor,
    pub event: Event,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Actor {
    Coordinator,
    Harness,
    Adapter,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
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
        feedback: Vec<Lesson>,
    },
    Snapshotted {
        task: TaskId,
        delivery: DeliveryId,
        snapshot: Snapshot,
        lines: u32,
        files: u32,
        covers: Option<LaunchId>,
    },
    Checkpointed {
        task: TaskId,
        /// `None` for an integration checkpoint (a snapshot with no
        /// implementer launch of its own): it marks nothing, since there
        /// is no launch to flag as checkpointed.
        launch: Option<LaunchId>,
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
        receipt: Receipt,
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
    /// An unused `Pair` grant repurposed as the next delivery's grant
    /// (design Section 5): the handler decides this, `apply` only records
    /// the resulting `grant` onto the existing `authority`.
    AuthorityRepurposed {
        task: TaskId,
        authority: AuthorityId,
        grant: Grant,
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    pub id: LaunchId,
    pub task: TaskId,
    pub delivery: DeliveryId,
    pub role: AgentRole,
    pub prompt: PathBuf,
    /// `None` while the launch is running; set once by `LaunchEnded`.
    pub outcome: Option<LaunchOutcome>,
    /// Captured from the harness's own stream by `LaunchEnded`. `None`
    /// means unknown, never zero.
    pub usage: Option<Tokens>,
    /// Set by `Checkpointed` naming this launch. Read by `next` to decide
    /// whether the previous implementer turn still needs a checkpoint or,
    /// once checkpointed, a snapshot.
    pub checkpointed: bool,
    pub process: Option<ProcessIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchOutcome {
    Completed { session: String, output: String },
    Failed { reason: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: OperationId,
    pub task: TaskId,
    pub delivery: DeliveryId,
    pub action: GitHubAction,
    pub status: OperationStatus,
    pub process: Option<ProcessIdentity>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
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
        timeout_seconds: u64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OperationStatus {
    Running,
    Unknown,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Observation {
    PullRequest(PullRequest),
    Check { exit: i32, artifact: PathBuf },
    Worktree { head: Sha, clean: bool },
    Failure { reason: String },
}
