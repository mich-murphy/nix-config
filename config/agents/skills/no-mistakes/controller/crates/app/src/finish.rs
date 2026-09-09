use crate::delivery_support::{review_snapshot, verify_proof};
use crate::error::Ctx;
use crate::task_support::{completed, digest_file, task_ref};
use crate::{AgentError, App, ConflictReason, EvidenceError, Output, Rejection};
use adapters::process::Recovery;
use domain::{
    command::RecoveryTarget,
    delivery::{self, DeliveryKind, Outcome},
    event::{Event, Observation, OperationStatus},
    ids::{OperationId, Sha, TaskId},
    sync::Sync,
    task::{Phase, Receipt},
};
use std::path::PathBuf;

impl App<'_> {
    pub(super) fn final_verify(
        &mut self,
        task: TaskId,
        commit: Sha,
        evidence: PathBuf,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("final-verify", Some(&task));
        let state = self.state("final-verify")?;
        let value = task_ref(&state, &task, "final-verify", self)?;
        let delivery = value
            .deliveries
            .last()
            .ok_or_else(|| ctx.reject(ConflictReason::NoDelivery))?;
        validate_acceptance(&ctx, value, delivery)?;
        validate_final_artifact(&ctx, &evidence)?;
        let snapshot = review_snapshot(delivery)
            .ok_or_else(|| ctx.reject(EvidenceError::Other("review snapshot missing".into())))?;
        verify_proof(value, delivery, snapshot).map_err(|error| ctx.reject(error))?;
        validate_main_head(&ctx, self, delivery, &snapshot.head, &commit)?;
        let mut events = Vec::new();
        if matches!(delivery.kind, DeliveryKind::Verification { .. }) {
            events.push(Event::DeliveryClosed {
                task: task.clone(),
                delivery: delivery.id,
                outcome: Outcome::Accepted {
                    at: self.services.clock.now(),
                },
            });
        }
        events.push(Event::Verified {
            task: task.clone(),
            receipt: Receipt::Delivery { commit },
        });
        self.commit("final-verify", Some(&task), events, check)
    }

    pub(super) fn complete(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("complete")?;
        let value = task_ref(&state, &task, "complete", self)?;
        let done = state
            .config
            .as_ref()
            .is_some_and(|config| match &value.sync {
                Sync::Confirmed(receipt) => receipt.status == config.jira.statuses.done,
                _ => false,
            });
        let subtasks_done = value.subtasks.values().all(|subtask| matches!(&subtask.sync, Sync::Confirmed(receipt) if state.config.as_ref().is_some_and(|config| receipt.status == config.jira.statuses.done)));
        if !matches!(value.phase, Phase::Verified { .. }) || !done || !subtasks_done {
            return Err(self.error(
                "complete",
                Some(&task),
                ConflictReason::CompletionRequiresJiraDone,
            ));
        }
        self.commit(
            "complete",
            Some(&task),
            vec![Event::Completed { task: task.clone() }],
            check,
        )
    }

    pub(super) fn cleanup(
        &mut self,
        task: TaskId,
        delete: bool,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("cleanup", Some(&task));
        let state = self.state("cleanup")?;
        let value = task_ref(&state, &task, "cleanup", self)?;
        if !completed(value) {
            return Err(ctx.reject(ConflictReason::CleanupRequiresCompletion));
        }
        let slot = value
            .deliveries
            .iter()
            .rev()
            .find_map(|delivery| delivery.work.as_ref().map(|work| work.slot.slot.clone()))
            .ok_or_else(|| ctx.reject(ConflictReason::NoOwnedSlot))?;
        clean_slot(&ctx, self, &slot, delete, check)?;
        self.commit(
            "cleanup",
            Some(&task),
            vec![Event::SlotReleased {
                task: task.clone(),
                slot,
            }],
            check,
        )
    }

    pub(super) fn recover_operation(
        &mut self,
        target: RecoveryTarget,
        terminate: bool,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("recover-operation")?;
        match target {
            RecoveryTarget::Launch { launch } => {
                crate::recovery_support::recover_launch(self, &state, launch, terminate, check)
            }
            RecoveryTarget::Operation { operation } => {
                let task = recoverable_operation(self, &state, operation, terminate)?;
                self.commit(
                    "recover-operation",
                    Some(&task),
                    vec![Event::OperationSettled {
                        operation,
                        status: OperationStatus::Failed,
                        observation: Observation::Failure {
                            reason: "owned operation terminated".into(),
                        },
                    }],
                    check,
                )
            }
        }
    }
}

fn validate_acceptance(
    ctx: &Ctx,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
) -> Result<(), AgentError> {
    let full = task
        .spec
        .criteria
        .iter()
        .map(|criterion| criterion.id.clone())
        .collect::<Vec<_>>();
    let accepted = delivery::accepts(delivery, &full)
        || matches!(delivery.kind, DeliveryKind::Verification { .. })
            && matches!(delivery.outcome, Outcome::Open)
            && delivery::acceptance_ready(delivery, &full);
    if accepted {
        Ok(())
    } else {
        Err(ctx.reject(EvidenceError::Other(
            "latest delivery lacks full criteria and PASS".into(),
        )))
    }
}

fn validate_final_artifact(ctx: &Ctx, evidence: &std::path::Path) -> Result<(), AgentError> {
    digest_file(evidence)
        .map(|_| ())
        .map_err(|message| ctx.reject(EvidenceError::Other(message)))
}

fn validate_main_head(
    ctx: &Ctx,
    app: &App<'_>,
    delivery: &domain::delivery::Delivery,
    reviewed: &Sha,
    commit: &Sha,
) -> Result<(), AgentError> {
    let on_main = app
        .services
        .vcs
        .on_main(commit)
        .map_err(|error| ctx.reject(Rejection::External(error.0)))?;
    let identity = match (&delivery.kind, &delivery.outcome) {
        (DeliveryKind::Code { pr: Some(pr) }, Outcome::Merged { commit: merged, .. }) => {
            &pr.head == reviewed && merged == commit
        }
        (DeliveryKind::Verification { of }, Outcome::Open | Outcome::Accepted { .. }) => {
            of == reviewed && of == commit
        }
        _ => false,
    };
    if identity && on_main {
        Ok(())
    } else {
        Err(ctx.reject(EvidenceError::Other(
            "merge identity or main ancestry does not match review".into(),
        )))
    }
}

fn recoverable_operation(
    app: &App<'_>,
    state: &domain::state::State,
    operation: OperationId,
    terminate: bool,
) -> Result<TaskId, AgentError> {
    let item = state
        .operations
        .iter()
        .find(|item| item.id == operation)
        .ok_or_else(|| {
            app.error(
                "recover-operation",
                None,
                Rejection::Invalid("unknown operation".into()),
            )
        })?;
    let ctx = app.ctx("recover-operation", Some(&item.task));
    let open = state
        .tasks
        .get(&item.task)
        .and_then(|task| {
            task.deliveries
                .iter()
                .find(|delivery| delivery.id == item.delivery)
        })
        .is_some_and(|delivery| matches!(delivery.outcome, Outcome::Open));
    if !open {
        return Err(ctx.reject(ConflictReason::ClosedDeliveryImmutable));
    }
    let unsettled = matches!(
        item.status,
        OperationStatus::Running | OperationStatus::Unknown
    );
    if !unsettled {
        return Err(ctx.reject(ConflictReason::OperationAlreadySettled));
    }
    let identity = item
        .process
        .ok_or_else(|| ctx.reject(ConflictReason::OperationHasNoProcess))?;
    let recovered = app
        .services
        .process
        .recover(identity, terminate)
        .map_err(|error| ctx.reject(Rejection::External(error.0)))?;
    match recovered {
        Recovery::Stopped | Recovery::Terminated => Ok(item.task.clone()),
        Recovery::Running => Err(ctx.reject(ConflictReason::ProcessStillRunning)),
    }
}

fn clean_slot(
    ctx: &Ctx,
    app: &App<'_>,
    slot: &domain::ids::SlotId,
    delete: bool,
    check: bool,
) -> Result<(), AgentError> {
    if check {
        return Ok(());
    }
    app.services
        .vcs
        .clean_slot(slot, delete)
        .map_err(|error| ctx.reject(Rejection::External(error.0)))
}
