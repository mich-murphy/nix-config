use crate::{
    Instant,
    ids::{JiraStatus, OperationId, TransitionId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sync {
    Pending,
    Unknown(StatusIntent),
    Confirmed(StatusReceipt),
    Failed(StatusFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusIntent {
    pub operation: OperationId,
    pub from: JiraStatus,
    pub target: JiraStatus,
    pub transition: TransitionId,
    pub attempts: u8,
    pub at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusReceipt {
    pub status: JiraStatus,
    pub observed: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusFailure {
    pub target: JiraStatus,
    pub actual: JiraStatus,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncError {
    UnknownOperation,
    RetryExhausted,
    StaleRead,
    AmbiguousTransition,
}

pub fn intend(current: &Sync, intent: StatusIntent) -> Result<Sync, SyncError> {
    if matches!(current, Sync::Unknown(_)) {
        return Err(SyncError::UnknownOperation);
    }
    if intent.attempts > 2 {
        return Err(SyncError::RetryExhausted);
    }
    Ok(Sync::Unknown(intent))
}

pub fn observe(current: &Sync, status: JiraStatus, at: Instant) -> Result<Sync, SyncError> {
    match current {
        Sync::Confirmed(receipt) if at < receipt.observed => Err(SyncError::StaleRead),
        Sync::Unknown(intent) if status == intent.target => Ok(Sync::Confirmed(StatusReceipt {
            status,
            observed: at,
        })),
        Sync::Unknown(intent) => Ok(Sync::Failed(StatusFailure {
            target: intent.target.clone(),
            actual: status,
            reason: "target status was not observed".into(),
        })),
        Sync::Pending | Sync::Confirmed(_) | Sync::Failed(_) => {
            Ok(Sync::Confirmed(StatusReceipt {
                status,
                observed: at,
            }))
        }
    }
}

pub fn transition<'a>(
    target: &JiraStatus,
    transitions: &'a [(TransitionId, JiraStatus)],
) -> Result<&'a TransitionId, SyncError> {
    let matches: Vec<_> = transitions
        .iter()
        .filter(|(_, status)| status == target)
        .collect();
    if matches.len() == 1 {
        Ok(&matches[0].0)
    } else {
        Err(SyncError::AmbiguousTransition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn intent() -> Result<StatusIntent, crate::ids::InvalidId> {
        Ok(StatusIntent {
            operation: OperationId(1),
            from: JiraStatus::from_str("todo")?,
            target: JiraStatus::from_str("progress")?,
            transition: TransitionId::from_str("31")?,
            attempts: 1,
            at: 10,
        })
    }

    #[test]
    fn unknown_status_blocks_replay() -> Result<(), crate::ids::InvalidId> {
        let sync = Sync::Unknown(intent()?);
        assert_eq!(intend(&sync, intent()?), Err(SyncError::UnknownOperation));
        Ok(())
    }

    #[test]
    fn observe_confirms_success() -> Result<(), crate::ids::InvalidId> {
        let intent = intent()?;
        let status = intent.target.clone();
        let sync = observe(&Sync::Unknown(intent), status, 11)
            .map_err(|_| crate::ids::InvalidId("sync"))?;
        assert!(matches!(sync, Sync::Confirmed(_)));
        Ok(())
    }
}
