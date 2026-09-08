use domain::{command::ActionEnvelope, task::Phase};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AgentError {
    pub failed: String,
    pub phase: Option<Box<Phase>>,
    pub why: Rejection,
    pub next: Option<Box<ActionEnvelope>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "class", content = "detail", rename_all = "kebab-case")]
pub enum Rejection {
    Invalid(String),
    Conflict(String),
    Budget(String),
    Authority(String),
    Evidence(String),
    External(String),
    Internal(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {:?}", self.failed, self.why)
    }
}

impl std::error::Error for AgentError {}

impl AgentError {
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
