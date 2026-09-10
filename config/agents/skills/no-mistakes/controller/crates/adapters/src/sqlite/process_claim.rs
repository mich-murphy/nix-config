//! The single active-process claim: which launch or operation currently
//! owns the run's one foreground process, taken when it starts and
//! released when it settles. Kept apart from the store so `sqlite.rs`
//! stays within the file gate.

use super::{StoreError, error};
use domain::event::Event;
use rusqlite::params;

pub(super) fn update_process_claim(
    transaction: &rusqlite::Transaction<'_>,
    event: &Event,
) -> Result<(), StoreError> {
    match process_claim(event) {
        Some(ProcessClaim::Take(kind, id)) => claim_process(transaction, kind, id),
        Some(ProcessClaim::Release(kind, id)) => clear_process(transaction, kind, id),
        None => Ok(()),
    }
}

/// What an event does to the single active-process claim.
enum ProcessClaim {
    Take(&'static str, u64),
    Release(&'static str, u64),
}

fn process_claim(event: &Event) -> Option<ProcessClaim> {
    match event {
        // A judge runs in the background beside whatever the coordinator
        // owns next, so it never takes the single active-process claim.
        Event::LaunchStarted { launch } if launch.role == domain::command::AgentRole::Judge => None,
        Event::LaunchStarted { launch } => Some(ProcessClaim::Take("launch", launch.id.0)),
        Event::OperationStarted { operation } => {
            Some(ProcessClaim::Take("operation", operation.id.0))
        }
        // Clearing a claim a judge never took deletes no row.
        Event::LaunchEnded { launch, .. } => Some(ProcessClaim::Release("launch", launch.0)),
        Event::OperationSettled { operation, .. } => {
            Some(ProcessClaim::Release("operation", operation.0))
        }
        _ => None,
    }
}

fn claim_process(
    transaction: &rusqlite::Transaction<'_>,
    kind: &str,
    id: u64,
) -> Result<(), StoreError> {
    transaction
        .execute(
            "INSERT INTO active_process(singleton, kind, id) VALUES(1, ?1, ?2)",
            params![kind, i64::try_from(id).map_err(error)?],
        )
        .map(|_| ())
        .map_err(error)
}

fn clear_process(
    transaction: &rusqlite::Transaction<'_>,
    kind: &str,
    id: u64,
) -> Result<(), StoreError> {
    transaction
        .execute(
            "DELETE FROM active_process WHERE singleton = 1 AND kind = ?1 AND id = ?2",
            params![kind, i64::try_from(id).map_err(error)?],
        )
        .map(|_| ())
        .map_err(error)
}
