mod apply;
mod apply_delivery;
mod next;
mod projection;

use crate::{
    acceptance::Snapshot,
    command::{Lesson, RunConfig},
    event::{Launch, Operation},
    ids::{Digest, LaunchId, TaskId},
    ports::Capabilities,
    risk::Profile,
    task::{Question, Task},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub config: Option<RunConfig>,
    pub profile: Option<Profile>,
    pub capabilities: Option<Capabilities>,
    pub tasks: BTreeMap<TaskId, Task>,
    pub order: Vec<TaskId>,
    pub questions: Vec<Question>,
    pub active: Option<TaskId>,
    pub launches: Vec<Launch>,
    pub checkpoints: BTreeMap<TaskId, LaunchId>,
    pub usage: BTreeMap<LaunchId, Option<crate::ports::Tokens>>,
    pub operations: Vec<Operation>,
    pub lessons: Vec<(TaskId, Lesson)>,
    pub human_reviews: BTreeMap<TaskId, (Snapshot, Digest)>,
    pub check_deadlines: BTreeMap<String, crate::Instant>,
    pub completed: BTreeSet<TaskId>,
    pub frozen: bool,
}

impl State {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            config: None,
            profile: None,
            capabilities: None,
            tasks: BTreeMap::new(),
            order: Vec::new(),
            questions: Vec::new(),
            active: None,
            launches: Vec::new(),
            checkpoints: BTreeMap::new(),
            usage: BTreeMap::new(),
            operations: Vec::new(),
            lessons: Vec::new(),
            human_reviews: BTreeMap::new(),
            check_deadlines: BTreeMap::new(),
            completed: BTreeSet::new(),
            frozen: false,
        }
    }
}

pub use apply::apply;
