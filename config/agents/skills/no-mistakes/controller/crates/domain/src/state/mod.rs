mod apply;
mod apply_delivery;
mod next;

use crate::{
    acceptance::Snapshot,
    command::{Lesson, RunConfig},
    event::{Launch, Operation},
    ids::{Digest, TaskId},
    ports::Capabilities,
    risk::Profile,
    task::{Question, Task},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub config: Option<RunConfig>,
    pub profile: Option<Profile>,
    pub capabilities: Option<Capabilities>,
    pub tasks: BTreeMap<TaskId, Task>,
    pub order: Vec<TaskId>,
    pub questions: Vec<Question>,
    pub active: Option<TaskId>,
    pub launches: Vec<Launch>,
    pub operations: Vec<Operation>,
    pub lessons: Vec<(TaskId, Lesson)>,
    pub human_reviews: BTreeMap<TaskId, (Snapshot, Digest)>,
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
            operations: Vec::new(),
            lessons: Vec::new(),
            human_reviews: BTreeMap::new(),
            completed: BTreeSet::new(),
            frozen: false,
        }
    }
}

pub use apply::apply;
