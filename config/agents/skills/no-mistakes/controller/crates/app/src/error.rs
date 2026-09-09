mod conflict;
mod evidence;

pub use conflict::ConflictReason;
pub use evidence::EvidenceError;

use crate::schema::AnnotatedAction;
use domain::{authority::AuthorityError, budget::BudgetKind, ids::TaskId, task::Phase};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AgentError {
    pub failed: String,
    pub phase: Option<Box<Phase>>,
    #[serde(flatten)]
    pub why: Rejection,
    /// A human-readable rendering of `why`, computed once at construction
    /// (`Rejection`'s own `Display`) so the JSON on stderr carries a
    /// message a person can read without decoding `class` and `reason`.
    pub message: String,
    pub next: Option<Box<AnnotatedAction>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "class", content = "reason", rename_all = "kebab-case")]
pub enum Rejection {
    Invalid(String),
    Conflict(ConflictReason),
    Budget(BudgetKind),
    Authority(AuthorityError),
    Evidence(EvidenceError),
    External(String),
    Internal(String),
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid request: {message}"),
            Self::Conflict(reason) => write!(formatter, "{reason}"),
            Self::Budget(kind) => write!(formatter, "{kind:?} budget exhausted"),
            Self::Authority(error) => write!(formatter, "authority rejected: {error:?}"),
            Self::Evidence(error) => write!(formatter, "{error}"),
            Self::External(message) => write!(formatter, "external failure: {message}"),
            Self::Internal(message) => write!(formatter, "internal error: {message}"),
        }
    }
}

impl From<ConflictReason> for Rejection {
    fn from(reason: ConflictReason) -> Self {
        Self::Conflict(reason)
    }
}

impl From<BudgetKind> for Rejection {
    fn from(kind: BudgetKind) -> Self {
        Self::Budget(kind)
    }
}

impl From<AuthorityError> for Rejection {
    fn from(error: AuthorityError) -> Self {
        Self::Authority(error)
    }
}

impl From<EvidenceError> for Rejection {
    fn from(error: EvidenceError) -> Self {
        Self::Evidence(error)
    }
}

impl From<domain::acceptance::AcceptanceError> for Rejection {
    fn from(error: domain::acceptance::AcceptanceError) -> Self {
        Self::Evidence(EvidenceError::Acceptance(error))
    }
}

impl From<domain::review::ReviewError> for Rejection {
    fn from(error: domain::review::ReviewError) -> Self {
        Self::Evidence(EvidenceError::Review(error))
    }
}

impl From<domain::delivery::DeliveryError> for Rejection {
    fn from(error: domain::delivery::DeliveryError) -> Self {
        Self::Conflict(ConflictReason::History(error))
    }
}

impl From<domain::sync::SyncError> for Rejection {
    fn from(error: domain::sync::SyncError) -> Self {
        Self::Conflict(ConflictReason::Status(error))
    }
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.failed, self.message)
    }
}

impl std::error::Error for AgentError {}

impl AgentError {
    #[must_use]
    pub fn new(
        failed: impl Into<String>,
        phase: Option<Box<Phase>>,
        why: impl Into<Rejection>,
        next: Option<Box<AnnotatedAction>>,
    ) -> Self {
        let why = why.into();
        Self {
            failed: failed.into(),
            phase,
            message: why.to_string(),
            why,
            next,
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self.why {
            Rejection::Invalid(_) => 2,
            Rejection::Conflict(_) => 3,
            Rejection::Budget(_) => 4,
            Rejection::Authority(_) => 5,
            Rejection::Evidence(_) => 6,
            Rejection::External(_) => 7,
            Rejection::Internal(_) => 8,
        }
    }
}

/// A command's fixed context for the rest of its handler: which command
/// failed, the affected task's actual phase, and the next valid action,
/// all read once so a handler with several fallible steps does not repeat
/// `self.error("grant", Some(&task), ...)` at every one. `reject` accepts
/// anything with a `From<_> for Rejection`, so most call sites become
/// `.map_err(|error| ctx.reject(error))?` or, through the same
/// conversion, a bare `?` where the surrounding function returns
/// `Result<_, Rejection>`.
pub(crate) struct Ctx {
    failed: String,
    phase: Option<Box<Phase>>,
    next: Option<Box<AnnotatedAction>>,
}

impl Ctx {
    pub(crate) fn reject(&self, why: impl Into<Rejection>) -> AgentError {
        AgentError::new(
            self.failed.clone(),
            self.phase.clone(),
            why,
            self.next.clone(),
        )
    }
}

impl crate::App<'_> {
    pub(crate) fn ctx(&self, command: &str, task: Option<&TaskId>) -> Ctx {
        let state = self.store.state().ok();
        let phase = task
            .and_then(|id| state.as_ref()?.tasks.get(id))
            .map(|value| Box::new(value.phase.clone()));
        let now = self.services.clock.now();
        let next = state
            .and_then(|value| value.next(now))
            .map(|envelope| Box::new(crate::schema::annotate(envelope)));
        Ctx {
            failed: command.into(),
            phase,
            next,
        }
    }
}
