use crate::{
    Instant,
    command::NextAction,
    state::State,
    task::{HoldReason, Task},
};

/// Priority of an actionable hold, lowest value first: an expired CI wait
/// resumes ahead of a needs-human hold, which is reported ahead of a budget
/// or superseded-PR hold. `None` means the hold is not yet actionable (a
/// CI wait whose deadline has not passed).
fn hold_priority(reason: &HoldReason, now: Instant) -> Option<u8> {
    match reason {
        HoldReason::CiPending { deadline, .. } if *deadline <= now => Some(0),
        HoldReason::CiPending { .. } => None,
        HoldReason::NeedsHuman { .. } => Some(1),
        HoldReason::BudgetExhausted(_) | HoldReason::SupersededPr { .. } => Some(2),
    }
}

fn hold_action(task: &Task, reason: &HoldReason) -> NextAction {
    match reason {
        HoldReason::NeedsHuman { remaining, .. } => NextAction::OpenDelivery {
            task: task.id.clone(),
            remaining: remaining.clone(),
        },
        HoldReason::CiPending { .. } => NextAction::Resume {
            task: task.id.clone(),
        },
        HoldReason::BudgetExhausted(_) | HoldReason::SupersededPr { .. } => NextAction::Hold {
            task: task.id.clone(),
            reason: reason.clone(),
        },
    }
}

/// Actionable holds surfaced ahead of claiming new work. A hold releases
/// the active task (`apply::apply` clears `state.active` on `Held`) so
/// another task may be claimed, but the coordinator must still see and act
/// on the hold rather than have it fall silent. Scanned in queue order,
/// highest priority first: an expired CI wait resumes, a needs-human hold
/// asks for the delivery it is waiting on, and a budget or superseded-PR
/// hold is reported.
pub(super) fn held_action(state: &State, now: Instant) -> Option<NextAction> {
    ordered_held(state)
        .filter_map(|task| {
            let hold = task.hold.as_ref()?;
            if matches!(hold.reason, HoldReason::NeedsHuman { .. })
                && task
                    .spec
                    .dependencies
                    .iter()
                    .any(|dependency| dependency.code && dependency.main_commit.is_none())
            {
                return None;
            }
            let priority = hold_priority(&hold.reason, now)?;
            Some((priority, task, &hold.reason))
        })
        .min_by_key(|(priority, _, _)| *priority)
        .map(|(_, task, reason)| hold_action(task, reason))
}

/// When nothing else is claimable, wait for the earliest still-pending CI
/// deadline rather than reporting idle: the coordinator should hold open
/// rather than assume the run is finished.
pub(super) fn pending_ci_wait(state: &State, now: Instant) -> Option<NextAction> {
    ordered_held(state)
        .filter_map(|task| match &task.hold {
            Some(hold) => match &hold.reason {
                HoldReason::CiPending { pr, deadline, .. } if *deadline > now => {
                    Some((task.id.clone(), *pr, *deadline))
                }
                _ => None,
            },
            None => None,
        })
        .min_by_key(|(_, _, deadline)| *deadline)
        .map(|(task, pr, deadline)| NextAction::AwaitChecks { task, pr, deadline })
}

fn ordered_held(state: &State) -> impl Iterator<Item = &Task> {
    state
        .order
        .iter()
        .filter_map(move |id| state.tasks.get(id))
        .filter(|task| task.hold.is_some())
}
