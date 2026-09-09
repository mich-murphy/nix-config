//! The typed payload `Rejection::Conflict` carries: the phase, hold and
//! delivery-history rules that exist today as ad hoc strings.
use domain::{delivery::DeliveryError, sync::SyncError};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConflictReason {
    NoDelivery,
    NoPlannedDelivery,
    NoOwnedSlot,
    NoPr,
    MembershipFrozen,
    QueueNotDiscovered,
    NewTasksOutOfScope,
    NotNextEligibleClaim,
    TaskOwnedElsewhere,
    CiDeadlinePending,
    ActiveOperationUnsettled,
    ActiveLaunchUnsettled,
    NotAvailableHold,
    TaskNotActive,
    ReplacementNotMerged,
    BriefRequiresClaimedTask,
    PlanRequiresSlot,
    BindRequiresClaimedTask,
    AlreadyCheckpointed,
    CheckpointRequired,
    CheckpointRequiresTerminalOrIntegration,
    VerificationRejectsImplementer,
    RequiresPlannedOrVerification,
    EscalationRequiresReview,
    RunAlreadyExists,
    JiraDoneRequiresVerified,
    CompletionRequiresJiraDone,
    CleanupRequiresCompletion,
    ClosedDeliveryImmutable,
    OperationAlreadySettled,
    OperationHasNoProcess,
    ProcessStillRunning,
    LaunchStillRunning,
    LaunchNotRecoverable,
    LaunchHasNoProcess,
    LessonRequiresVerified,
    LessonAlreadyRecorded,
    OpenRequiresNeedsHumanHold,
    TerminalTaskCannotOpen,
    JiraOwnershipUnresolved,
    CriteriaChanged,
    HistoricalMergeLeftMain,
    ReplacementIdentityChanged,
    ClosedDeliveryChanged,
    /// A worktree slot's actual state (missing, dirty, foreign, or a
    /// binding kind that does not match the requested reuse) rejects this
    /// binding. Carries the debug-formatted `SlotState`, which is not
    /// itself a serialisable domain type.
    SlotUnsafe(String),
    History(DeliveryError),
    Status(SyncError),
}

impl fmt::Display for ConflictReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDelivery => formatter.write_str("task has no delivery"),
            Self::NoPlannedDelivery => formatter.write_str("task has no planned delivery"),
            Self::NoOwnedSlot => formatter.write_str("task has no owned slot"),
            Self::NoPr => formatter.write_str("task has no PR"),
            Self::MembershipFrozen => formatter.write_str("membership is already frozen"),
            Self::QueueNotDiscovered => formatter.write_str("discover the queue first"),
            Self::NewTasksOutOfScope => formatter.write_str("new tasks are follow-up scope"),
            Self::NotNextEligibleClaim => {
                formatter.write_str("task is not the next eligible claim")
            }
            Self::TaskOwnedElsewhere => formatter.write_str("another run owns this task"),
            Self::CiDeadlinePending => {
                formatter.write_str("CI hold requires the recorded deadline to expire")
            }
            Self::ActiveOperationUnsettled => {
                formatter.write_str("settle the active operation first")
            }
            Self::ActiveLaunchUnsettled => formatter.write_str("settle the active launch first"),
            Self::NotAvailableHold => formatter.write_str("task is not an available hold"),
            Self::TaskNotActive => formatter.write_str("task is not active"),
            Self::ReplacementNotMerged => formatter.write_str("replacement is not merged on main"),
            Self::BriefRequiresClaimedTask => {
                formatter.write_str("brief requires a claimed task and criteria")
            }
            Self::PlanRequiresSlot => formatter.write_str("bind a slot before planning"),
            Self::BindRequiresClaimedTask => {
                formatter.write_str("bind requires the claimed task and a task branch")
            }
            Self::AlreadyCheckpointed => {
                formatter.write_str("implementation turn is already checkpointed")
            }
            Self::CheckpointRequired => {
                formatter.write_str("checkpoint the previous implementer turn")
            }
            Self::CheckpointRequiresTerminalOrIntegration => {
                formatter.write_str("checkpoint requires a terminal turn or integration snapshot")
            }
            Self::VerificationRejectsImplementer => {
                formatter.write_str("verification delivery rejects implementers")
            }
            Self::RequiresPlannedOrVerification => {
                formatter.write_str("agent requires planned or verification work")
            }
            Self::EscalationRequiresReview => {
                formatter.write_str("escalation requires a completed reviewer launch")
            }
            Self::RunAlreadyExists => formatter.write_str("run already exists"),
            Self::JiraDoneRequiresVerified => {
                formatter.write_str("Jira Done requires verified delivery")
            }
            Self::CompletionRequiresJiraDone => {
                formatter.write_str("completion needs verification and confirmed Jira Done")
            }
            Self::CleanupRequiresCompletion => formatter.write_str("cannot clean unfinished work"),
            Self::ClosedDeliveryImmutable => {
                formatter.write_str("closed delivery operations are immutable")
            }
            Self::OperationAlreadySettled => formatter.write_str("operation is already settled"),
            Self::OperationHasNoProcess => {
                formatter.write_str("operation has no owned process; observe its external state")
            }
            Self::ProcessStillRunning => {
                formatter.write_str("owned process is still running; use --terminate to stop it")
            }
            Self::LaunchStillRunning => formatter.write_str("owned launch is still running"),
            Self::LaunchNotRecoverable => {
                formatter.write_str("recovery requires an unsettled terminated launch")
            }
            Self::LaunchHasNoProcess => {
                formatter.write_str("launch has no persisted process identity")
            }
            Self::LessonRequiresVerified => {
                formatter.write_str("accepted lessons require verified delivery")
            }
            Self::LessonAlreadyRecorded => formatter.write_str("lesson already recorded"),
            Self::OpenRequiresNeedsHumanHold => {
                formatter.write_str("open-delivery requires a needs-human hold")
            }
            Self::TerminalTaskCannotOpen => {
                formatter.write_str("terminal tasks cannot open another delivery")
            }
            Self::JiraOwnershipUnresolved => {
                formatter.write_str("fresh Jira read does not establish unresolved run ownership")
            }
            Self::CriteriaChanged => formatter.write_str("criteria changed"),
            Self::HistoricalMergeLeftMain => formatter.write_str("historical merge left main"),
            Self::ReplacementIdentityChanged => formatter.write_str("replacement identity changed"),
            Self::ClosedDeliveryChanged => {
                formatter.write_str("closed delivery observation changed")
            }
            Self::SlotUnsafe(state) => {
                write!(formatter, "slot is not safe for this binding: {state}")
            }
            Self::History(error) => write!(formatter, "delivery history: {error:?}"),
            Self::Status(error) => write!(formatter, "status synchronisation: {error:?}"),
        }
    }
}
