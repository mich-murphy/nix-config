use crate::task_support::{
    assignment, guard_role, next_launch, pair_for, previous_session, previous_session_from_role,
    review_target, task_ref,
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
use std::{fs, path::PathBuf};

impl App<'_> {
    pub(super) fn run_agent(
        &mut self,
        task: TaskId,
        role: AgentRole,
        prompt: PathBuf,
        fallback: Option<Fallback>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("run-agent")?;
        let value = task_ref(&state, &task, "run-agent", self)?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        guard_role(value, delivery, role).map_err(|message| {
            self.error("run-agent", Some(&task), Rejection::Conflict(message))
        })?;
        let (assignment, budget_kind) =
            assignment(&state, value.tier.current, role, fallback.as_ref()).map_err(|message| {
                self.error("run-agent", Some(&task), Rejection::Invalid(message))
            })?;
        let counted = pair_for(value, role).is_none_or(|authority| !authority::scoped(authority));
        if counted
            && budget::remaining(
                budget_kind,
                &value.budgets,
                value.tier.current,
                &value.authorities,
            ) == 0
        {
            return Err(self.error(
                "run-agent",
                Some(&task),
                Rejection::Budget(format!("{budget_kind:?} budget exhausted")),
            ));
        }
        let id = next_launch(&state);
        let launch = Launch {
            id,
            task: task.clone(),
            delivery: delivery.id,
            role,
            prompt: prompt.clone(),
            session: None,
            counted,
        };
        let mut events = vec![
            Event::LaunchStarted { launch },
            Event::BudgetSpent {
                task: task.clone(),
                budget: budget_kind,
                counted,
            },
        ];
        if let Some(authority) = pair_for(value, role) {
            events.push(Event::PairSpent {
                task: task.clone(),
                authority: authority.id,
                launch: id,
                implementation: role == AgentRole::Implementer,
            });
        }
        if check {
            return self.commit("run-agent", Some(&task), events, true);
        }
        let prompt_text = fs::read_to_string(&prompt).map_err(|error| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Invalid(error.to_string()),
            )
        })?;
        let request = LaunchRequest {
            id,
            assignment,
            prompt: prompt_text,
            cwd: state
                .config
                .as_ref()
                .map(|config| config.repo.clone())
                .unwrap_or_default(),
            session: previous_session(&state, &task, role),
            reviewer: role != AgentRole::Implementer,
        };
        let reviewer = role != AgentRole::Implementer;
        if !reviewer {
            return self.run_implementer(task, events, request);
        }
        self.run_reviewer(task, delivery.id, events, request, value)
    }

    fn run_implementer(
        &mut self,
        task: TaskId,
        events: Vec<Event>,
        request: LaunchRequest,
    ) -> Result<Output, AgentError> {
        let mut records = self.write("run-agent", Some(&task), events, false)?;
        let result =
            self.services.harness.run(&request).map_err(|error| {
                self.error("run-agent", Some(&task), Rejection::External(error.0))
            })?;
        records.extend(self.write(
            "run-agent",
            Some(&task),
            vec![Event::LaunchEnded {
                launch: request.id,
                result: LaunchOutcome::Completed {
                    session: result.session,
                    output: result.output.clone(),
                },
                usage: result.tokens,
            }],
            false,
        )?);
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
        let result =
            self.services.harness.run(&request).map_err(|error| {
                self.error("run-agent", Some(&task), Rejection::External(error.0))
            })?;
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
        review.verdict =
            review::verdict(&review.findings, &review.evidence_gaps, value.tier.current);
        review::validate(
            &review,
            &snapshot,
            previous_session_from_role(value, AgentRole::Implementer).as_deref(),
        )
        .map_err(|error| {
            self.error(
                "run-agent",
                Some(&task),
                Rejection::Evidence(format!("invalid review: {error:?}")),
            )
        })?;
        events.push(Event::LaunchEnded {
            launch: request.id,
            result: LaunchOutcome::Completed {
                session: result.session,
                output: result.output.clone(),
            },
            usage: result.tokens,
        });
        events.push(Event::ReviewSettled {
            task: task.clone(),
            delivery,
            launch: request.id,
            findings: review.findings,
            gaps: review.evidence_gaps,
            verdict: review.verdict,
        });
        let records = self.write("run-agent", Some(&task), events, false)?;
        Ok(Output {
            events: records,
            result: ResultData::Launch {
                launch: request.id,
                output: result.output,
            },
        })
    }
}
