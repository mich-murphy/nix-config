use super::State;
use super::apply_delivery::with_task;
use crate::{
    acceptance::Snapshot,
    budget::Budgets,
    command::DiscoveredTask,
    ids::{Digest, Sha, TaskId},
    risk::{Signals, TierState, classify, raise},
    sync::Sync,
    task::{ExclusionReason, Phase, Question, Task},
};
use std::collections::BTreeMap;

pub(super) fn apply_discovery(state: &mut State, tasks: &[DiscoveredTask], order: &[TaskId]) {
    state.frozen = true;
    state.order = order.to_vec();
    for found in tasks {
        let signals = Signals {
            files: 0,
            human_only_criteria: found
                .spec
                .criteria
                .iter()
                .filter(|criterion| criterion.human_only)
                .count() as u32,
            dependencies: found.spec.dependencies.len() as u32,
            ..Signals::default()
        };
        let phase = initial_phase(found);
        state.tasks.insert(
            found.id.clone(),
            Task {
                id: found.id.clone(),
                spec: found.spec.clone(),
                phase,
                hold: None,
                sync: Sync::Pending,
                budgets: Budgets::default(),
                deliveries: Vec::new(),
                authorities: Vec::new(),
                tier: TierState {
                    current: classify(&signals),
                    provisional: true,
                    history: Vec::new(),
                },
                subtasks: BTreeMap::new(),
                baselines: BTreeMap::new(),
                judgment: None,
            },
        );
    }
}

fn initial_phase(found: &DiscoveredTask) -> Phase {
    if !found.spec.member {
        Phase::Excluded(ExclusionReason::NotMember)
    } else if found.spec.was_terminal {
        Phase::Excluded(ExclusionReason::TerminalBeforeRun)
    } else if !found.spec.ownership_clear {
        Phase::NeedsInput(Question {
            task: Some(found.id.clone()),
            text: found.spec.ownership_evidence.clone(),
        })
    } else if let Some(dependency) = found
        .spec
        .dependencies
        .iter()
        .find(|dependency| dependency.code && dependency.main_commit.is_none())
    {
        Phase::Blocked(crate::task::BlockReason::Dependency(
            dependency.task.clone(),
        ))
    } else {
        Phase::Queued
    }
}

pub(super) fn apply_refresh(state: &mut State, changed: &[DiscoveredTask], removed: &[TaskId]) {
    for found in changed {
        with_task(state, &found.id, |task| refresh_task(task, found));
        state
            .questions
            .retain(|question| question.task.as_ref() != Some(&found.id));
        if matches!(initial_phase(found), Phase::NeedsInput(_)) {
            state.questions.push(Question {
                task: Some(found.id.clone()),
                text: found.spec.ownership_evidence.clone(),
            });
        }
    }
    for id in removed {
        if state.active.as_ref() == Some(id) {
            state.active = None;
        }
        state
            .questions
            .retain(|question| question.task.as_ref() != Some(id));
        with_task(state, id, |task| {
            task.phase = Phase::Excluded(ExclusionReason::NotMember);
            task.hold = None;
        });
    }
}

fn refresh_task(task: &mut Task, found: &DiscoveredTask) {
    task.spec = found.spec.clone();
    if matches!(
        task.phase,
        Phase::Queued | Phase::Blocked(_) | Phase::NeedsInput(_)
    ) {
        task.phase = initial_phase(found);
    }
}

pub(super) fn claim(state: &mut State, task: &TaskId) {
    state.active = Some(task.clone());
    with_task(state, task, |value| value.phase = Phase::Claimed);
}

pub(super) fn brief(
    state: &mut State,
    task: &TaskId,
    criteria: &[crate::task::Criterion],
    requirements: &Digest,
    tier: crate::risk::Tier,
    provisional: bool,
) {
    with_task(state, task, |value| {
        if value.spec.requirements != *requirements {
            invalidate_plan(value);
        }
        value.spec.criteria = criteria.to_vec();
        value.spec.requirements = requirements.clone();
        value.tier.current = raise(value.tier.current, tier);
        value.tier.provisional = provisional;
    });
}

fn invalidate_plan(task: &mut Task) {
    if let Some(delivery) = task.deliveries.last_mut()
        && let Some(work) = delivery.work.as_mut()
    {
        work.plan = None;
        work.snapshot = None;
    }
}

pub(super) fn raise_tier(
    state: &mut State,
    task: &TaskId,
    tier: crate::risk::Tier,
    reason: &str,
    at: u64,
) {
    with_task(state, task, |value| {
        value.tier.current = raise(value.tier.current, tier);
        value.tier.history.push(crate::risk::TierChange {
            to: value.tier.current,
            reason: reason.to_owned(),
            at,
        });
    });
}

pub(super) fn bind(
    state: &mut State,
    task: &TaskId,
    binding: &crate::task::SlotBinding,
    branch: &str,
    base: &Sha,
) {
    let work = crate::task::PlannedWork {
        slot: binding.clone(),
        branch: branch.to_owned(),
        base: base.clone(),
        snapshot: None,
        plan: None,
        feedback: Vec::new(),
        snapshot_launch: None,
    };
    with_task(state, task, |value| {
        if let Some(delivery) = value.deliveries.last_mut()
            && matches!(delivery.outcome, crate::delivery::Outcome::Open)
        {
            delivery.work = Some(work.clone());
        }
        value.phase = Phase::Planned;
    });
}

pub(super) fn release(state: &mut State, task: &TaskId) {
    if state.active.as_ref() == Some(task) {
        state.active = None;
    }
}

pub(super) fn plan(
    state: &mut State,
    task: &TaskId,
    plan: &crate::task::Plan,
    feedback: &[crate::command::Lesson],
) {
    with_task(state, task, |value| {
        if let Some(work) = value
            .deliveries
            .last_mut()
            .and_then(|delivery| delivery.work.as_mut())
        {
            work.plan = Some(plan.clone());
            work.feedback = feedback.to_vec();
        }
        for (criterion, digest) in &plan.baselines {
            value
                .baselines
                .entry(criterion.clone())
                .or_insert_with(|| digest.clone());
        }
    });
}

pub(super) fn snapshot(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    snapshot: &Snapshot,
    covers: Option<crate::ids::LaunchId>,
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            update_delivery_snapshot(item, snapshot, covers);
        }
        if matches!(value.phase, Phase::Planned) {
            value.phase = Phase::InFlight;
        }
    });
}

fn update_delivery_snapshot(
    delivery: &mut crate::delivery::Delivery,
    snapshot: &Snapshot,
    covers: Option<crate::ids::LaunchId>,
) {
    delivery.prior_review = delivery.review.take();
    delivery.human_review = None;
    if let Some(work) = delivery.work.as_mut() {
        work.snapshot = Some(snapshot.clone());
        work.snapshot_launch = covers;
    }
}

pub(super) fn checkpoint(
    state: &mut State,
    task: &TaskId,
    launch: Option<crate::ids::LaunchId>,
    advanced: bool,
    stalled: u32,
) {
    if let Some(id) = launch
        && let Some(item) = state.launches.iter_mut().find(|item| item.id == id)
    {
        item.checkpointed = true;
    }
    with_task(state, task, |value| {
        value.budgets.stalled_checkpoints = if advanced { 0 } else { stalled };
    });
}
