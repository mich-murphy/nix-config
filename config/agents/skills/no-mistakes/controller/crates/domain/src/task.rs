use crate::{
    Instant,
    authority::Authority,
    budget::{BudgetKind, Budgets},
    delivery::Delivery,
    ids::{
        AuthorityId, CriterionId, DeliveryId, Digest, IssueKey, JiraStatus, PrNumber, Sha, SlotId,
        TaskId,
    },
    risk::TierState,
    sync::Sync,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: TaskId,
    pub spec: TaskSpec,
    pub phase: Phase,
    pub hold: Option<Hold>,
    pub sync: Sync,
    pub budgets: Budgets,
    pub deliveries: Vec<Delivery>,
    pub authorities: Vec<Authority>,
    pub tier: TierState,
    pub subtasks: BTreeMap<IssueKey, Subtask>,
    /// Set per criterion by the first `Planned` event that carries it and
    /// immutable for that criterion afterwards: baselines belong to the
    /// task, not to any one delivery's plan, so verification and follow-up
    /// deliveries (which have no plan of their own) can still be verified
    /// against them. A later `Planned` event may still add a baseline for a
    /// criterion the task had none for yet (for example one a narrowing
    /// introduced); it may never change one already recorded.
    pub baselines: BTreeMap<CriterionId, Digest>,
    /// Set once by `Judged`. Absent until the judge has run; `default` so
    /// projections written before judging existed still load.
    #[serde(default)]
    pub judgment: Option<crate::judge::Judgment>,
}

impl Task {
    /// The delivery the task's current phase acts on: every phase from
    /// `Planned` onward has exactly one, and it is always the last one
    /// opened. `PlannedWork` lives only here (design decision 2a), so this
    /// is the one place that reaches for it.
    #[must_use]
    pub fn current_delivery(&self) -> Option<&Delivery> {
        self.deliveries.last()
    }

    /// The current delivery's planned work, if it has taken one. `None`
    /// before the first `bind-slot` and for a `Verification` delivery,
    /// which has no worktree of its own.
    #[must_use]
    pub fn current_work(&self) -> Option<&PlannedWork> {
        self.current_delivery()?.work.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub parent: TaskId,
    pub criteria: Vec<Criterion>,
    pub dependencies: Vec<Dependency>,
    pub priority: u32,
    pub due: Option<Instant>,
    pub rank: String,
    pub not_before: Option<Instant>,
    pub member: bool,
    pub ownership_clear: bool,
    pub ownership_evidence: String,
    pub was_terminal: bool,
    pub requirements: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub task: TaskId,
    pub code: bool,
    pub main_commit: Option<Sha>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: CriterionId,
    pub text: String,
    pub human_only: bool,
}

/// A phase carries only what is exclusive to it. `Planned` and `InFlight`
/// carry no payload: the task's current delivery (`Task::current_delivery`)
/// is always the one they act on, and that delivery's own `work`, `proof`
/// and `review` are the only record of what stage of implementation,
/// review or repair it is in. Deriving that sub-state from the delivery in
/// `next`, rather than storing it here, is owner's decision 2a: a stored
/// sub-state and a delivery that disagreed would both be representable,
/// and only one of them could be true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Phase {
    Queued,
    Blocked(BlockReason),
    NeedsInput(Question),
    Claimed,
    Planned,
    InFlight,
    Merged {
        delivery: DeliveryId,
        commit: Sha,
    },
    Verified {
        receipt: Receipt,
    },
    /// Verified and separately confirmed done: Jira Done recorded and
    /// cleanup permitted. Distinct from `Verified` so "every criterion
    /// passed" and "the run has recorded that fact" cannot be conflated.
    Completed {
        receipt: Receipt,
    },
    Excluded(ExclusionReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedWork {
    pub slot: SlotBinding,
    pub branch: String,
    pub base: Sha,
    pub snapshot: Option<crate::acceptance::Snapshot>,
    pub plan: Option<Plan>,
    pub feedback: Vec<crate::command::Lesson>,
    /// The latest implementer launch the current snapshot covers. `None`
    /// means either no implementer launch has happened yet or the snapshot
    /// is integration-only; either is a valid reason for a snapshot to exist
    /// without one.
    pub snapshot_launch: Option<crate::ids::LaunchId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub deliverable: String,
    pub components: Vec<String>,
    pub examples: Vec<String>,
    pub baselines: BTreeMap<CriterionId, Digest>,
    pub lesson_families: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotBinding {
    pub slot: SlotId,
    pub origin: SlotOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SlotOrigin {
    Fresh,
    Reused { authority: AuthorityId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    pub since: Instant,
    pub reason: HoldReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub enum HoldReason {
    CiPending {
        pr: PrNumber,
        head: Sha,
        deadline: Instant,
    },
    NeedsHuman {
        diagnosis: String,
        remaining: Vec<CriterionId>,
    },
    BudgetExhausted(BudgetKind),
    SupersededPr {
        pr: PrNumber,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum BlockReason {
    Dependency(TaskId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub task: Option<TaskId>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ExclusionReason {
    NotMember,
    TerminalBeforeRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Receipt {
    Delivery { commit: Sha },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Subtask {
    pub criteria: Vec<CriterionId>,
    pub owned: bool,
    pub was_terminal: bool,
    pub status: JiraStatus,
    pub sync: Sync,
}
