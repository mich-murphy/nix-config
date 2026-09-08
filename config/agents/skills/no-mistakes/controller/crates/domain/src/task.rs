use crate::{
    Instant,
    authority::Authority,
    budget::{BudgetKind, Budgets},
    delivery::{Delivery, Replacement},
    ids::{
        AuthorityId, CriterionId, DeliveryId, Digest, IssueKey, JiraStatus, PrNumber, Sha, SlotId,
        TaskId,
    },
    risk::TierState,
    sync::Sync,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub jira_status: JiraStatus,
    pub requirements: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub task: TaskId,
    pub code: bool,
    pub verified: bool,
    pub main_commit: Option<Sha>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: CriterionId,
    pub text: String,
    pub human_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Queued,
    Blocked(BlockReason),
    NeedsInput(Question),
    Claimed,
    Planned { work: PlannedWork },
    InFlight { work: PlannedWork, stage: WorkStage },
    Merged { delivery: DeliveryId, commit: Sha },
    Verified { receipt: Receipt },
    Excluded(ExclusionReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedWork {
    pub slot: SlotBinding,
    pub branch: String,
    pub base: Sha,
    pub snapshot: Option<crate::acceptance::Snapshot>,
    pub plan: Option<Plan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub deliverable: String,
    pub components: Vec<String>,
    pub examples: Vec<String>,
    pub baselines: BTreeMap<CriterionId, Digest>,
    pub lesson_families: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBinding {
    pub slot: SlotId,
    pub origin: SlotOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotOrigin {
    Fresh,
    Reused { authority: AuthorityId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkStage {
    Building,
    Validating,
    Reviewing {
        launch: crate::ids::LaunchId,
    },
    Repairing {
        findings: Vec<crate::ids::FindingId>,
    },
    Publishing {
        pr: PrNumber,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    pub since: Instant,
    pub reason: HoldReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        replacement: Option<Replacement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    Dependency(TaskId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub task: Option<TaskId>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExclusionReason {
    NotMember,
    TerminalBeforeRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Receipt {
    Delivery { commit: Sha },
    Existing { main: Sha },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subtask {
    pub criteria: Vec<CriterionId>,
    pub owned: bool,
    pub was_terminal: bool,
    pub status: JiraStatus,
    pub sync: Sync,
}
