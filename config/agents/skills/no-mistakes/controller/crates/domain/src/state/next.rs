mod format;

use self::format::{command_for, template_for};
use super::State;
use crate::{
    acceptance::Snapshot,
    budget::{self, BudgetKind},
    command::{ActionEnvelope, NextAction, PublishStep},
    delivery::{DeliveryKind, PrState},
    event::OperationStatus,
    ids::{FindingId, IssueKey, TaskId},
    review::Verdict,
    sync::Sync,
    task::{Phase, Task, WorkStage},
};

impl State {
    #[must_use]
    pub fn next(&self) -> Option<ActionEnvelope> {
        let action = self.choose_action()?;
        let (command, schema) = command_for(&action);
        let template = template_for(&action);
        Some(ActionEnvelope {
            action,
            command,
            template,
            schema,
        })
    }

    fn choose_action(&self) -> Option<NextAction> {
        self.config.as_ref()?;
        if !self.questions.is_empty() {
            return Some(NextAction::AnswerQuestions {
                questions: self.questions.clone(),
            });
        }
        if let Some(launch) = self.launches.iter().find(|launch| launch.session.is_none()) {
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
        self.active
            .as_ref()
            .map_or_else(|| self.queue_action(), |task| self.active_action(task))
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

    fn pending_checkpoint(&self, task: &TaskId) -> Option<crate::ids::LaunchId> {
        let latest = self.launches.iter().rev().find(|launch| {
            launch.task == *task
                && launch.role == crate::command::AgentRole::Implementer
                && launch.session.is_some()
        })?;
        (self.checkpoints.get(task) != Some(&latest.id)).then_some(latest.id)
    }

    fn queue_action(&self) -> Option<NextAction> {
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
        if let Some(hold) = &task.hold {
            return Some(hold_action(task, &hold.reason));
        }
        match &task.phase {
            Phase::Queued | Phase::Blocked(_) | Phase::NeedsInput(_) | Phase::Excluded(_) => None,
            Phase::Claimed => Some(self.claimed_action(task)),
            Phase::Planned { work } => Some(planned_action(task, work)),
            Phase::InFlight { work, stage } => {
                self.in_flight_action(task, work.snapshot.as_ref(), stage)
            }
            Phase::Merged { commit, .. } => self.merged_action(task, commit),
            Phase::Verified { .. } => self.verified_action(task),
        }
    }

    fn claimed_action(&self, task: &Task) -> NextAction {
        let progress = self
            .config
            .as_ref()
            .map(|config| config.jira.statuses.progress.clone());
        match (&task.sync, progress) {
            (Sync::Unknown(_), _) => NextAction::ObserveStatus {
                task: task.id.clone(),
                issue: IssueKey::from(task.id.clone()),
            },
            (Sync::Confirmed(receipt), Some(target)) if receipt.status == target => {
                claimed_ready(task)
            }
            (_, Some(target)) => NextAction::SyncStatus {
                task: task.id.clone(),
                target,
            },
            (_, None) => NextAction::Brief {
                task: task.id.clone(),
            },
        }
    }

    fn in_flight_action(
        &self,
        task: &Task,
        snapshot: Option<&Snapshot>,
        stage: &WorkStage,
    ) -> Option<NextAction> {
        let delivery = task.deliveries.last()?;
        if matches!(stage, WorkStage::Building) {
            return Some(NextAction::Implement {
                task: task.id.clone(),
                remaining_turns: budget::remaining(
                    BudgetKind::Implementation,
                    &task.budgets,
                    task.tier.current,
                    &task.authorities,
                ),
            });
        }
        if delivery.proof.entries.len() < delivery.criteria.len() {
            let missing = delivery
                .criteria
                .iter()
                .filter(|id| !delivery.proof.entries.contains_key(*id))
                .cloned()
                .collect();
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
        publish_action(task, delivery)
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

    fn verified_action(&self, task: &Task) -> Option<NextAction> {
        if !self.completed.contains(&task.id) {
            return Some(NextAction::Complete {
                task: task.id.clone(),
            });
        }
        task.deliveries
            .last()
            .and_then(|delivery| delivery.work.as_ref())
            .map(|work| NextAction::Cleanup {
                task: task.id.clone(),
                slot: work.slot.slot.clone(),
            })
    }
}

fn planned_action(task: &Task, work: &crate::task::PlannedWork) -> NextAction {
    if work.plan.is_none() {
        NextAction::Plan {
            task: task.id.clone(),
        }
    } else {
        NextAction::Implement {
            task: task.id.clone(),
            remaining_turns: budget::remaining(
                BudgetKind::Implementation,
                &task.budgets,
                task.tier.current,
                &task.authorities,
            ),
        }
    }
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

fn verification_action(
    task: &Task,
    delivery: &crate::delivery::Delivery,
    commit: &crate::ids::Sha,
) -> Option<NextAction> {
    let missing = delivery
        .criteria
        .iter()
        .filter(|id| !delivery.proof.entries.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(NextAction::RecordProof {
            task: task.id.clone(),
            missing,
        });
    }
    delivery.review.is_none().then(|| NextAction::Review {
        task: task.id.clone(),
        snapshot: Snapshot {
            base: commit.clone(),
            head: commit.clone(),
            requirements: task.spec.requirements.clone(),
        },
    })
}

fn publish_action(task: &Task, delivery: &crate::delivery::Delivery) -> Option<NextAction> {
    match &delivery.kind {
        DeliveryKind::Verification { .. } => Some(NextAction::FinalVerify {
            task: task.id.clone(),
            commit: delivery.base.clone(),
        }),
        DeliveryKind::Code { pr: None } => Some(NextAction::Publish {
            task: task.id.clone(),
            step: PublishStep::Create,
        }),
        DeliveryKind::Code { pr: Some(pr) } if pr.state == PrState::Open && pr.draft => {
            Some(NextAction::Publish {
                task: task.id.clone(),
                step: PublishStep::Ready,
            })
        }
        DeliveryKind::Code { pr: Some(pr) } if pr.state == PrState::Open => {
            let pending = pr
                .checks
                .iter()
                .any(|check| check.required && check.state == crate::delivery::CheckState::Pending);
            if pending {
                Some(NextAction::AwaitChecks {
                    task: task.id.clone(),
                    pr: pr.number,
                    deadline: 0,
                })
            } else {
                Some(NextAction::Publish {
                    task: task.id.clone(),
                    step: PublishStep::Merge,
                })
            }
        }
        DeliveryKind::Code { pr: Some(_) } => None,
    }
}

fn hold_action(task: &Task, reason: &crate::task::HoldReason) -> NextAction {
    match reason {
        crate::task::HoldReason::CiPending { pr, deadline, .. } => NextAction::AwaitChecks {
            task: task.id.clone(),
            pr: *pr,
            deadline: *deadline,
        },
        crate::task::HoldReason::NeedsHuman { remaining, .. } => NextAction::OpenDelivery {
            task: task.id.clone(),
            remaining: remaining.clone(),
        },
        crate::task::HoldReason::BudgetExhausted(_)
        | crate::task::HoldReason::SupersededPr { .. } => NextAction::Hold {
            task: task.id.clone(),
            reason: reason.clone(),
        },
    }
}

fn unfinished_pr(task: &Task) -> bool {
    task.deliveries.last().is_some_and(|delivery| {
        matches!(&delivery.kind, DeliveryKind::Code { pr: Some(pr) } if pr.state != PrState::Merged)
    })
}

fn terminal(phase: &Phase) -> bool {
    matches!(phase, Phase::Verified { .. } | Phase::Excluded(_))
}
