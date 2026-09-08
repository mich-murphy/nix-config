use crate::delivery::PublishInput;
use crate::delivery_support::{
    current_snapshot, execute_publish, get_task, next_operation, publish_action, publish_ready,
    validate_closed,
};
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    delivery::{self, CheckState, DeliveryKind, Outcome, PrState, Replacement},
    event::{Event, Observation, Operation, OperationStatus},
    ids::{PrNumber, TaskId},
    task::HoldReason,
};

impl App<'_> {
    pub(super) fn publish(
        &mut self,
        task: TaskId,
        input: PublishInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("publish")?;
        let value = get_task(&state, &task)
            .map_err(|message| self.error("publish", Some(&task), Rejection::Invalid(message)))?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "publish",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        let snapshot = current_snapshot(value).ok_or_else(|| {
            self.error(
                "publish",
                Some(&task),
                Rejection::Evidence("publish requires a snapshot".into()),
            )
        })?;
        publish_ready(&state, value, delivery, &input)
            .map_err(|message| self.error("publish", Some(&task), Rejection::Evidence(message)))?;
        let id = next_operation(&state);
        let action = publish_action(delivery, snapshot, &input)
            .map_err(|message| self.error("publish", Some(&task), Rejection::Invalid(message)))?;
        let operation = Operation {
            id,
            task: task.clone(),
            delivery: delivery.id,
            action,
            status: OperationStatus::Running,
        };
        if check {
            return self.commit(
                "publish",
                Some(&task),
                vec![Event::OperationStarted { operation }],
                true,
            );
        }
        let mut records = self.write(
            "publish",
            Some(&task),
            vec![Event::OperationStarted { operation }],
            false,
        )?;
        let observed = execute_publish(self, delivery, snapshot, &input)
            .map_err(|message| self.error("publish", Some(&task), Rejection::External(message)))?;
        let mut events = vec![
            Event::OperationSettled {
                operation: id,
                status: OperationStatus::Confirmed,
                observation: Observation::PullRequest(observed.clone()),
            },
            Event::PrObserved {
                task: task.clone(),
                delivery: delivery.id,
                pr: observed.clone(),
            },
        ];
        if observed.state == PrState::Merged {
            let merge = observed.merge.clone().ok_or_else(|| {
                self.error(
                    "publish",
                    Some(&task),
                    Rejection::External("merged PR omitted merge commit".into()),
                )
            })?;
            events.push(Event::DeliveryClosed {
                task: task.clone(),
                delivery: delivery.id,
                outcome: Outcome::Merged {
                    commit: merge,
                    at: self.services.clock.now(),
                },
            });
        }
        records.extend(self.write("publish", Some(&task), events, false)?);
        Ok(Output {
            events: records,
            result: ResultData::Applied,
        })
    }

    pub(super) fn observe_pr(
        &mut self,
        task: TaskId,
        pr: PrNumber,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("observe-pr")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("observe-pr", Some(&task), Rejection::Invalid(message))
        })?;
        let known = value.deliveries.iter().find(|delivery| matches!(&delivery.kind, DeliveryKind::Code { pr: Some(known) } if known.number == pr));
        let current = value.deliveries.last().ok_or_else(|| {
            self.error(
                "observe-pr",
                Some(&task),
                Rejection::Invalid("unknown PR".into()),
            )
        })?;
        let replacement = known.is_none()
            && matches!(
                value.hold.as_ref().map(|hold| &hold.reason),
                Some(HoldReason::SupersededPr { .. })
            );
        if known.is_none() && !replacement {
            return Err(self.error(
                "observe-pr",
                Some(&task),
                Rejection::Invalid("unknown PR".into()),
            ));
        }
        let delivery = known.unwrap_or(current);
        let observation =
            self.services.github.observe(pr).map_err(|error| {
                self.error("observe-pr", Some(&task), Rejection::External(error.0))
            })?;
        if replacement {
            return self.observe_replacement(task, current, observation, check);
        }
        if delivery::closed(delivery) {
            validate_closed(delivery, &observation).map_err(|message| {
                self.error("observe-pr", Some(&task), Rejection::Conflict(message))
            })?;
            return Ok(Output {
                events: Vec::new(),
                result: ResultData::Valid,
            });
        }
        let mut events = vec![Event::PrObserved {
            task: task.clone(),
            delivery: delivery.id,
            pr: observation.clone(),
        }];
        if observation.state == PrState::Merged {
            let merge = observation.merge.clone().ok_or_else(|| {
                self.error(
                    "observe-pr",
                    Some(&task),
                    Rejection::External("merge commit missing".into()),
                )
            })?;
            events.push(Event::DeliveryClosed {
                task: task.clone(),
                delivery: delivery.id,
                outcome: Outcome::Merged {
                    commit: merge,
                    at: self.services.clock.now(),
                },
            });
        } else if observation.state == PrState::Closed {
            events.push(Event::Held {
                task: task.clone(),
                reason: HoldReason::SupersededPr {
                    pr,
                    replacement: None,
                },
                at: self.services.clock.now(),
            });
        }
        self.commit("observe-pr", Some(&task), events, check)
    }

    fn observe_replacement(
        &mut self,
        task: TaskId,
        delivery: &domain::delivery::Delivery,
        observation: domain::delivery::PullRequest,
        check: bool,
    ) -> Result<Output, AgentError> {
        if observation.state != PrState::Merged
            || observation.merge.is_none()
            || !self
                .services
                .vcs
                .on_main(observation.merge.as_ref().ok_or_else(|| {
                    self.error(
                        "observe-pr",
                        Some(&task),
                        Rejection::External("replacement merge is missing".into()),
                    )
                })?)
                .map_err(|error| {
                    self.error("observe-pr", Some(&task), Rejection::External(error.0))
                })?
        {
            return Err(self.error(
                "observe-pr",
                Some(&task),
                Rejection::Conflict("replacement is not merged on main".into()),
            ));
        }
        let replacement = Replacement {
            pr: observation.number,
            head: observation.head,
            merge: observation.merge.ok_or_else(|| {
                self.error(
                    "observe-pr",
                    Some(&task),
                    Rejection::External("replacement merge is missing".into()),
                )
            })?,
        };
        self.commit(
            "observe-pr",
            Some(&task),
            vec![
                Event::DeliveryClosed {
                    task: task.clone(),
                    delivery: delivery.id,
                    outcome: Outcome::Replaced {
                        by: replacement,
                        at: self.services.clock.now(),
                    },
                },
                Event::Held {
                    task: task.clone(),
                    reason: HoldReason::NeedsHuman {
                        diagnosis: "closed PR was replaced externally".into(),
                        remaining: delivery.criteria.clone(),
                    },
                    at: self.services.clock.now(),
                },
            ],
            check,
        )
    }

    pub(super) fn poll_checks(
        &mut self,
        task: TaskId,
        wait: bool,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("poll-checks")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("poll-checks", Some(&task), Rejection::Invalid(message))
        })?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "poll-checks",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        let pr = match &delivery.kind {
            DeliveryKind::Code { pr: Some(pr) } => pr,
            _ => {
                return Err(self.error(
                    "poll-checks",
                    Some(&task),
                    Rejection::Conflict("task has no PR".into()),
                ));
            }
        };
        let observation = self.services.github.observe(pr.number).map_err(|error| {
            self.error("poll-checks", Some(&task), Rejection::External(error.0))
        })?;
        let mut events = vec![Event::PrObserved {
            task: task.clone(),
            delivery: delivery.id,
            pr: observation.clone(),
        }];
        let pending = observation
            .checks
            .iter()
            .any(|item| item.required && item.state == CheckState::Pending);
        if pending && wait {
            let deadline = value
                .hold
                .as_ref()
                .and_then(|hold| match hold.reason {
                    HoldReason::CiPending { deadline, .. } => Some(deadline),
                    _ => None,
                })
                .unwrap_or_else(|| self.services.clock.now().saturating_add(1800));
            events.push(Event::Held {
                task: task.clone(),
                reason: HoldReason::CiPending {
                    pr: pr.number,
                    head: pr.head.clone(),
                    deadline,
                },
                at: self.services.clock.now(),
            });
        }
        self.commit("poll-checks", Some(&task), events, check)
    }
}
