//! Slot binding: whether a slot is safe to bind fresh or reuse, and the
//! path-scope check a snapshot enforces against a delivery's `paths`.

use crate::task_support::completed;
use crate::{AgentError, App, Rejection};
use domain::{
    authority::{self, Grant},
    delivery::Delivery,
    ids::{AuthorityId, SlotId, TaskId},
};

pub(super) fn validate_slot(
    state: &domain::state::State,
    task: &TaskId,
    slot: &SlotId,
    authority: Option<AuthorityId>,
) -> Result<bool, String> {
    let owners = slot_owners(state, slot);
    if owners
        .iter()
        .any(|owner| !state.tasks.get(owner).is_some_and(completed))
    {
        return Err("slot has an unfinished owner".into());
    }
    let historical = !owners.is_empty();
    match (historical, authority) {
        (false, None) => Ok(false),
        (true, Some(id)) => validate_reuse(state, task, slot, id).map(|()| true),
        (true, None) => Err("historical slot needs a slot-reuse grant".into()),
        (false, Some(_)) => Err("slot-reuse grant requires a historical checkout".into()),
    }
}

fn slot_owners(state: &domain::state::State, slot: &SlotId) -> Vec<TaskId> {
    state
        .tasks
        .values()
        .filter(|value| {
            value.deliveries.iter().any(|delivery| {
                delivery
                    .work
                    .as_ref()
                    .is_some_and(|work| work.slot.slot == *slot)
            })
        })
        .map(|value| value.id.clone())
        .collect()
}

fn validate_reuse(
    state: &domain::state::State,
    task: &TaskId,
    slot: &SlotId,
    id: AuthorityId,
) -> Result<(), String> {
    let value = state.tasks.get(task).ok_or("unknown task")?;
    let grant = value
        .authorities
        .iter()
        .find(|entry| entry.id == id)
        .ok_or("unknown authority")?;
    authority::validate_use(grant, &value.spec.requirements)
        .map_err(|_| "slot-reuse grant is spent or stale")?;
    let Grant::SlotReuse {
        historical,
        slot: granted,
    } = &grant.grant
    else {
        return Err("authority is not a slot-reuse grant".into());
    };
    let historical_owner_done = state.tasks.get(historical).is_some_and(completed);
    if granted != slot || !historical_owner_done {
        return Err("slot-reuse grant does not name a completed owner".into());
    }
    let unstarted = value.deliveries.last().is_none_or(|delivery| {
        delivery.work.is_none()
            && delivery.launches.is_empty()
            && delivery.operations.is_empty()
            && delivery.proof.entries.is_empty()
            && delivery.review.is_none()
    });
    if unstarted {
        Ok(())
    } else {
        Err("slot reuse requires an unstarted delivery".into())
    }
}

pub(super) fn bind_worktree(
    app: &App<'_>,
    task: &TaskId,
    slot: &SlotId,
    branch: &str,
    reuse: bool,
    check: bool,
) -> Result<(), AgentError> {
    let state = app
        .services
        .vcs
        .inspect_slot(slot)
        .map_err(|error| app.error("bind-slot", Some(task), Rejection::External(error.0)))?;
    let safe = matches!((&state, reuse), (domain::ports::SlotState::Missing, false))
        || matches!(
            (&state, reuse),
            (domain::ports::SlotState::Checkout { clean: true, .. }, true)
        );
    if !safe {
        return Err(app.error(
            "bind-slot",
            Some(task),
            Rejection::Conflict(format!("slot is not safe for this binding: {state:?}")),
        ));
    }
    if check {
        return Ok(());
    }
    let result = if reuse {
        app.services.vcs.reuse_slot(slot, branch)
    } else {
        app.services.vcs.bind_slot(task, slot, branch)
    };
    result.map_err(|error| app.error("bind-slot", Some(task), Rejection::External(error.0)))
}

pub(super) fn validate_paths(
    app: &App<'_>,
    task: &TaskId,
    delivery: &Delivery,
    base: &domain::ids::Sha,
    head: &domain::ids::Sha,
) -> Result<(), AgentError> {
    if delivery.paths.is_none() {
        return Ok(());
    }
    let commits = app
        .services
        .vcs
        .commit_paths(base, head)
        .map_err(|error| app.error("snapshot", Some(task), Rejection::External(error.0)))?;
    domain::delivery::paths_allow(delivery, &commits).map_err(|error| {
        app.error(
            "snapshot",
            Some(task),
            Rejection::Authority(format!("path scope rejected: {error:?}")),
        )
    })
}
