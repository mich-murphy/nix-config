use crate::delivery_support::verify_proof;
use crate::task_support::{
    assignment, guard_role, next_launch, pair_for, previous_session, review_target, task_ref,
};
use crate::{AgentError, App, Rejection};
use domain::{
    authority, budget,
    command::{AgentRole, Fallback},
    event::{Event, Launch},
    ids::{DeliveryId, TaskId},
    ports::LaunchRequest,
};
use std::{fs, path::Path};

pub(super) struct PreparedLaunch {
    pub(super) task: domain::task::Task,
    pub(super) delivery: DeliveryId,
    pub(super) events: Vec<Event>,
    pub(super) request: LaunchRequest,
}

pub(super) fn prepare_launch(
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
    if state.launches.iter().any(|launch| launch.outcome.is_none()) {
        return Err(app.error(
            "run-agent",
            Some(id),
            Rejection::Conflict("settle the active launch first".into()),
        ));
    }
    require_checkpoint(app, state, id)?;
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
                outcome: None,
                usage: None,
                checkpointed: false,
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
            && matches!(
                launch.outcome,
                Some(domain::event::LaunchOutcome::Completed { .. })
            )
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

fn require_checkpoint(
    app: &App<'_>,
    state: &domain::state::State,
    task: &TaskId,
) -> Result<(), AgentError> {
    let latest = state.launches.iter().rev().find(|launch| {
        launch.task == *task && launch.role == AgentRole::Implementer && launch.outcome.is_some()
    });
    if latest.is_none_or(|launch| launch.checkpointed) {
        Ok(())
    } else {
        Err(AgentError {
            failed: "run-agent".into(),
            phase: state
                .tasks
                .get(task)
                .map(|value| Box::new(value.phase.clone())),
            why: Rejection::Conflict("checkpoint the previous implementer turn".into()),
            next: state.next(app.services.clock.now()).map(Box::new),
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
