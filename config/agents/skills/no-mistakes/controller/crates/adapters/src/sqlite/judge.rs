//! Bookkeeping for the judge beside the event log: rejected commands
//! (friction evidence) and the judges still running in the background
//! (so the trace instance closes after the last one). Like `trace`, both
//! tables are created on first use and never enter the projection.

use super::Store;
use domain::{ids::TaskId, judge::RejectionNote, ports::PortError};
use rusqlite::params;

fn port(error: impl std::fmt::Display) -> PortError {
    PortError(error.to_string())
}

impl Store {
    fn judge_tables(&self) -> Result<(), PortError> {
        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS rejections (
                    id INTEGER PRIMARY KEY,
                    at INTEGER NOT NULL,
                    command TEXT NOT NULL,
                    task TEXT,
                    check_only INTEGER NOT NULL DEFAULT 0,
                    class TEXT NOT NULL,
                    message TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS judges (task TEXT PRIMARY KEY)",
            )
            .map_err(port)
    }

    /// Appends one rejected command. Called from the CLI's failure path,
    /// so it must never itself fail loudly: the caller ignores the result.
    pub fn record_rejection(&self, note: &RejectionNote) -> Result<(), PortError> {
        self.judge_tables()?;
        self.connection
            .execute(
                "INSERT INTO rejections(at, command, task, check_only, class, message)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    i64::try_from(note.at).map_err(port)?,
                    note.command,
                    note.task.as_ref().map(ToString::to_string),
                    note.check,
                    note.class,
                    note.message,
                ],
            )
            .map(|_| ())
            .map_err(port)
    }

    /// Every rejection in order, oldest first.
    pub fn rejections(&self) -> Result<Vec<RejectionNote>, PortError> {
        self.judge_tables()?;
        let mut query = self
            .connection
            .prepare(
                "SELECT at, command, task, check_only, class, message FROM rejections ORDER BY id",
            )
            .map_err(port)?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(port)?;
        let mut notes = Vec::new();
        for row in rows {
            let (at, command, task, check, class, message) = row.map_err(port)?;
            notes.push(RejectionNote {
                at: u64::try_from(at).map_err(port)?,
                command,
                task: task.map(|text| text.parse()).transpose().map_err(port)?,
                check,
                class,
                message,
            });
        }
        Ok(notes)
    }

    /// Marks `task` as having a judge in flight. Idempotent.
    pub fn judge_started(&self, task: &TaskId) -> Result<(), PortError> {
        self.judge_tables()?;
        self.connection
            .execute(
                "INSERT OR IGNORE INTO judges(task) VALUES(?1)",
                params![task.to_string()],
            )
            .map(|_| ())
            .map_err(port)
    }

    /// Clears `task`'s in-flight mark and returns how many judges remain.
    pub fn judge_finished(&self, task: &TaskId) -> Result<u64, PortError> {
        self.judge_tables()?;
        self.connection
            .execute(
                "DELETE FROM judges WHERE task = ?1",
                params![task.to_string()],
            )
            .map_err(port)?;
        self.judges_pending()
    }

    pub fn judges_pending(&self) -> Result<u64, PortError> {
        self.judge_tables()?;
        self.connection
            .query_row("SELECT COUNT(*) FROM judges", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(port)
            .and_then(|count| u64::try_from(count).map_err(port))
    }
}
