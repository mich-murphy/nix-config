use crate::delivery_support::{get_task, review_snapshot, verify_proof};
use crate::task_support::digest_file;
use crate::{AgentError, App, Output, Rejection};
use domain::{
    command::RecoveryTarget,
    delivery::{self, DeliveryKind, Outcome},
    event::{Event, LaunchOutcome, Observation, OperationStatus},
    ids::{OperationId, Sha, TaskId},
    sync::Sync,
    task::Phase,
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
        let state = self.state("final-verify")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("final-verify", Some(&task), Rejection::Invalid(message))
        })?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "final-verify",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        validate_acceptance(self, value, delivery, &task)?;
        validate_final_artifact(self, &task, &evidence)?;
        let snapshot = review_snapshot(delivery).ok_or_else(|| {
            self.error(
                "final-verify",
                Some(&task),
                Rejection::Evidence("review snapshot missing".into()),
            )
        })?;
        verify_proof(value, delivery, snapshot).map_err(|message| {
            self.error("final-verify", Some(&task), Rejection::Evidence(message))
        })?;
        validate_main_head(self, &task, &snapshot.head, &commit)?;
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
            commit,
        });
        self.commit("final-verify", Some(&task), events, check)
    }

    pub(super) fn complete(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("complete")?;
        let value = get_task(&state, &task)
            .map_err(|message| self.error("complete", Some(&task), Rejection::Invalid(message)))?;
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
                Rejection::Conflict("completion needs verification and confirmed Jira Done".into()),
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
        let state = self.state("cleanup")?;
        let value = get_task(&state, &task)
            .map_err(|message| self.error("cleanup", Some(&task), Rejection::Invalid(message)))?;
        if !state.completed.contains(&task) {
            return Err(self.error(
                "cleanup",
                Some(&task),
                Rejection::Conflict("cannot clean unfinished work".into()),
            ));
        }
        let slot = value
            .deliveries
            .iter()
            .rev()
            .find_map(|delivery| delivery.work.as_ref().map(|work| work.slot.slot.clone()))
            .ok_or_else(|| {
                self.error(
                    "cleanup",
                    Some(&task),
                    Rejection::Conflict("task has no owned slot".into()),
                )
            })?;
        clean_slot(self, &task, &slot, delete, check)?;
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
                let task = unsettled_launch(self, &state, launch, terminate)?;
                self.commit(
                    "recover-operation",
                    Some(&task),
                    vec![Event::LaunchEnded {
                        launch,
                        result: LaunchOutcome::Cancelled,
                        usage: None,
                    }],
                    check,
                )
            }
            RecoveryTarget::Operation { operation } => {
                let task = unsettled_operation(self, &state, operation, terminate)?;
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
    app: &App<'_>,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    id: &TaskId,
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
        Err(app.error(
            "final-verify",
            Some(id),
            Rejection::Evidence("latest delivery lacks full criteria and PASS".into()),
        ))
    }
}

fn validate_final_artifact(
    app: &App<'_>,
    task: &TaskId,
    evidence: &std::path::Path,
) -> Result<(), AgentError> {
    digest_file(evidence)
        .map(|_| ())
        .map_err(|message| app.error("final-verify", Some(task), Rejection::Evidence(message)))
}

fn validate_main_head(
    app: &App<'_>,
    task: &TaskId,
    reviewed: &Sha,
    commit: &Sha,
) -> Result<(), AgentError> {
    let on_main = app
        .services
        .vcs
        .on_main(commit)
        .map_err(|error| app.error("final-verify", Some(task), Rejection::External(error.0)))?;
    if reviewed == commit && on_main {
        Ok(())
    } else {
        Err(app.error(
            "final-verify",
            Some(task),
            Rejection::Evidence("exact reviewed head is not on main".into()),
        ))
    }
}

fn unsettled_launch(
    app: &App<'_>,
    state: &domain::state::State,
    launch: domain::ids::LaunchId,
    terminate: bool,
) -> Result<TaskId, AgentError> {
    let item = state
        .launches
        .iter()
        .find(|item| item.id == launch)
        .ok_or_else(|| {
            app.error(
                "recover-operation",
                None,
                Rejection::Invalid("unknown launch".into()),
            )
        })?;
    if item.session.is_none() && terminate {
        Ok(item.task.clone())
    } else {
        Err(app.error(
            "recover-operation",
            Some(&item.task),
            Rejection::Conflict("recovery requires an unsettled terminated launch".into()),
        ))
    }
}

fn unsettled_operation(
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
    let unsettled = matches!(
        item.status,
        OperationStatus::Running | OperationStatus::Unknown
    );
    if unsettled && terminate {
        Ok(item.task.clone())
    } else {
        Err(app.error(
            "recover-operation",
            Some(&item.task),
            Rejection::Conflict("recovery requires an unsettled terminated operation".into()),
        ))
    }
}

fn clean_slot(
    app: &App<'_>,
    task: &TaskId,
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
        .map_err(|error| app.error("cleanup", Some(task), Rejection::External(error.0)))
}
