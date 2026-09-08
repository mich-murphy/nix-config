use super::State;
use super::apply_delivery::{
    close_delivery, disposition, finish_launch, invalidate_proof, narrow, observe_pr, record_proof,
    settle_operation, settle_review, use_authority, verify, with_task,
};
use crate::{
    acceptance::Snapshot,
    budget::{self, Budgets},
    command::DiscoveredTask,
    event::Event,
    ids::{Digest, Sha, TaskId},
    risk::{Signals, TierState, classify, raise},
    sync::Sync,
    task::{ExclusionReason, Phase, Question, Task, WorkStage},
};
use std::collections::BTreeMap;

pub fn apply(state: &mut State, event: &Event) {
    match event {
        Event::RunInitialized { config, profile } => {
            state.config = Some((**config).clone());
            state.profile = Some((**profile).clone());
        }
        Event::QueueDiscovered { tasks, order, .. } => apply_discovery(state, tasks, order),
        Event::QueueRefreshed { changed, removed } => apply_refresh(state, changed, removed),
        Event::QuestionRaised { task, text } => state.questions.push(Question {
            task: task.clone(),
            text: text.clone(),
        }),
        Event::Claimed { task, .. } => claim(state, task),
        Event::Held { task, reason, at } => with_task(state, task, |value| {
            value.hold = Some(crate::task::Hold {
                since: *at,
                reason: reason.clone(),
            })
        }),
        Event::Resumed { task } => with_task(state, task, |value| value.hold = None),
        Event::Briefed {
            task,
            criteria,
            requirements,
            tier,
            provisional,
        } => brief(state, task, criteria, requirements, *tier, *provisional),
        Event::TierRaised {
            task,
            to,
            reason,
            at,
        } => raise_tier(state, task, *to, reason, *at),
        Event::SlotBound {
            task,
            binding,
            branch,
            base,
        } => bind(state, task, binding, branch, base),
        Event::SlotReleased { task, .. } => release(state, task),
        Event::Planned { task, plan: value } => plan(state, task, value),
        Event::Snapshotted {
            task,
            delivery,
            snapshot: value,
            ..
        } => snapshot(state, task, *delivery, value),
        Event::Checkpointed {
            task,
            advanced,
            stalled,
            ..
        } => checkpoint(state, task, *advanced, *stalled),
        Event::SubtaskRecorded {
            task,
            issue,
            subtask,
        } => with_task(state, task, |value| {
            value.subtasks.insert(issue.clone(), subtask.clone());
        }),
        Event::Excluded { task, phase } => {
            with_task(state, task, |value| value.phase = phase.clone())
        }
        Event::LaunchStarted { launch } => state.launches.push(launch.clone()),
        Event::LaunchEnded { launch, result, .. } => finish_launch(state, *launch, result),
        Event::ProofRecorded {
            task,
            delivery,
            entries,
        } => record_proof(state, task, *delivery, entries),
        Event::ProofInvalidated { task, delivery, .. } => invalidate_proof(state, task, *delivery),
        Event::ReviewSettled {
            task,
            delivery,
            launch,
            findings,
            gaps,
            verdict,
        } => settle_review(state, task, *delivery, *launch, findings, gaps, *verdict),
        Event::Dispositioned {
            task,
            finding,
            disposition: value,
        } => disposition(state, task, finding, value),
        Event::HumanReviewed {
            task,
            snapshot,
            receipt,
        } => {
            state
                .human_reviews
                .insert(task.clone(), (snapshot.clone(), receipt.clone()));
        }
        Event::LessonRecorded { task, lesson } => {
            state.lessons.push((task.clone(), lesson.clone()))
        }
        Event::DeliveryOpened { task, delivery } => with_task(state, task, |value| {
            value.deliveries.push((**delivery).clone())
        }),
        Event::AcceptanceNarrowed {
            task,
            delivery,
            criteria,
            ..
        } => narrow(state, task, *delivery, criteria),
        Event::OperationStarted { operation } => state.operations.push(operation.clone()),
        Event::OperationSettled {
            operation, status, ..
        } => settle_operation(state, *operation, *status),
        Event::PrObserved { task, delivery, pr } => observe_pr(state, task, *delivery, pr),
        Event::DeliveryClosed {
            task,
            delivery,
            outcome,
        } => close_delivery(state, task, *delivery, outcome),
        Event::Verified { task, commit } => verify(state, task, commit),
        Event::Completed { task } => {
            state.completed.insert(task.clone());
        }
        Event::StatusIntended { task, .. } => {
            with_task(state, task, |value| value.sync = Sync::Pending)
        }
        Event::StatusObserved { task, sync, .. } => {
            with_task(state, task, |value| value.sync = sync.clone())
        }
        Event::AuthorityRegistered { task, authority } => with_task(state, task, |value| {
            value.authorities.push((**authority).clone())
        }),
        Event::GrantUsed {
            task,
            authority,
            by,
        } => use_authority(state, task, *authority, *by),
        Event::BudgetSpent {
            task,
            budget,
            counted,
        } => with_task(state, task, |value| {
            budget::spend(&mut value.budgets, *budget, *counted)
        }),
    }
}

fn apply_discovery(state: &mut State, tasks: &[DiscoveredTask], order: &[TaskId]) {
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

fn apply_refresh(state: &mut State, changed: &[DiscoveredTask], removed: &[TaskId]) {
    for found in changed {
        with_task(state, &found.id, |task| task.spec = found.spec.clone());
    }
    for id in removed {
        with_task(state, id, |task| {
            task.phase = Phase::Excluded(ExclusionReason::NotMember)
        });
    }
}

fn claim(state: &mut State, task: &TaskId) {
    state.active = Some(task.clone());
    with_task(state, task, |value| value.phase = Phase::Claimed);
}

fn brief(
    state: &mut State,
    task: &TaskId,
    criteria: &[crate::task::Criterion],
    requirements: &Digest,
    tier: crate::risk::Tier,
    provisional: bool,
) {
    with_task(state, task, |value| {
        value.spec.criteria = criteria.to_vec();
        value.spec.requirements = requirements.clone();
        value.tier.current = raise(value.tier.current, tier);
        value.tier.provisional = provisional;
    });
}

fn raise_tier(state: &mut State, task: &TaskId, tier: crate::risk::Tier, reason: &str, at: u64) {
    with_task(state, task, |value| {
        value.tier.current = raise(value.tier.current, tier);
        value.tier.history.push(crate::risk::TierChange {
            to: value.tier.current,
            reason: reason.to_owned(),
            at,
        });
    });
}

fn bind(
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
    };
    with_task(state, task, |value| value.phase = Phase::Planned { work });
}

fn release(state: &mut State, task: &TaskId) {
    if state.active.as_ref() == Some(task) {
        state.active = None;
    }
}

fn plan(state: &mut State, task: &TaskId, plan: &crate::task::Plan) {
    with_task(state, task, |value| match &mut value.phase {
        Phase::Planned { work } | Phase::InFlight { work, .. } => work.plan = Some(plan.clone()),
        _ => {}
    });
}

fn snapshot(
    state: &mut State,
    task: &TaskId,
    delivery: crate::ids::DeliveryId,
    snapshot: &Snapshot,
) {
    with_task(state, task, |value| {
        if let Some(item) = value.deliveries.iter_mut().find(|item| item.id == delivery) {
            item.review = None;
        }
        match &mut value.phase {
            Phase::Planned { work } => {
                work.snapshot = Some(snapshot.clone());
                value.phase = Phase::InFlight {
                    work: work.clone(),
                    stage: WorkStage::Validating,
                };
            }
            Phase::InFlight { work, .. } => work.snapshot = Some(snapshot.clone()),
            _ => {}
        }
    });
    state.human_reviews.remove(task);
}

fn checkpoint(state: &mut State, task: &TaskId, advanced: bool, stalled: u32) {
    with_task(state, task, |value| {
        value.budgets.stalled_checkpoints = if advanced { 0 } else { stalled };
    });
}
