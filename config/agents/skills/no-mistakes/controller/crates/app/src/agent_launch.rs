use crate::delivery_support::verify_proof;
use crate::error::Ctx;
use crate::task_support::{
    assignment, guard_role, next_launch, pair_for, previous_session, review_target, task_ref,
};
use crate::{AgentError, App, ConflictReason, Rejection};
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
    let ctx = app.ctx("run-agent", Some(id));
    let state = app.state("run-agent")?;
    let task = task_ref(&state, id, "run-agent", app)?.clone();
    validate_launch(&ctx, &state, &task, id, role)?;
    let delivery = task
        .deliveries
        .last()
        .ok_or_else(|| ctx.reject(ConflictReason::NoDelivery))?;
    let delivery_id = delivery.id;
    let (assignment, budget_kind, counted) =
        launch_budget(&ctx, &state, &task, id, role, fallback)?;
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
        .map_err(|error| ctx.reject(Rejection::Invalid(error.to_string())))?;
    let prompt_text = crate::task_support::launch_prompt(&task, role, prompt_text);
    let request = LaunchRequest {
        id: launch,
        assignment,
        prompt: prompt_text,
        cwd: state
            .config
            .as_ref()
            .map(|config| config.repo.clone())
            .unwrap_or_default(),
        // A failed or cancelled launch records an empty session, and an empty
        // `--resume` is rejected by the harness, so one killed launch would
        // otherwise poison every later launch of this role in the delivery.
        session: previous_session(&state, id, delivery_id, role)
            .filter(|session| !session.is_empty()),
        reviewer: role != AgentRole::Implementer,
        // The path the reviewer's schema will be written to, for a
        // `Native` harness (Codex). Only the path is decided here; the
        // file itself is written just before the launch actually runs
        // (`agent_support::invoke`), never during `--check`.
        output_schema: (role != AgentRole::Implementer)
            .then(|| app.store.root().join("review-schema.json")),
    };
    Ok(PreparedLaunch {
        task,
        delivery: delivery_id,
        events,
        request,
    })
}

fn validate_launch(
    ctx: &Ctx,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
    role: AgentRole,
) -> Result<(), AgentError> {
    // A background judge is not the coordinator's active launch.
    if state
        .launches
        .iter()
        .any(|launch| launch.outcome.is_none() && launch.role != AgentRole::Judge)
    {
        return Err(ctx.reject(ConflictReason::ActiveLaunchUnsettled));
    }
    require_checkpoint(ctx, state, id)?;
    let delivery = task
        .deliveries
        .last()
        .ok_or_else(|| ctx.reject(ConflictReason::NoDelivery))?;
    guard_role(task, delivery, role).map_err(|reason| ctx.reject(reason))?;
    require_prior_review(ctx, state, id, role)?;
    ensure_readiness(ctx, task, delivery, role)
}

fn launch_budget(
    ctx: &Ctx,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
    role: AgentRole,
    fallback: Option<&Fallback>,
) -> Result<(domain::risk::Assignment, budget::BudgetKind, bool), AgentError> {
    let (assignment, base) = assignment(state, task.tier.current, role, fallback)
        .map_err(|message| ctx.reject(Rejection::Invalid(message)))?;
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
    let caps = state.config.as_ref().map(|config| &config.caps);
    if counted && !launch_capacity(task, role, kind, caps) {
        return Err(ctx.reject(kind));
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
    ctx: &Ctx,
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
        Err(ctx.reject(ConflictReason::EscalationRequiresReview))
    }
}

fn launch_capacity(
    task: &domain::task::Task,
    role: AgentRole,
    kind: budget::BudgetKind,
    caps: Option<&std::collections::BTreeMap<domain::risk::Tier, budget::Limits>>,
) -> bool {
    let available = |budget| {
        budget::remaining(
            budget,
            &task.budgets,
            task.tier.current,
            &task.authorities,
            caps,
        ) > 0
    };
    available(kind) && (role != AgentRole::Escalation || available(budget::BudgetKind::Review))
}

fn require_checkpoint(
    ctx: &Ctx,
    state: &domain::state::State,
    id: &TaskId,
) -> Result<(), AgentError> {
    let latest = state.launches.iter().rev().find(|launch| {
        launch.task == *id && launch.role == AgentRole::Implementer && launch.outcome.is_some()
    });
    if latest.is_none_or(|launch| launch.checkpointed) {
        Ok(())
    } else {
        Err(ctx.reject(ConflictReason::CheckpointRequired))
    }
}

fn ensure_readiness(
    ctx: &Ctx,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    role: AgentRole,
) -> Result<(), AgentError> {
    if role == AgentRole::Implementer {
        return Ok(());
    }
    let snapshot = review_target(task, delivery)
        .ok_or_else(|| ctx.reject(crate::EvidenceError::MissingSnapshot))?;
    verify_proof(task, delivery, &snapshot).map_err(|error| ctx.reject(error))
}
