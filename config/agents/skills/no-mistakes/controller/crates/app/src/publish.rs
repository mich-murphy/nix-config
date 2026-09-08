use crate::delivery::PublishInput;
use crate::delivery_support::{
    append_acceptance_hold, current_snapshot, execute_publish, get_task, next_operation,
    publish_action, publish_ready, validate_closed,
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
        let operation = publish_operation(self, &state, &task, delivery, snapshot, &input)?;
        self.execute_publish_flow(value, delivery, snapshot, &input, operation, check)
    }

    fn execute_publish_flow(
        &mut self,
        task: &domain::task::Task,
        delivery: &domain::delivery::Delivery,
        snapshot: &domain::acceptance::Snapshot,
        input: &PublishInput,
        operation: Operation,
        check: bool,
    ) -> Result<Output, AgentError> {
        if check {
            return self.commit(
                "publish",
                Some(&task.id),
                vec![Event::OperationStarted { operation }],
                true,
            );
        }
        let id = operation.id;
        let mut records = self.write(
            "publish",
            Some(&task.id),
            vec![Event::OperationStarted { operation }],
            false,
        )?;
        let observed = execute_publish(self, delivery, snapshot, input).map_err(|message| {
            self.error("publish", Some(&task.id), Rejection::External(message))
        })?;
        let events = publish_events(self, task, delivery, id, observed)?;
        records.extend(self.write("publish", Some(&task.id), events, false)?);
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
        let (delivery, current, replacement) = observed_delivery(self, value, &task, pr)?;
        let observation =
            self.services.github.observe(pr).map_err(|error| {
                self.error("observe-pr", Some(&task), Rejection::External(error.0))
            })?;
        if replacement {
            return self.observe_replacement(task, current, observation, check);
        }
        if delivery::closed(delivery) {
            return validate_closed_output(self, &task, delivery, &observation);
        }
        let events = observation_events(self, value, delivery, pr, observation)?;
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
            let deadline = state
                .check_deadlines
                .get(&format!("{}:{}", task, pr.head))
                .copied()
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

fn publish_operation(
    app: &App<'_>,
    state: &domain::state::State,
    task: &TaskId,
    delivery: &domain::delivery::Delivery,
    snapshot: &domain::acceptance::Snapshot,
    input: &PublishInput,
) -> Result<Operation, AgentError> {
    let action = publish_action(delivery, snapshot, input)
        .map_err(|message| app.error("publish", Some(task), Rejection::Invalid(message)))?;
    Ok(Operation {
        id: next_operation(state),
        task: task.clone(),
        delivery: delivery.id,
        action,
        status: OperationStatus::Running,
        process: None,
        timeout_seconds: 300,
    })
}

fn publish_events(
    app: &App<'_>,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    operation: domain::ids::OperationId,
    observed: domain::delivery::PullRequest,
) -> Result<Vec<Event>, AgentError> {
    let mut events = vec![
        Event::OperationSettled {
            operation,
            status: OperationStatus::Confirmed,
            observation: Observation::PullRequest(observed.clone()),
        },
        Event::PrObserved {
            task: task.id.clone(),
            delivery: delivery.id,
            pr: observed.clone(),
        },
    ];
    if let Some(event) = merged_event(app, &task.id, delivery.id, &observed, "publish")? {
        events.push(event);
        append_acceptance_hold(app, task, delivery, &mut events);
    }
    Ok(events)
}

fn observed_delivery<'a>(
    app: &App<'_>,
    value: &'a domain::task::Task,
    task: &TaskId,
    pr: PrNumber,
) -> Result<
    (
        &'a domain::delivery::Delivery,
        &'a domain::delivery::Delivery,
        bool,
    ),
    AgentError,
> {
    let known = value.deliveries.iter().find(|delivery| {
        matches!(&delivery.kind, DeliveryKind::Code { pr: Some(known) } if known.number == pr)
    });
    let current = value.deliveries.last().ok_or_else(|| {
        app.error(
            "observe-pr",
            Some(task),
            Rejection::Invalid("unknown PR".into()),
        )
    })?;
    let replacement = known.is_none()
        && matches!(
            value.hold.as_ref().map(|hold| &hold.reason),
            Some(HoldReason::SupersededPr { .. })
        );
    known
        .or(replacement.then_some(current))
        .map(|delivery| (delivery, current, replacement))
        .ok_or_else(|| {
            app.error(
                "observe-pr",
                Some(task),
                Rejection::Invalid("unknown PR".into()),
            )
        })
}

fn observation_events(
    app: &App<'_>,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    pr: PrNumber,
    observation: domain::delivery::PullRequest,
) -> Result<Vec<Event>, AgentError> {
    let mut events = vec![Event::PrObserved {
        task: task.id.clone(),
        delivery: delivery.id,
        pr: observation.clone(),
    }];
    if let Some(event) = merged_event(app, &task.id, delivery.id, &observation, "observe-pr")? {
        events.push(event);
        append_acceptance_hold(app, task, delivery, &mut events);
    } else if observation.state == PrState::Closed {
        events.push(Event::Held {
            task: task.id.clone(),
            reason: HoldReason::SupersededPr {
                pr,
                replacement: None,
            },
            at: app.services.clock.now(),
        });
    }
    Ok(events)
}

fn merged_event(
    app: &App<'_>,
    task: &TaskId,
    delivery: domain::ids::DeliveryId,
    pr: &domain::delivery::PullRequest,
    command: &str,
) -> Result<Option<Event>, AgentError> {
    if pr.state != PrState::Merged {
        return Ok(None);
    }
    let commit = pr.merge.clone().ok_or_else(|| {
        app.error(
            command,
            Some(task),
            Rejection::External("merged PR omitted merge commit".into()),
        )
    })?;
    Ok(Some(Event::DeliveryClosed {
        task: task.clone(),
        delivery,
        outcome: Outcome::Merged {
            commit,
            at: app.services.clock.now(),
        },
    }))
}

fn validate_closed_output(
    app: &App<'_>,
    task: &TaskId,
    delivery: &domain::delivery::Delivery,
    observed: &domain::delivery::PullRequest,
) -> Result<Output, AgentError> {
    validate_closed(delivery, observed)
        .map_err(|message| app.error("observe-pr", Some(task), Rejection::Conflict(message)))?;
    Ok(Output {
        events: Vec::new(),
        result: ResultData::Valid,
    })
}
