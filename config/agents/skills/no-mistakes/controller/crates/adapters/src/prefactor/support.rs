//! Small pieces the tracer leans on: the payload file handed to the CLI,
//! the pending-record queue in `TraceState`, and timestamp formatting.

use domain::{
    Instant,
    event::EventRecord,
    ports::{PortError, TraceState},
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) const PENDING: &str = "pending";
/// Distinguishes payload files and bulk keys within one process.
static COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn next_number() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// A JSON payload handed to the CLI as `@file`, removed when dropped.
/// Payloads go through a file because one argv entry has a hard size limit.
pub(super) struct PayloadFile(PathBuf);

impl PayloadFile {
    pub(super) fn new(value: &Value) -> Result<Self, PortError> {
        let path = std::env::temp_dir().join(format!(
            "no-mistakes-trace-{}-{}.json",
            std::process::id(),
            next_number()
        ));
        std::fs::write(&path, value.to_string()).map_err(|error| PortError(error.to_string()))?;
        Ok(Self(path))
    }

    pub(super) fn arg(&self) -> String {
        format!("@{}", self.0.display())
    }
}

impl Drop for PayloadFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub(super) fn load_pending(state: &dyn TraceState) -> Vec<EventRecord> {
    state
        .get(PENDING)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub(super) fn save_pending(state: &mut dyn TraceState, pending: &[EventRecord]) {
    if pending.is_empty() {
        let _ = state.remove(PENDING);
    } else if let Ok(text) = serde_json::to_string(pending) {
        let _ = state.set(PENDING, &text);
    }
}

/// Formats Unix seconds as UTC RFC 3339 (civil-from-days, no dependency).
pub(crate) fn rfc3339(at: Instant) -> String {
    let days = at / 86_400;
    let seconds = at % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}
