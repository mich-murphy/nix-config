use crate::delivery_support::{current_sync, get_task, next_operation};
use crate::{AgentError, App, Output, Rejection, ResultData};
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
        let state = self.state("set-status")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("set-status", Some(&task), Rejection::Invalid(message))
        })?;
        validate_done(self, &state, value, &task, &issue, &target)?;
        let transition = select_transition(self, &task, &target, &transitions)?;
        let current_sync = current_sync(value, &issue).ok_or_else(|| {
            self.error(
                "set-status",
                Some(&task),
                Rejection::Invalid("unrecorded subtask".into()),
            )
        })?;
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
        sync::intend(current_sync, intent.clone()).map_err(|error| {
            self.error(
                "set-status",
                Some(&task),
                Rejection::Conflict(format!("status intent rejected: {error:?}")),
            )
        })?;
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
}

fn select_transition(
    app: &App<'_>,
    task: &TaskId,
    target: &JiraStatus,
    transitions: &[Transition],
) -> Result<domain::ids::TransitionId, AgentError> {
    let pairs = transitions
        .iter()
        .map(|entry| (entry.id.clone(), entry.to.clone()))
        .collect::<Vec<_>>();
    sync::transition(target, &pairs).cloned().map_err(|error| {
        app.error(
            "set-status",
            Some(task),
            Rejection::Invalid(format!("transition rejected: {error:?}")),
        )
    })
}

fn attempts(sync: &Sync) -> u8 {
    match sync {
        Sync::Failed(failure) => failure.intent.attempts.saturating_add(1),
        Sync::Pending | Sync::Unknown(_) | Sync::Confirmed(_) => 1,
    }
}

fn validate_done(
    app: &App<'_>,
    state: &domain::state::State,
    task: &domain::task::Task,
    id: &TaskId,
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
    if !matches!(task.phase, Phase::Verified { .. }) {
        return Err(app.error(
            "set-status",
            Some(id),
            Rejection::Conflict("Jira Done requires verified delivery".into()),
        ));
    }
    if IssueKey::from(id.clone()) != *issue && !task.subtasks.contains_key(issue) {
        return Err(app.error(
            "set-status",
            Some(id),
            Rejection::Invalid("unrecorded subtask".into()),
        ));
    }
    Ok(())
}
