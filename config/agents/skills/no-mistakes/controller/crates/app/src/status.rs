use crate::delivery_support::{current_sync, review_snapshot, verify_proof};
use crate::error::Ctx;
use crate::task_support::{next_operation, task_ref};
use crate::{AgentError, App, ConflictReason, Output, Rejection, ResultData};
use domain::{
    command::Transition,
    event::Event,
    ids::{IssueKey, JiraStatus, TaskId},
    sync::{self, Sync},
    task::Phase,
};

impl App<'_> {
    pub(super) fn set_status(
        &mut self,
        task: TaskId,
        issue: IssueKey,
        current: JiraStatus,
        target: JiraStatus,
        transitions: Vec<Transition>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("set-status", Some(&task));
        let state = self.state("set-status")?;
        let value = task_ref(&state, &task, "set-status", self)?;
        validate_done(&ctx, &state, value, &issue, &target)?;
        let transition = select_transition(&ctx, &target, &transitions)?;
        let current_sync = current_sync(value, &issue)
            .ok_or_else(|| ctx.reject(Rejection::Invalid("unrecorded subtask".into())))?;
        let attempts = attempts(current_sync);
        let operation = next_operation(&state);
        let intent = domain::sync::StatusIntent {
            operation,
            from: current.clone(),
            target: target.clone(),
            transition: transition.clone(),
            attempts,
            at: self.services.clock.now(),
        };
        sync::intend(current_sync, intent.clone()).map_err(|error| ctx.reject(error))?;
        let events = vec![Event::StatusIntended {
            task: task.clone(),
            issue,
            current,
            target,
            transition: transition.clone(),
            operation,
            attempts,
            at: intent.at,
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
        let ctx = self.ctx("observe-status", Some(&task));
        let state = self.state("observe-status")?;
        let value = task_ref(&state, &task, "observe-status", self)?;
        if evidence.trim().is_empty() {
            return Err(ctx.reject(Rejection::Invalid(
                "fresh observation evidence is required".into(),
            )));
        }
        let current = current_sync(value, &issue)
            .ok_or_else(|| ctx.reject(Rejection::Invalid("unrecorded subtask".into())))?;
        let observed = sync::observe(current, status.clone(), self.services.clock.now())
            .map_err(|error| ctx.reject(error))?;
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
}

fn select_transition(
    ctx: &Ctx,
    target: &JiraStatus,
    transitions: &[Transition],
) -> Result<domain::ids::TransitionId, AgentError> {
    let pairs = transitions
        .iter()
        .map(|entry| (entry.id.clone(), entry.to.clone()))
        .collect::<Vec<_>>();
    sync::transition(target, &pairs).cloned().map_err(|error| {
        ctx.reject(Rejection::Invalid(format!(
            "transition rejected: {error:?}"
        )))
    })
}

fn attempts(sync: &Sync) -> u8 {
    match sync {
        Sync::Failed(failure) => failure.intent.attempts.saturating_add(1),
        Sync::Pending | Sync::Unknown(_) | Sync::Confirmed(_) => 1,
    }
}

fn validate_done(
    ctx: &Ctx,
    state: &domain::state::State,
    task: &domain::task::Task,
    issue: &IssueKey,
    target: &JiraStatus,
) -> Result<(), AgentError> {
    let done = state
        .config
        .as_ref()
        .is_some_and(|config| target == &config.jira.statuses.done);
    if !done {
        return Ok(());
    }
    if !matches!(task.phase, Phase::Verified { .. } | Phase::Completed { .. }) {
        return Err(ctx.reject(ConflictReason::JiraDoneRequiresVerified));
    }
    if IssueKey::from(task.id.clone()) != *issue && !task.subtasks.contains_key(issue) {
        return Err(ctx.reject(Rejection::Invalid("unrecorded subtask".into())));
    }
    let delivery = task.deliveries.last().ok_or_else(|| {
        ctx.reject(crate::EvidenceError::Other(
            "verified task has no delivery".into(),
        ))
    })?;
    let snapshot = review_snapshot(delivery).ok_or_else(|| {
        ctx.reject(crate::EvidenceError::Other(
            "verified task has no review snapshot".into(),
        ))
    })?;
    verify_proof(task, delivery, snapshot).map_err(|error| ctx.reject(error))?;
    Ok(())
}
