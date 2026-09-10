mod format;
mod holds;
mod publish;
mod sync;

use self::format::{command_for, template_for};
use self::holds::{held_action, pending_ci_wait};
use self::publish::{publish_action, verification_action};
use self::sync::sync_step;
use super::State;
use crate::{
    Instant,
    acceptance::Snapshot,
    budget::{self, BudgetKind},
    command::{ActionEnvelope, NextAction},
    delivery::{Delivery, DeliveryKind, PrState},
    event::{Launch, OperationStatus},
    ids::{CriterionId, FindingId, IssueKey, LaunchId, TaskId},
    review::Verdict,
    task::{Phase, PlannedWork, Task},
};

impl State {
    #[must_use]
    pub fn next(&self, now: Instant) -> Option<ActionEnvelope> {
        let action = self.choose_action(now)?;
        let name = command_for(&action);
        let template = template_for(&action);
        Some(ActionEnvelope {
            action,
            name,
            template,
        })
    }

    fn choose_action(&self, now: Instant) -> Option<NextAction> {
        self.config.as_ref()?;
        // A judge runs in the background beside the coordinator's own
        // work, so an open judge launch never asks to be monitored.
        if let Some(launch) = self.launches.iter().find(|launch| {
            launch.outcome.is_none() && launch.role != crate::command::AgentRole::Judge
        }) {
            return Some(NextAction::MonitorLaunch { launch: launch.id });
        }
        if let Some(operation) = self.operations.iter().find(|operation| {
            matches!(
                operation.status,
                OperationStatus::Running | OperationStatus::Unknown
            ) && self.operation_is_open(operation)
        }) {
            return Some(NextAction::SettleOperation {
                operation: operation.id,
            });
        }
        if let Some(task) = self.active.clone() {
            return self.active_action(&task);
        }
        if let Some(action) = held_action(self, now) {
            return Some(action);
        }
        self.queue_action(now)
    }

    fn operation_is_open(&self, operation: &crate::event::Operation) -> bool {
        self.tasks
            .get(&operation.task)
            .and_then(|task| {
                task.deliveries
                    .iter()
                    .find(|delivery| delivery.id == operation.delivery)
            })
            .is_some_and(|delivery| matches!(delivery.outcome, crate::delivery::Outcome::Open))
    }

    fn active_action(&self, task: &TaskId) -> Option<NextAction> {
        if let Some(launch) = self.pending_checkpoint(task) {
            return Some(NextAction::Checkpoint {
                task: task.clone(),
                launch,
            });
        }
        self.tasks
            .get(task)
            .and_then(|value| self.task_action(value))
    }

    fn pending_checkpoint(&self, task: &TaskId) -> Option<LaunchId> {
        let latest = self.latest_implementer_launch(task)?;
        (!latest.checkpointed).then_some(latest.id)
    }

    /// The most recent implementer launch that has settled, or `None` if
    /// none has (either no implementer turn has run yet, or the current
    /// one is still in flight). `pending_checkpoint` and `needs_snapshot`
    /// are the only readers: both derive their answer from this one
    /// launch's own `checkpointed` flag rather than a separate map.
    fn latest_implementer_launch(&self, task: &TaskId) -> Option<&Launch> {
        self.launches.iter().rev().find(|launch| {
            launch.task == *task
                && launch.role == crate::command::AgentRole::Implementer
                && launch.outcome.is_some()
        })
    }

    fn queue_action(&self, now: Instant) -> Option<NextAction> {
        let config = self.config.as_ref()?;
        let unfinished: Vec<_> = self
            .tasks
            .values()
            .filter(|task| unfinished_pr(task))
            .map(|task| task.id.clone())
            .collect();
        if unfinished.len() >= config.max_open_prs as usize {
            return Some(NextAction::RecoverUnfinished { tasks: unfinished });
        }
        if let Some(task) = self.order.iter().find_map(|id| self.eligible(id)) {
            return Some(NextAction::Claim { task });
        }
        // An open question holds only the task it is about (its phase is
        // `NeedsInput`, so `eligible` skips it); it is put to the
        // coordinator once no other task can proceed, never ahead of the
        // rest of the queue.
        if !self.questions.is_empty() {
            return Some(NextAction::AnswerQuestions {
                questions: self.questions.clone(),
            });
        }
        if let Some(action) = pending_ci_wait(self, now) {
            return Some(action);
        }
        let remaining = self
            .tasks
            .values()
            .filter(|task| !terminal(&task.phase))
            .map(|task| task.id.clone())
            .collect();
        Some(NextAction::Report { remaining })
    }

    fn eligible(&self, id: &TaskId) -> Option<TaskId> {
        let task = self.tasks.get(id)?;
        if !matches!(task.phase, Phase::Queued) || task.spec.not_before.is_some() {
            return None;
        }
        let dependencies_ready = task
            .spec
            .dependencies
            .iter()
            .all(|dependency| !dependency.code || dependency.main_commit.is_some());
        dependencies_ready.then(|| id.clone())
    }

    fn task_action(&self, task: &Task) -> Option<NextAction> {
        match &task.phase {
            Phase::Queued | Phase::Blocked(_) | Phase::NeedsInput(_) | Phase::Excluded(_) => None,
            Phase::Claimed => Some(self.claimed_action(task)),
            Phase::Planned => Some(self.planned_action(task, task.current_work()?)),
            Phase::InFlight => self.in_flight_action(task, task.current_work()?),
            Phase::Merged { commit, .. } => self.merged_action(task, commit),
            Phase::Verified { .. } => self.verified_action(task),
            Phase::Completed { .. } => self.completed_action(task),
        }
    }

    fn claimed_action(&self, task: &Task) -> NextAction {
        let Some(progress) = self
            .config
            .as_ref()
            .map(|config| config.jira.statuses.progress.clone())
        else {
            return NextAction::Brief {
                task: task.id.clone(),
            };
        };
        let issue = IssueKey::from(task.id.clone());
        sync_step(&task.id, &issue, &task.sync, &progress).unwrap_or_else(|| claimed_ready(task))
    }

    fn planned_action(&self, task: &Task, work: &PlannedWork) -> NextAction {
        if work.plan.is_none() {
            return NextAction::Plan {
                task: task.id.clone(),
            };
        }
        if self.needs_snapshot(task, work) {
            return NextAction::Snapshot {
                task: task.id.clone(),
            };
        }
        NextAction::Implement {
            task: task.id.clone(),
            remaining_turns: budget::remaining(
                BudgetKind::Implementation,
                &task.budgets,
                task.tier.current,
                &task.authorities,
                self.config.as_ref().map(|config| &config.caps),
            ),
        }
    }

    /// After an implementer launch has been checkpointed, the next action
    /// is `Snapshot` until a snapshot has been taken since that launch
    /// (design Section 11). An integration checkpoint (one with no launch
    /// of its own) never triggers this: there is no new launch to
    /// snapshot.
    fn needs_snapshot(&self, task: &Task, work: &PlannedWork) -> bool {
        self.latest_implementer_launch(&task.id)
            .filter(|launch| launch.checkpointed)
            .is_some_and(|launch| work.snapshot_launch != Some(launch.id))
    }

    fn in_flight_action(&self, task: &Task, work: &PlannedWork) -> Option<NextAction> {
        let delivery = task.deliveries.last()?;
        if self.needs_snapshot(task, work) {
            return Some(NextAction::Snapshot {
                task: task.id.clone(),
            });
        }
        let snapshot = work.snapshot.as_ref();
        let missing = missing_criteria(delivery, snapshot);
        if !missing.is_empty() {
            return Some(NextAction::RecordProof {
                task: task.id.clone(),
                missing,
            });
        }
        if delivery.review.is_none() {
            return snapshot.cloned().map(|value| NextAction::Review {
                task: task.id.clone(),
                snapshot: value,
            });
        }
        let review = delivery.review.as_ref()?;
        self.review_action(task, delivery, review)
    }

    /// Once a review has settled, act on its computed verdict: a blocked
    /// verdict asks for the missing evidence, a changes-required verdict
    /// asks for repair, and an unresolved finding asks for disposition
    /// before anything may publish.
    fn review_action(
        &self,
        task: &Task,
        delivery: &Delivery,
        review: &crate::review::Review,
    ) -> Option<NextAction> {
        if review.verdict == Verdict::Blocked {
            return Some(NextAction::ResolveGaps {
                task: task.id.clone(),
                gaps: review.evidence_gaps.clone(),
            });
        }
        if review.verdict == Verdict::ChangesRequired {
            let findings = review
                .findings
                .iter()
                .map(|finding| finding.id.clone())
                .collect();
            return Some(NextAction::Repair {
                task: task.id.clone(),
                findings,
            });
        }
        let undisposed: Vec<FindingId> = review
            .findings
            .iter()
            .filter(|finding| !review.dispositions.contains_key(&finding.id))
            .map(|finding| finding.id.clone())
            .collect();
        if !undisposed.is_empty() {
            return Some(NextAction::Disposition {
                task: task.id.clone(),
                findings: undisposed,
            });
        }
        if let Some(action) = self.review_sync_gate(task, delivery) {
            return Some(action);
        }
        publish_action(task, delivery)
    }

    /// Once the current code delivery has a PR (draft or ready), Jira must
    /// reflect Review before any further publish step (design Section 11).
    fn review_sync_gate(&self, task: &Task, delivery: &Delivery) -> Option<NextAction> {
        let DeliveryKind::Code { pr: Some(pr) } = &delivery.kind else {
            return None;
        };
        if pr.state != PrState::Open {
            return None;
        }
        let review = self.config.as_ref()?.jira.statuses.review.clone();
        let issue = IssueKey::from(task.id.clone());
        sync_step(&task.id, &issue, &task.sync, &review)
    }

    fn merged_action(&self, task: &Task, commit: &crate::ids::Sha) -> Option<NextAction> {
        let delivery = task.deliveries.last()?;
        if matches!(delivery.kind, DeliveryKind::Verification { .. })
            && let Some(action) = verification_action(task, delivery, commit)
        {
            return Some(action);
        }
        Some(NextAction::FinalVerify {
            task: task.id.clone(),
            commit: commit.clone(),
        })
    }

    /// After `Verified`, Jira must confirm Done for the parent and every
    /// recorded subtask before `Complete` (design Section 11).
    fn verified_action(&self, task: &Task) -> Option<NextAction> {
        let done = self.config.as_ref()?.jira.statuses.done.clone();
        let issue = IssueKey::from(task.id.clone());
        if let Some(action) = sync_step(&task.id, &issue, &task.sync, &done) {
            return Some(action);
        }
        if let Some(action) = subtasks_sync_step(task, &done) {
            return Some(action);
        }
        Some(NextAction::Complete {
            task: task.id.clone(),
        })
    }

    /// Once `Completed`, nothing remains but returning the owned slot.
    fn completed_action(&self, task: &Task) -> Option<NextAction> {
        task.current_work().map(|work| NextAction::Cleanup {
            task: task.id.clone(),
            slot: work.slot.slot.clone(),
        })
    }
}

fn missing_criteria(delivery: &Delivery, snapshot: Option<&Snapshot>) -> Vec<CriterionId> {
    delivery
        .criteria
        .iter()
        .filter(|id| entry_stale_or_missing(delivery, id, snapshot))
        .cloned()
        .collect()
}

/// A criterion counts as missing both when no proof entry exists and when
/// its entry was recorded against a snapshot the current one has since
/// replaced (design Section 11: a head change invalidates stale proof).
fn entry_stale_or_missing(
    delivery: &Delivery,
    id: &CriterionId,
    snapshot: Option<&Snapshot>,
) -> bool {
    match delivery.proof.entries.get(id) {
        None => true,
        Some(entry) => snapshot.is_none_or(|snapshot| &entry.snapshot != snapshot),
    }
}

fn subtasks_sync_step(task: &Task, target: &crate::ids::JiraStatus) -> Option<NextAction> {
    task.subtasks
        .iter()
        .find_map(|(issue, subtask)| sync_step(&task.id, issue, &subtask.sync, target))
}

fn claimed_ready(task: &Task) -> NextAction {
    if task.spec.criteria.is_empty() {
        NextAction::Brief {
            task: task.id.clone(),
        }
    } else {
        NextAction::BindSlot {
            task: task.id.clone(),
        }
    }
}

fn unfinished_pr(task: &Task) -> bool {
    task.deliveries.last().is_some_and(|delivery| {
        matches!(&delivery.kind, DeliveryKind::Code { pr: Some(pr) } if pr.state != PrState::Merged)
    })
}

fn terminal(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::Verified { .. } | Phase::Completed { .. } | Phase::Excluded(_)
    )
}
