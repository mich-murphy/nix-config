use crate::agent_launch::prepare_launch;
use crate::task_support::{previous_session, review_target};
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    command::{AgentRole, Fallback},
    event::{Event, LaunchOutcome},
    ids::{DeliveryId, TaskId},
    ports::{LaunchRequest, LaunchResult},
    review::{self, Review},
};
use std::path::PathBuf;

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

    /// `LaunchStarted`, `BudgetSpent` and any `PairSpent` are already
    /// committed (by `agent_support::invoke`, before the process passed the
    /// gate). A launch that times out or is cancelled after that point is
    /// already charged and is left unsettled for `recover-operation`; only a
    /// successful run writes `LaunchEnded` here.
    fn run_implementer(
        &mut self,
        task: TaskId,
        events: Vec<Event>,
        request: LaunchRequest,
    ) -> Result<Output, AgentError> {
        let (mut records, result) = crate::agent_support::invoke(self, &task, &request, events)?;
        let end = vec![Event::LaunchEnded {
            launch: request.id,
            result: LaunchOutcome::Completed {
                session: result.session,
                output: result.output.clone(),
            },
            usage: result.tokens,
        }];
        records.extend(self.write("run-agent", Some(&task), end, false)?);
        Ok(Output {
            events: records,
            result: ResultData::Launch {
                launch: request.id,
                output: result.output,
            },
        })
    }

    /// Codex is `Native` and pi is `SelfValidated`: a malformed, mismatched
    /// or invalid review is a failed, charged launch, not a free re-prompt
    /// (design Section 14). The launch is settled `Failed` here rather than
    /// left open for `recover-operation`.
    fn run_reviewer(
        &mut self,
        task: TaskId,
        delivery: DeliveryId,
        events: Vec<Event>,
        request: LaunchRequest,
        value: &domain::task::Task,
    ) -> Result<Output, AgentError> {
        let (mut records, result) = crate::agent_support::invoke(self, &task, &request, events)?;
        let state = self.state("run-agent")?;
        match settle_review(value, &state, &task, &request, &result) {
            Ok(review) => {
                let settled = vec![
                    Event::LaunchEnded {
                        launch: request.id,
                        result: LaunchOutcome::Completed {
                            session: result.session.clone(),
                            output: result.output.clone(),
                        },
                        usage: result.tokens,
                    },
                    Event::ReviewSettled {
                        task: task.clone(),
                        delivery,
                        review: Box::new(review),
                    },
                ];
                records.extend(self.write("run-agent", Some(&task), settled, false)?);
                Ok(Output {
                    events: records,
                    result: ResultData::Launch {
                        launch: request.id,
                        output: result.output,
                    },
                })
            }
            Err(rejection) => {
                let failed = vec![Event::LaunchEnded {
                    launch: request.id,
                    result: LaunchOutcome::Failed {
                        reason: format!("{rejection:?}"),
                    },
                    usage: result.tokens,
                }];
                records.extend(self.write("run-agent", Some(&task), failed, false)?);
                Err(self.error("run-agent", Some(&task), rejection))
            }
        }
    }
}

fn settle_review(
    value: &domain::task::Task,
    state: &domain::state::State,
    task: &TaskId,
    request: &LaunchRequest,
    result: &LaunchResult,
) -> Result<Review, Rejection> {
    let mut review: Review = serde_json::from_str(&result.output)
        .map_err(|error| Rejection::Invalid(format!("malformed review: {error}")))?;
    let delivery = value
        .deliveries
        .last()
        .ok_or_else(|| Rejection::Conflict("task has no delivery".into()))?;
    let snapshot = review_target(value, delivery)
        .ok_or_else(|| Rejection::Evidence("review requires a snapshot".into()))?;
    if review.launch != request.id || review.session != result.session {
        return Err(Rejection::Invalid(
            "review launch or session does not match the harness".into(),
        ));
    }
    review.verdict = review::verdict(&review.findings, &review.evidence_gaps, value.tier.current);
    let implementer = previous_session_from_state(state, task, AgentRole::Implementer);
    review::validate(&review, &snapshot, implementer.as_deref())
        .map_err(|error| Rejection::Evidence(format!("invalid review: {error:?}")))?;
    Ok(review)
}

fn previous_session_from_state(
    state: &domain::state::State,
    task: &TaskId,
    role: AgentRole,
) -> Option<String> {
    previous_session(state, task, role).filter(|session| !session.is_empty())
}
