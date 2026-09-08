use crate::delivery_support::{
    current_sync, get_task, next_operation, review_snapshot, verify_proof,
};
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    command::Transition,
    delivery::{self, DeliveryKind, Outcome},
    event::{Event, Observation, OperationStatus},
    ids::{IssueKey, JiraStatus, OperationId, Sha, TaskId},
    sync::{self, Sync},
    task::Phase,
};
use std::path::PathBuf;

impl App<'_> {
    pub(super) fn final_verify(
        &mut self,
        task: TaskId,
        commit: Sha,
        _evidence: PathBuf,
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
        if !delivery::accepts(
            delivery,
            &value
                .spec
                .criteria
                .iter()
                .map(|criterion| criterion.id.clone())
                .collect::<Vec<_>>(),
        ) {
            return Err(self.error(
                "final-verify",
                Some(&task),
                Rejection::Evidence("latest delivery lacks full criteria and PASS".into()),
            ));
        }
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
        if snapshot.head != commit
            || !self.services.vcs.on_main(&commit).map_err(|error| {
                self.error("final-verify", Some(&task), Rejection::External(error.0))
            })?
        {
            return Err(self.error(
                "final-verify",
                Some(&task),
                Rejection::Evidence("exact reviewed head is not on main".into()),
            ));
        }
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
        if !check {
            self.services
                .vcs
                .clean_slot(&slot, delete)
                .map_err(|error| {
                    self.error("cleanup", Some(&task), Rejection::External(error.0))
                })?;
        }
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

    pub(super) fn set_status(
        &mut self,
        task: TaskId,
        issue: IssueKey,
        current: JiraStatus,
        target: JiraStatus,
        transitions: Vec<Transition>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("set-status")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("set-status", Some(&task), Rejection::Invalid(message))
        })?;
        if target
            == state
                .config
                .as_ref()
                .map(|config| config.jira.statuses.done.clone())
                .unwrap_or(current.clone())
            && !matches!(value.phase, Phase::Verified { .. })
        {
            return Err(self.error(
                "set-status",
                Some(&task),
                Rejection::Conflict("Jira Done requires verified delivery".into()),
            ));
        }
        let pairs: Vec<_> = transitions
            .iter()
            .map(|entry| (entry.id.clone(), entry.to.clone()))
            .collect();
        let transition = sync::transition(&target, &pairs)
            .map_err(|error| {
                self.error(
                    "set-status",
                    Some(&task),
                    Rejection::Invalid(format!("transition rejected: {error:?}")),
                )
            })?
            .clone();
        let attempts = current_sync(value, &issue)
            .map(|sync| match sync {
                Sync::Unknown(intent) => intent.attempts.saturating_add(1),
                Sync::Failed(_) => 2,
                _ => 1,
            })
            .unwrap_or(1);
        let operation = next_operation(&state);
        let events = vec![Event::StatusIntended {
            task: task.clone(),
            issue,
            current,
            target,
            transition: transition.clone(),
            operation,
            attempts,
            at: self.services.clock.now(),
        }];
        let mut output = self.commit("set-status", Some(&task), events, check)?;
        output.result = ResultData::Transition { transition };
        Ok(output)
    }

    pub(super) fn observe_status(
        &mut self,
        task: TaskId,
        issue: IssueKey,
        status: JiraStatus,
        evidence: String,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("observe-status")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("observe-status", Some(&task), Rejection::Invalid(message))
        })?;
        if evidence.trim().is_empty() {
            return Err(self.error(
                "observe-status",
                Some(&task),
                Rejection::Invalid("fresh observation evidence is required".into()),
            ));
        }
        let current = current_sync(value, &issue).ok_or_else(|| {
            self.error(
                "observe-status",
                Some(&task),
                Rejection::Invalid("unrecorded subtask".into()),
            )
        })?;
        let observed =
            sync::observe(current, status.clone(), self.services.clock.now()).map_err(|error| {
                self.error(
                    "observe-status",
                    Some(&task),
                    Rejection::Conflict(format!("observation rejected: {error:?}")),
                )
            })?;
        self.commit(
            "observe-status",
            Some(&task),
            vec![Event::StatusObserved {
                task: task.clone(),
                issue,
                read: status,
                sync: observed,
            }],
            check,
        )
    }

    pub(super) fn recover_operation(
        &mut self,
        operation: OperationId,
        terminate: bool,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("recover-operation")?;
        let item = state
            .operations
            .iter()
            .find(|item| item.id == operation)
            .ok_or_else(|| {
                self.error(
                    "recover-operation",
                    None,
                    Rejection::Invalid("unknown operation".into()),
                )
            })?;
        if !matches!(
            item.status,
            OperationStatus::Running | OperationStatus::Unknown
        ) || !terminate
        {
            return Err(self.error(
                "recover-operation",
                Some(&item.task),
                Rejection::Conflict("recovery requires an unsettled terminated operation".into()),
            ));
        }
        let task = item.task.clone();
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
