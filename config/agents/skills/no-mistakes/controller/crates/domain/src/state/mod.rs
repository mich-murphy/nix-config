mod apply;
mod apply_delivery;
mod next;
mod projection;

use crate::{
    command::{Lesson, RunConfig},
    event::{Launch, Operation},
    ids::TaskId,
    ports::Capabilities,
    risk::Profile,
    task::{Question, Task},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub operations: Vec<Operation>,
    pub lessons: Vec<RecordedLesson>,
    pub frozen: bool,
}

/// One accepted or proposed lesson, tied to the task it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedLesson {
    pub task: TaskId,
    pub lesson: Lesson,
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
            frozen: false,
        }
    }
}

pub use apply::apply;
