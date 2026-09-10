//! `TraceState` over the run's own SQLite file: one key/value table beside
//! the event log, created on first use so runs initialized before tracing
//! existed keep working. Nothing here enters the projection or its digest.

use super::Store;
use domain::ports::{PortError, TraceState};
use rusqlite::{OptionalExtension, params};

fn port(error: impl std::fmt::Display) -> PortError {
    PortError(error.to_string())
}

impl Store {
    fn trace_table(&self) -> Result<(), PortError> {
        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS trace (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            )
            .map_err(port)
    }
}

impl TraceState for Store {
    fn get(&self, key: &str) -> Result<Option<String>, PortError> {
        self.trace_table()?;
        self.connection
            .query_row(
                "SELECT value FROM trace WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(port)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), PortError> {
        self.trace_table()?;
        self.connection
            .execute(
                "INSERT INTO trace(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
            .map_err(port)
    }

    fn remove(&mut self, key: &str) -> Result<(), PortError> {
        self.trace_table()?;
        self.connection
            .execute("DELETE FROM trace WHERE key = ?1", params![key])
            .map(|_| ())
            .map_err(port)
    }
}
