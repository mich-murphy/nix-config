//! The typed payload `Rejection::Evidence` carries: the two domain error
//! enums proof and review checks already produce, plus the few cases that
//! have no typed domain counterpart.
use domain::{acceptance::AcceptanceError, review::ReviewError};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EvidenceError {
    Acceptance(AcceptanceError),
    Review(ReviewError),
    /// A review or publish step needs the current snapshot, and none is
    /// recorded yet.
    MissingSnapshot,
    Other(String),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acceptance(error) => write!(formatter, "acceptance failed: {error:?}"),
            Self::Review(error) => write!(formatter, "invalid review: {error:?}"),
            Self::MissingSnapshot => formatter.write_str("review requires a snapshot"),
            Self::Other(message) => formatter.write_str(message),
        }
    }
}

impl From<AcceptanceError> for EvidenceError {
    fn from(error: AcceptanceError) -> Self {
        Self::Acceptance(error)
    }
}

impl From<ReviewError> for EvidenceError {
    fn from(error: ReviewError) -> Self {
        Self::Review(error)
    }
}

impl From<String> for EvidenceError {
    fn from(message: String) -> Self {
        Self::Other(message)
    }
}
