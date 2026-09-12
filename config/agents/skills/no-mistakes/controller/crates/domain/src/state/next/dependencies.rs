use crate::{
    ids::TaskId,
    state::State,
    task::{HoldReason, Phase},
};
use std::collections::BTreeSet;

/// A needs-human hold may reveal an unfinished prerequisite after the held
/// delivery was opened. Keep that prerequisite inside the same governed queue
/// and choose its earliest eligible leaf before unrelated work.
pub(super) fn held_dependency_candidate(state: &State) -> Option<TaskId> {
    let pending = held_dependency_roots(state);
    let required = collect_required_dependencies(state, pending);
    state
        .order
        .iter()
        .find(|id| required.contains(*id) && eligible(state, id))
        .cloned()
}

fn held_dependency_roots(state: &State) -> Vec<TaskId> {
    state
        .tasks
        .values()
        .filter(|task| {
            matches!(
                task.hold.as_ref().map(|hold| &hold.reason),
                Some(HoldReason::NeedsHuman { .. })
            )
        })
        .flat_map(|task| task.spec.dependencies.iter())
        .filter(|dependency| dependency.code && dependency.main_commit.is_none())
        .map(|dependency| dependency.task.clone())
        .collect()
}

fn collect_required_dependencies(state: &State, mut pending: Vec<TaskId>) -> BTreeSet<TaskId> {
    let mut required = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !required.insert(id.clone()) {
            continue;
        }
        let Some(task) = state.tasks.get(&id) else {
            continue;
        };
        pending.extend(
            task.spec
                .dependencies
                .iter()
                .filter(|dependency| dependency.code && dependency.main_commit.is_none())
                .map(|dependency| dependency.task.clone()),
        );
    }
    required
}

fn eligible(state: &State, id: &TaskId) -> bool {
    let Some(task) = state.tasks.get(id) else {
        return false;
    };
    matches!(task.phase, Phase::Queued)
        && task.spec.not_before.is_none()
        && task
            .spec
            .dependencies
            .iter()
            .all(|dependency| !dependency.code || dependency.main_commit.is_some())
}
