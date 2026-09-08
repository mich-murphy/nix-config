use crate::delivery_support::verify_proof;
use crate::task_support::{
    assignment, guard_role, next_launch, pair_for, previous_session, review_target, task_ref,
};
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    authority, budget,
    command::{AgentRole, Fallback},
    event::{Event, Launch, LaunchOutcome},
    ids::{DeliveryId, TaskId},
    ports::LaunchRequest,
    review::{self, Review},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl App<'_> {
    pub(super) fn run_agent(
        &mut self,
        task: TaskId,
        role: AgentRole,
        prompt: PathBuf,
        fallback: Option<Fallback>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let prepared = prepare_launch(self, &task, role, &prompt, fallback.as_ref())?;
        if check {
            return self.commit("run-agent", Some(&task), prepared.events, true);
        }
        if role == AgentRole::Implementer {
            return self.run_implementer(task, prepared.events, prepared.request);
        }
        self.run_reviewer(
            task,
            prepared.delivery,
            prepared.events,
            prepared.request,
            &prepared.task,
        )
    }

    fn run_implementer(
        &mut self,
        task: TaskId,
        events: Vec<Event>,
        request: LaunchRequest,
    ) -> Result<Output, AgentError> {
        let mut events = events;
        let (mut records, result) =
            crate::agent_support::invoke(self, &task, &request, &mut events)?;
        events.push(Event::LaunchEnded {
            launch: request.id,
            result: LaunchOutcome::Completed {
                session: result.session,
                output: result.output.clone(),
            },
            usage: result.tokens,
        });
        records.extend(self.write("run-agent", Some(&task), events, false)?);
        Ok(Output {
            events: records,
            result: ResultData::Launch {
                launch: request.id,
                output: result.output,
            },
        })
    }

    fn run_reviewer(
        &mut self,
        task: TaskId,
        delivery: DeliveryId,
        mut events: Vec<Event>,
        request: LaunchRequest,
        value: &domain::task::Task,
    ) -> Result<Output, AgentError> {
        let (mut records, result) =
            crate::agent_support::invoke(self, &task, &request, &mut events)?;
        let mut review: Review = serde_json::from_str(&result.output).map_err(|error| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Invalid(format!("malformed review: {error}")),
            )
        })?;
        let snapshot = review_target(
            value,
            value.deliveries.last().ok_or_else(|| {
                self.error(
                    "run-agent",
                    Some(&task),
                    Rejection::Conflict("task has no delivery".into()),
                )
            })?,
        )
        .ok_or_else(|| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Evidence("review requires a snapshot".into()),
            )
        })?;
        if review.launch != request.id || review.session != result.session {
            return Err(self.error(
                "run-agent",
                Some(&task),
                Rejection::Invalid("review launch or session does not match the harness".into()),
            ));
        }
        review.verdict =
            review::verdict(&review.findings, &review.evidence_gaps, value.tier.current);
        let implementer =
            previous_session_from_state(&self.state("run-agent")?, &task, AgentRole::Implementer);
        review::validate(&review, &snapshot, implementer.as_deref()).map_err(|error| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Evidence(format!("invalid review: {error:?}")),
            )
        })?;
        events.push(Event::LaunchEnded {
            launch: request.id,
            result: LaunchOutcome::Completed {
                session: result.session.clone(),
                output: result.output.clone(),
            },
            usage: result.tokens,
        });
        events.push(Event::ReviewSettled {
            task: task.clone(),
            delivery,
            review: Box::new(review),
        });
        records.extend(self.write("run-agent", Some(&task), events, false)?);
        Ok(Output {
            events: records,
            result: ResultData::Launch {
                launch: request.id,
                output: result.output,
            },
        })
    }
}

struct PreparedLaunch {
    task: domain::task::Task,
    delivery: DeliveryId,
    events: Vec<Event>,
    request: LaunchRequest,
}

fn prepare_launch(
    app: &App<'_>,
    id: &TaskId,
    role: AgentRole,
    prompt: &Path,
    fallback: Option<&Fallback>,
) -> Result<PreparedLaunch, AgentError> {
    let state = app.state("run-agent")?;
    let task = task_ref(&state, id, "run-agent", app)?.clone();
    validate_launch(app, &state, &task, id, role)?;
    let delivery = task.deliveries.last().ok_or_else(|| {
        app.error(
            "run-agent",
            Some(id),
            Rejection::Conflict("task has no delivery".into()),
        )
    })?;
    let delivery_id = delivery.id;
    let (assignment, budget_kind, counted) = launch_budget(app, &state, &task, id, role, fallback)?;
    let launch = next_launch(&state);
    let events = launch_events(
        &task,
        id,
        delivery_id,
        role,
        prompt,
        launch,
        budget_kind,
        counted,
    );
    let prompt_text = fs::read_to_string(prompt)
        .map_err(|error| app.error("run-agent", Some(id), Rejection::Invalid(error.to_string())))?;
    let prompt_text = crate::task_support::launch_prompt(&task, prompt_text);
    let request = LaunchRequest {
        id: launch,
        assignment,
        prompt: prompt_text,
        cwd: state
            .config
            .as_ref()
            .map(|config| config.repo.clone())
            .unwrap_or_default(),
        session: previous_session(&state, id, role),
        reviewer: role != AgentRole::Implementer,
    };
    Ok(PreparedLaunch {
        task,
        delivery: delivery_id,
        events,
        request,
    })
}

fn validate_launch(
    app: &App<'_>,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
    role: AgentRole,
) -> Result<(), AgentError> {
    if state.launches.iter().any(|launch| launch.session.is_none()) {
        return Err(app.error(
            "run-agent",
            Some(id),
            Rejection::Conflict("settle the active launch first".into()),
        ));
    }
    require_checkpoint(state, id)?;
    let delivery = task.deliveries.last().ok_or_else(|| {
        app.error(
            "run-agent",
            Some(id),
            Rejection::Conflict("task has no delivery".into()),
        )
    })?;
    guard_role(task, delivery, role)
        .map_err(|message| app.error("run-agent", Some(id), Rejection::Conflict(message)))?;
    require_prior_review(app, state, id, role)?;
    ensure_readiness(app, task, delivery, id, role)
}

fn launch_budget(
    app: &App<'_>,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
    role: AgentRole,
    fallback: Option<&Fallback>,
) -> Result<(domain::risk::Assignment, budget::BudgetKind, bool), AgentError> {
    let (assignment, base) = assignment(state, task.tier.current, role, fallback)
        .map_err(|message| app.error("run-agent", Some(id), Rejection::Invalid(message)))?;
    let repaired = role == AgentRole::Implementer
        && state
            .launches
            .iter()
            .any(|launch| launch.task == *id && launch.role == AgentRole::Reviewer);
    let kind = if repaired {
        budget::BudgetKind::Repair
    } else {
        base
    };
    let counted = pair_for(task, role).is_none_or(|authority| !authority::scoped(authority));
    if counted && !launch_capacity(task, role, kind) {
        return Err(app.error(
            "run-agent",
            Some(id),
            Rejection::Budget(format!("{kind:?} budget exhausted")),
        ));
    }
    Ok((assignment, kind, counted))
}

#[allow(clippy::too_many_arguments)]
fn launch_events(
    task: &domain::task::Task,
    id: &TaskId,
    delivery: DeliveryId,
    role: AgentRole,
    prompt: &Path,
    launch: domain::ids::LaunchId,
    budget: budget::BudgetKind,
    counted: bool,
) -> Vec<Event> {
    let mut events = vec![
        Event::LaunchStarted {
            launch: Launch {
                id: launch,
                task: id.clone(),
                delivery,
                role,
                prompt: prompt.to_path_buf(),
                session: None,
                counted,
                process: None,
            },
        },
        Event::BudgetSpent {
            task: id.clone(),
            budget,
            counted,
        },
    ];
    if role == AgentRole::Escalation {
        events.push(Event::BudgetSpent {
            task: id.clone(),
            budget: budget::BudgetKind::Review,
            counted,
        });
    }
    if let Some(authority) = pair_for(task, role) {
        events.push(Event::PairSpent {
            task: id.clone(),
            authority: authority.id,
            launch,
            implementation: role == AgentRole::Implementer,
        });
    }
    events
}

fn require_prior_review(
    app: &App<'_>,
    state: &domain::state::State,
    task: &TaskId,
    role: AgentRole,
) -> Result<(), AgentError> {
    let prior = state.launches.iter().any(|launch| {
        launch.task == *task
            && launch.role == AgentRole::Reviewer
            && launch
                .session
                .as_deref()
                .is_some_and(|session| !session.is_empty())
    });
    if role != AgentRole::Escalation || prior {
        Ok(())
    } else {
        Err(app.error(
            "run-agent",
            Some(task),
            Rejection::Conflict("escalation requires a completed reviewer launch".into()),
        ))
    }
}

fn launch_capacity(task: &domain::task::Task, role: AgentRole, kind: budget::BudgetKind) -> bool {
    let available =
        |budget| budget::remaining(budget, &task.budgets, task.tier.current, &task.authorities) > 0;
    available(kind) && (role != AgentRole::Escalation || available(budget::BudgetKind::Review))
}

fn require_checkpoint(state: &domain::state::State, task: &TaskId) -> Result<(), AgentError> {
    let latest = state.launches.iter().rev().find(|launch| {
        launch.task == *task && launch.role == AgentRole::Implementer && launch.session.is_some()
    });
    if latest.is_none_or(|launch| state.checkpoints.get(task) == Some(&launch.id)) {
        Ok(())
    } else {
        Err(AgentError {
            failed: "run-agent".into(),
            phase: state
                .tasks
                .get(task)
                .map(|value| Box::new(value.phase.clone())),
            why: Rejection::Conflict("checkpoint the previous implementer turn".into()),
            next: state.next().map(Box::new),
        })
    }
}

fn ensure_readiness(
    app: &App<'_>,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    id: &TaskId,
    role: AgentRole,
) -> Result<(), AgentError> {
    if role == AgentRole::Implementer {
        return Ok(());
    }
    let snapshot = review_target(task, delivery).ok_or_else(|| {
        app.error(
            "run-agent",
            Some(id),
            Rejection::Evidence("review requires a snapshot".into()),
        )
    })?;
    verify_proof(task, delivery, &snapshot)
        .map_err(|message| app.error("run-agent", Some(id), Rejection::Evidence(message)))
}

fn previous_session_from_state(
    state: &domain::state::State,
    task: &TaskId,
    role: AgentRole,
) -> Option<String> {
    previous_session(state, task, role).filter(|session| !session.is_empty())
}
