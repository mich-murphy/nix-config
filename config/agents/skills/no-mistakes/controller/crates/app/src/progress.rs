use crate::task_support::{current_work_delivery, task_ref};
use crate::{AgentError, App, Output, Rejection};
use domain::{budget::BudgetKind, command::AgentRole, event::Event, ids::TaskId};

pub(super) struct CheckpointInput {
    pub advanced: bool,
    pub observation: String,
    pub next: String,
    pub outside_paths: Vec<String>,
    pub scope_reason: Option<String>,
}

impl App<'_> {
    pub(super) fn checkpoint(
        &mut self,
        task: TaskId,
        input: CheckpointInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("checkpoint")?;
        let value = task_ref(&state, &task, "checkpoint", self)?;
        let launch = checkpoint_target(self, &state, value, &task)?;
        if already_checkpointed(&state, launch) {
            return Err(self.error(
                "checkpoint",
                Some(&task),
                Rejection::Conflict("implementation turn is already checkpointed".into()),
            ));
        }
        if !input.outside_paths.is_empty()
            && input.scope_reason.as_deref().is_none_or(str::is_empty)
        {
            return Err(self.error(
                "checkpoint",
                Some(&task),
                Rejection::Invalid("out-of-plan paths need a scope reason".into()),
            ));
        }
        let stalled = checkpoint_stalls(value, launch.is_none(), input.advanced);
        let mut events = vec![Event::Checkpointed {
            task: task.clone(),
            launch,
            advanced: input.advanced,
            stalled,
            observation: input.observation,
            next: input.next,
        }];
        if stalled >= domain::budget::Limits::for_tier(value.tier.current).stalled {
            events.push(Event::Held {
                task: task.clone(),
                reason: domain::task::HoldReason::BudgetExhausted(BudgetKind::Stalled),
                at: self.services.clock.now(),
            });
        }
        self.commit("checkpoint", Some(&task), events, check)
    }
}

/// The launch this checkpoint marks, or `None` for an integration
/// checkpoint (a snapshot with no implementer launch of its own).
fn checkpoint_target(
    app: &App<'_>,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
) -> Result<Option<domain::ids::LaunchId>, AgentError> {
    let latest = state
        .launches
        .iter()
        .rev()
        .find(|launch| {
            launch.task == *id && launch.role == AgentRole::Implementer && launch.outcome.is_some()
        })
        .map(|launch| launch.id);
    let integration = latest.is_none()
        && current_work_delivery(task).is_some_and(|(work, _)| work.snapshot.is_some());
    if latest.is_some() || integration {
        Ok(latest)
    } else {
        Err(app.error(
            "checkpoint",
            Some(id),
            Rejection::Conflict(
                "checkpoint requires a terminal turn or integration snapshot".into(),
            ),
        ))
    }
}

fn already_checkpointed(
    state: &domain::state::State,
    launch: Option<domain::ids::LaunchId>,
) -> bool {
    launch.is_some_and(|id| {
        state
            .launches
            .iter()
            .any(|item| item.id == id && item.checkpointed)
    })
}

fn checkpoint_stalls(task: &domain::task::Task, integration: bool, advanced: bool) -> u32 {
    if integration {
        task.budgets.stalled_checkpoints
    } else if advanced {
        0
    } else {
        task.budgets.stalled_checkpoints.saturating_add(1)
    }
}
