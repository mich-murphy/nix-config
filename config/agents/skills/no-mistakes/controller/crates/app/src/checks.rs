use crate::delivery_support::{append_acceptance_hold, validate_closed};
use crate::task_support::task_ref;
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    Instant,
    delivery::{self, CheckState, DeliveryKind, Outcome, PrState, PullRequest, Replacement},
    event::Event,
    ids::{DeliveryId, PrNumber, TaskId},
    task::{HoldReason, Task},
};

impl App<'_> {
    pub(super) fn observe_pr(
        &mut self,
        task: TaskId,
        pr: PrNumber,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("observe-pr")?;
        let value = task_ref(&state, &task, "observe-pr", self)?;
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

    /// Without `--wait`, observes once and returns. With `--wait`, polls
    /// GitHub every 30 seconds until no required check is pending or the
    /// per-head deadline passes (first observed pending, plus 1800
    /// seconds, persisted in `check_deadlines` so it survives a hold and
    /// resume); on deadline it records the `CiPending` hold.
    pub(super) fn poll_checks(
        &mut self,
        task: TaskId,
        wait: bool,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("poll-checks")?;
        let value = task_ref(&state, &task, "poll-checks", self)?;
        let (pr_number, delivery_id, mut deadline) = poll_target(value).map_err(|message| {
            self.error("poll-checks", Some(&task), Rejection::Conflict(message))
        })?;
        let mut observation = observe_checks(self, &task, pr_number)?;
        let mut pending = checks_pending(&observation);
        if wait && !check {
            (observation, pending) =
                self.wait_for_checks(&task, pr_number, &mut deadline, observation)?;
        }
        let events = poll_events(
            self,
            &task,
            PollOutcome {
                delivery: delivery_id,
                pr: pr_number,
                observation,
                pending,
                wait,
                deadline,
            },
        );
        self.commit("poll-checks", Some(&task), events, check)
    }

    fn wait_for_checks(
        &self,
        task: &TaskId,
        pr: PrNumber,
        deadline: &mut Option<domain::Instant>,
        mut observation: domain::delivery::PullRequest,
    ) -> Result<(domain::delivery::PullRequest, bool), AgentError> {
        let mut pending = checks_pending(&observation);
        while pending && !self.check_deadline_passed(deadline) {
            self.services.clock.sleep(30);
            observation = observe_checks(self, task, pr)?;
            pending = checks_pending(&observation);
        }
        Ok((observation, pending))
    }

    fn check_deadline_passed(&self, deadline: &mut Option<domain::Instant>) -> bool {
        let bound = *deadline.get_or_insert_with(|| self.services.clock.now().saturating_add(1800));
        self.services.clock.now() >= bound
    }
}

/// The PR, delivery and persisted per-head deadline `poll_checks` polls
/// against, or the conflict message to reject with.
fn poll_target(value: &Task) -> Result<(PrNumber, DeliveryId, Option<Instant>), String> {
    let delivery = value
        .deliveries
        .last()
        .ok_or_else(|| "task has no delivery".to_string())?;
    let pr = match &delivery.kind {
        DeliveryKind::Code { pr: Some(pr) } => pr,
        _ => return Err("task has no PR".into()),
    };
    let deadline = delivery.check_deadlines.get(&pr.head).copied();
    Ok((pr.number, delivery.id, deadline))
}

/// Result of observing (and possibly waiting on) a PR's checks, gathered
/// for `poll_events` to turn into committed events.
struct PollOutcome {
    delivery: DeliveryId,
    pr: PrNumber,
    observation: PullRequest,
    pending: bool,
    wait: bool,
    deadline: Option<Instant>,
}

/// The events `poll_checks` commits: the observation itself, plus a
/// `CiPending` hold once waiting has exhausted the per-head deadline with a
/// required check still pending.
fn poll_events(app: &App<'_>, task: &TaskId, outcome: PollOutcome) -> Vec<Event> {
    let mut events = vec![Event::PrObserved {
        task: task.clone(),
        delivery: outcome.delivery,
        pr: outcome.observation.clone(),
    }];
    if outcome.pending && outcome.wait {
        let bound = outcome
            .deadline
            .unwrap_or_else(|| app.services.clock.now().saturating_add(1800));
        events.push(Event::Held {
            task: task.clone(),
            reason: HoldReason::CiPending {
                pr: outcome.pr,
                head: outcome.observation.head,
                deadline: bound,
            },
            at: app.services.clock.now(),
        });
    }
    events
}

fn observe_checks(
    app: &App<'_>,
    task: &TaskId,
    pr: PrNumber,
) -> Result<domain::delivery::PullRequest, AgentError> {
    app.services
        .github
        .observe(pr)
        .map_err(|error| app.error("poll-checks", Some(task), Rejection::External(error.0)))
}

fn checks_pending(observation: &domain::delivery::PullRequest) -> bool {
    observation
        .checks
        .iter()
        .any(|item| item.required && item.state == CheckState::Pending)
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
            reason: HoldReason::SupersededPr { pr },
            at: app.services.clock.now(),
        });
    }
    Ok(events)
}

pub(super) fn merged_event(
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
