//! Infallible fixture constructors for the golden-path scripts
//! (`../golden_path.rs`, `../golden_path_verification.rs`,
//! `../golden_path_recovery.rs`). A bad literal here is a broken test, not
//! a runtime condition to propagate, so each constructor panics with a
//! clear message instead of returning `Result` — building a `Step` table
//! then needs no `?` (design Section 17 rule 4).

use super::task_named;
use domain::{
    command::DiscoveredTask,
    ids::{CriterionId, Digest, JiraStatus, SlotId, TaskId, TransitionId},
};
use std::{path::PathBuf, str::FromStr};

fn parsed<T, E: std::fmt::Display>(what: &str, value: &str, result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("bad {what} literal {value:?}: {error}"))
}

pub fn task_id(value: &str) -> TaskId {
    parsed("task id", value, TaskId::from_str(value))
}

pub fn jira_status(value: &str) -> JiraStatus {
    parsed("Jira status", value, JiraStatus::from_str(value))
}

pub fn transition_id(value: &str) -> TransitionId {
    parsed("transition id", value, TransitionId::from_str(value))
}

pub fn criterion_id(value: &str) -> CriterionId {
    parsed("criterion id", value, CriterionId::from_str(value))
}

pub fn slot_id(value: &str) -> SlotId {
    parsed("slot id", value, SlotId::from_str(value))
}

/// A 64-character digest built by repeating `seed`, the shape every
/// fixture in these scripts uses for a baseline or evidence digest.
pub fn digest(seed: char) -> Digest {
    let value = seed.to_string().repeat(64);
    parsed("digest", &value, Digest::from_str(&value))
}

/// `id` discovered as a member subtask of GAIN-1 in `todo`, the shape
/// every golden-path script starts a task from.
pub fn discovered_task(id: &str) -> DiscoveredTask {
    task_named(id).unwrap_or_else(|error| panic!("bad discovered task {id:?}: {error}"))
}

/// Writes `contents` to `directory/name` and returns the path, panicking
/// on an IO failure rather than threading a `?` through table
/// construction.
pub fn write_text(directory: &std::path::Path, name: &str, contents: &str) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, contents)
        .unwrap_or_else(|error| panic!("could not write {path:?}: {error}"));
    path
}
