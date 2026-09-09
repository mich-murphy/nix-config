use crate::{
    Instant,
    ids::{JiraStatus, OperationId, TransitionId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Sync {
    Pending,
    Unknown(StatusIntent),
    Confirmed(StatusReceipt),
    Failed(StatusFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusIntent {
    pub operation: OperationId,
    pub from: JiraStatus,
    pub target: JiraStatus,
    pub transition: TransitionId,
    pub attempts: u8,
    pub at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusReceipt {
    pub status: JiraStatus,
    pub observed: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusFailure {
    pub intent: StatusIntent,
    pub actual: JiraStatus,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
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
            intent: intent.clone(),
            actual: status,
            reason: "target status was not observed".into(),
        })),
        Sync::Confirmed(receipt) if status != receipt.status => Err(SyncError::StaleRead),
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

    #[test]
    fn status_retry_is_bounded() -> Result<(), crate::ids::InvalidId> {
        let mut retry = intent()?;
        retry.attempts = 3;
        assert_eq!(
            intend(&Sync::Pending, retry),
            Err(SyncError::RetryExhausted)
        );
        Ok(())
    }

    #[test]
    fn stale_read_cannot_overwrite() -> Result<(), crate::ids::InvalidId> {
        let done = JiraStatus::from_str("done")?;
        let confirmed = Sync::Confirmed(StatusReceipt {
            status: done,
            observed: 20,
        });
        assert_eq!(
            observe(&confirmed, JiraStatus::from_str("progress")?, 21),
            Err(SyncError::StaleRead)
        );
        Ok(())
    }

    #[test]
    fn ambiguous_transition_rejected() -> Result<(), crate::ids::InvalidId> {
        let target = JiraStatus::from_str("done")?;
        let choices = [
            (TransitionId::from_str("1")?, target.clone()),
            (TransitionId::from_str("2")?, target.clone()),
        ];
        assert_eq!(
            transition(&target, &choices),
            Err(SyncError::AmbiguousTransition)
        );
        Ok(())
    }
}
