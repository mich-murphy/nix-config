use domain::{
    event::{Actor, Event, EventRecord},
    state::{State, apply},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

pub struct Store {
    connection: Connection,
    root: PathBuf,
}

impl Store {
    pub fn create(root: &Path) -> Result<Self, StoreError> {
        std::fs::create_dir_all(root).map_err(error)?;
        let connection = Connection::open(root.join("state.sqlite3")).map_err(error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(error)?;
        connection
            .execute_batch(
                "CREATE TABLE events (
                seq INTEGER PRIMARY KEY,
                at INTEGER NOT NULL,
                actor TEXT NOT NULL,
                kind TEXT NOT NULL,
                event TEXT NOT NULL,
                phase TEXT NOT NULL,
                digest TEXT NOT NULL
             );
             CREATE TABLE projection (
                singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                state TEXT NOT NULL,
                digest TEXT NOT NULL
             );
             CREATE TABLE active_process (
                singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                kind TEXT NOT NULL,
                id INTEGER NOT NULL
             );",
            )
            .map_err(error)?;
        let empty = State::empty();
        let state = serde_json::to_string(&empty).map_err(error)?;
        connection
            .execute(
                "INSERT INTO projection(singleton, state, digest) VALUES(1, ?1, ?2)",
                params![state, chain_digest("", &empty)?],
            )
            .map_err(error)?;
        Ok(Self {
            connection,
            root: root.to_owned(),
        })
    }

    pub fn open(root: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open(root.join("state.sqlite3")).map_err(error)?;
        Ok(Self {
            connection,
            root: root.to_owned(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The read path: the materialised projection row, taken on trust. No
    /// fold and no comparison happen here, so a read costs one row lookup
    /// regardless of event log length. Use `verify` to check the
    /// projection against a fresh fold.
    pub fn state(&self) -> Result<State, StoreError> {
        read_projection(&self.connection)
    }

    /// Folds the entire event log and compares it to the stored
    /// projection, the tamper-evident check `state` used to perform on
    /// every read. Callers that need that assurance (`status`) call this
    /// explicitly instead of paying its cost on every read.
    pub fn verify(&self) -> Result<(), StoreError> {
        let projection = read_projection(&self.connection)?;
        let folded = self.fold()?;
        let projected = serde_json::to_vec(&projection).map_err(error)?;
        let expected = serde_json::to_vec(&folded).map_err(error)?;
        if projected == expected {
            Ok(())
        } else {
            Err(StoreError("projection does not match event fold".into()))
        }
    }

    pub fn commit(
        &mut self,
        actor: Actor,
        at: u64,
        events: &[Event],
    ) -> Result<Vec<EventRecord>, StoreError> {
        // BEGIN IMMEDIATE claims the write lock before any read, so a
        // second concurrent writer conflicts here rather than surviving
        // all the way to its own insert.
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        // Folds from the current projection plus the new events, not
        // from scratch: a read of the materialised row rather than a
        // replay of the whole event log.
        let mut state = read_projection(&transaction)?;
        let mut sequence = last_sequence(&transaction)?.saturating_add(1);
        let previous = last_digest(&transaction)?.unwrap_or_default();
        let mut digest = previous;
        let mut records = Vec::with_capacity(events.len());
        for event in events {
            let meta = AppendMeta {
                sequence,
                at,
                actor,
                previous: &digest,
            };
            let (record, next_digest) = append(&transaction, &mut state, event, meta)?;
            digest = next_digest;
            records.push(record);
            sequence = sequence.saturating_add(1);
        }
        let encoded = serde_json::to_string(&state).map_err(error)?;
        transaction
            .execute(
                "UPDATE projection SET state = ?1, digest = ?2 WHERE singleton = 1",
                params![encoded, digest],
            )
            .map_err(error)?;
        transaction.commit().map_err(error)?;
        Ok(records)
    }

    pub fn events(&self) -> Result<Vec<EventRecord>, StoreError> {
        let mut query = self
            .connection
            .prepare("SELECT seq, at, actor, event FROM events ORDER BY seq")
            .map_err(error)?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(error)?;
        let mut records = Vec::new();
        for row in rows {
            let (sequence, at, actor, event) = row.map_err(error)?;
            records.push(EventRecord {
                sequence: u64::try_from(sequence).map_err(error)?,
                at: u64::try_from(at).map_err(error)?,
                actor: serde_json::from_str(&format!("\"{actor}\"")).map_err(error)?,
                event: serde_json::from_str(&event).map_err(error)?,
            });
        }
        Ok(records)
    }

    fn fold(&self) -> Result<State, StoreError> {
        let mut state = State::empty();
        for record in self.events()? {
            apply(&mut state, &record.event);
        }
        Ok(state)
    }
}

fn read_projection(connection: &Connection) -> Result<State, StoreError> {
    let stored: String = connection
        .query_row(
            "SELECT state FROM projection WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(error)?;
    serde_json::from_str(&stored).map_err(error)
}

fn last_sequence(connection: &Connection) -> Result<u64, StoreError> {
    connection
        .query_row("SELECT COALESCE(MAX(seq), 0) FROM events", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(error)
        .and_then(|value| u64::try_from(value).map_err(error))
}

fn last_digest(connection: &Connection) -> Result<Option<String>, StoreError> {
    connection
        .query_row(
            "SELECT digest FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(error)
}

struct AppendMeta<'a> {
    sequence: u64,
    at: u64,
    actor: Actor,
    previous: &'a str,
}

fn append(
    transaction: &rusqlite::Transaction<'_>,
    state: &mut State,
    event: &Event,
    meta: AppendMeta<'_>,
) -> Result<(EventRecord, String), StoreError> {
    update_process_claim(transaction, event)?;
    apply(state, event);
    let digest = chain_digest(meta.previous, state)?;
    let record = EventRecord {
        sequence: meta.sequence,
        at: meta.at,
        actor: meta.actor,
        event: event.clone(),
    };
    insert(transaction, &record, &phase(event, state), &digest)?;
    Ok((record, digest))
}

fn update_process_claim(
    transaction: &rusqlite::Transaction<'_>,
    event: &Event,
) -> Result<(), StoreError> {
    match event {
        Event::LaunchStarted { launch } => claim_process(transaction, "launch", launch.id.0)?,
        Event::OperationStarted { operation } => {
            claim_process(transaction, "operation", operation.id.0)?;
        }
        Event::LaunchEnded { launch, .. } => clear_process(transaction, "launch", launch.0)?,
        Event::OperationSettled { operation, .. } => {
            clear_process(transaction, "operation", operation.0)?;
        }
        _ => {}
    }
    Ok(())
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

fn insert(
    transaction: &rusqlite::Transaction<'_>,
    record: &EventRecord,
    phase: &str,
    digest: &str,
) -> Result<(), StoreError> {
    let actor = serde_json::to_string(&record.actor).map_err(error)?;
    let event = serde_json::to_string(&record.event).map_err(error)?;
    let kind = event_kind(&event)?;
    transaction.execute(
        "INSERT INTO events(seq, at, actor, kind, event, phase, digest) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![i64::try_from(record.sequence).map_err(error)?, i64::try_from(record.at).map_err(error)?, actor.trim_matches('"'), kind, event, phase, digest],
    ).map_err(error)?;
    Ok(())
}

fn chain_digest(previous: &str, state: &State) -> Result<String, StoreError> {
    let mut hash = Sha256::new();
    hash.update(previous.as_bytes());
    hash.update(serde_json::to_vec(state).map_err(error)?);
    Ok(format!("{:x}", hash.finalize()))
}

fn event_kind(encoded: &str) -> Result<String, StoreError> {
    #[derive(serde::Deserialize)]
    struct Kind {
        kind: String,
    }
    serde_json::from_str::<Kind>(encoded)
        .map(|value| value.kind)
        .map_err(error)
}

fn phase(event: &Event, state: &State) -> String {
    let task = event_task(event);
    task.and_then(|id| state.tasks.get(id))
        .map_or_else(|| "run".into(), |task| format!("{:?}", task.phase))
}

fn event_task(event: &Event) -> Option<&domain::ids::TaskId> {
    match event {
        Event::QuestionRaised { task, .. } => task.as_ref(),
        Event::Claimed { task, .. }
        | Event::Held { task, .. }
        | Event::Resumed { task }
        | Event::Briefed { task, .. }
        | Event::TierRaised { task, .. }
        | Event::SlotBound { task, .. }
        | Event::SlotReleased { task, .. }
        | Event::Planned { task, .. }
        | Event::Snapshotted { task, .. }
        | Event::Checkpointed { task, .. }
        | Event::SubtaskRecorded { task, .. }
        | Event::ProofRecorded { task, .. }
        | Event::ProofInvalidated { task, .. }
        | Event::ReviewSettled { task, .. }
        | Event::Dispositioned { task, .. }
        | Event::HumanReviewed { task, .. }
        | Event::LessonRecorded { task, .. }
        | Event::DeliveryOpened { task, .. }
        | Event::AcceptanceNarrowed { task, .. }
        | Event::PrObserved { task, .. }
        | Event::DeliveryClosed { task, .. }
        | Event::Verified { task, .. }
        | Event::Completed { task }
        | Event::StatusIntended { task, .. }
        | Event::StatusObserved { task, .. }
        | Event::AuthorityRegistered { task, .. }
        | Event::AuthorityRepurposed { task, .. }
        | Event::GrantUsed { task, .. }
        | Event::PairSpent { task, .. }
        | Event::BudgetSpent { task, .. } => Some(task),
        Event::LaunchStarted { launch } => Some(&launch.task),
        Event::OperationStarted { operation } => Some(&operation.task),
        Event::RunInitialized { .. }
        | Event::QueueDiscovered { .. }
        | Event::QueueRefreshed { .. }
        | Event::LaunchEnded { .. }
        | Event::OperationSettled { .. } => None,
    }
}

fn error(value: impl std::fmt::Display) -> StoreError {
    StoreError(value.to_string())
}

#[cfg(test)]
mod tests;
