//! The LLM-as-judge. `judge` grades one task from the ledger, the
//! coordinator transcript and the rejection log, through the run's own
//! harness on the tier's reviewer model, and records the verdict as a
//! `Judged` event. `complete` and `usage-report` start it detached so the
//! coordinator never waits; the run's quality payload is re-sent to the
//! trace after every verdict.

mod brief;
mod coordinator;
mod transcript;

use crate::agent_support::OutputSchema;
use crate::task_support::{next_launch, task_ref};
use crate::{AgentError, App, ConflictReason, Output, Rejection, ResultData};
use adapters::ProcessRequest;
use domain::{
    command::AgentRole,
    event::{Event, Launch, LaunchOutcome},
    ids::{DeliveryId, LaunchId, TaskId},
    judge::{self, JudgeReport, Judgment},
    ports::{LaunchRequest, LaunchResult, TraceState},
    risk::{Assignment, Effort, Tier},
    state::State,
    task::{Phase, Task},
};
use std::collections::BTreeMap;

const RUBRIC: &str = include_str!("judge/rubric.md");
/// Trace-state flag: `usage-report` ran while judges were still out, so
/// the last judge to land closes the instance.
const FINISH_REQUESTED: &str = "finish-requested";
/// Trace-state key holding the coordinator transcript location.
pub(super) const COORDINATOR: &str = "coordinator";

/// The hook's note for the current working directory, for `init` to keep.
pub(super) fn coordinator_note() -> Option<String> {
    coordinator::locate()
}

impl App<'_> {
    pub(super) fn judge(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let ctx = self.ctx("judge", Some(&task));
        let state = self.state("judge")?;
        let value = task_ref(&state, &task, "judge", self)?.clone();
        if value.judgment.is_some() {
            return Err(ctx.reject(ConflictReason::AlreadyJudged));
        }
        if !judgeable(&value) {
            return Err(ctx.reject(ConflictReason::JudgeRequiresClaimedTask));
        }
        let launch = next_launch(&state);
        let prompt_path = self
            .store
            .root()
            .join("judge")
            .join(format!("{task}.prompt.md"));
        let events = vec![Event::LaunchStarted {
            launch: Launch {
                id: launch,
                task: task.clone(),
                delivery: value.deliveries.last().map_or(DeliveryId(0), |d| d.id),
                role: AgentRole::Judge,
                prompt: prompt_path.clone(),
                outcome: None,
                usage: None,
                checkpointed: false,
                process: None,
            },
        }];
        if check {
            return self.commit("judge", Some(&task), events, true);
        }
        let prompt = self.judge_prompt(&state, &value)?;
        std::fs::create_dir_all(self.store.root().join("judge"))
            .and_then(|()| std::fs::write(&prompt_path, &prompt))
            .map_err(|error| ctx.reject(Rejection::Internal(error.to_string())))?;
        let request = LaunchRequest {
            id: launch,
            assignment: judge_assignment(&state, value.tier.current)
                .map_err(|message| ctx.reject(Rejection::Invalid(message)))?,
            prompt,
            cwd: state
                .config
                .as_ref()
                .map(|config| config.repo.clone())
                .unwrap_or_default(),
            session: None,
            reviewer: true,
            output_schema: Some(self.store.root().join("judge-schema.json")),
        };
        let outcome = self.run_judge(&task, launch, &request, events);
        self.settle_judge(&task);
        outcome
    }

    fn judge_prompt(&self, state: &State, task: &Task) -> Result<String, AgentError> {
        let events = self.store.events().map_err(|error| {
            self.error(
                "judge",
                Some(&task.id),
                Rejection::Internal(error.to_string()),
            )
        })?;
        let rejections = self.store.rejections().unwrap_or_default();
        let coordinator = self
            .store
            .get(COORDINATOR)
            .ok()
            .flatten()
            .or_else(coordinator::locate);
        let now = self.services.clock.now();
        let body = brief::render(
            state,
            task,
            &events,
            &rejections,
            coordinator.as_deref(),
            now,
        );
        let schema =
            serde_json::to_string_pretty(&schemars::schema_for!(JudgeReport)).unwrap_or_default();
        Ok(format!(
            "{RUBRIC}\n\n## Output schema\n\n```json\n{schema}\n```\n\n{body}"
        ))
    }

    fn run_judge(
        &mut self,
        task: &TaskId,
        launch: LaunchId,
        request: &LaunchRequest,
        events: Vec<Event>,
    ) -> Result<Output, AgentError> {
        let (mut records, result) = crate::agent_support::invoke(
            self,
            "judge",
            task,
            request,
            events,
            OutputSchema::Judge,
        )?;
        match parse_report(&result) {
            Ok(report) => {
                let settled = vec![
                    Event::LaunchEnded {
                        launch,
                        result: LaunchOutcome::Completed {
                            session: result.session.clone(),
                            output: result.output.clone(),
                        },
                        usage: result.tokens,
                    },
                    Event::Judged {
                        task: task.clone(),
                        judgment: Box::new(Judgment {
                            launch,
                            session: result.session.clone(),
                            report,
                        }),
                    },
                ];
                records.extend(self.write("judge", Some(task), settled, false)?);
                self.publish_quality();
                Ok(Output {
                    events: records,
                    next: None,
                    result: ResultData::Launch {
                        launch,
                        output: result.output,
                    },
                })
            }
            Err(rejection) => {
                let failed = vec![Event::LaunchEnded {
                    launch,
                    result: LaunchOutcome::Failed {
                        reason: format!("{rejection:?}"),
                    },
                    usage: result.tokens,
                }];
                records.extend(self.write("judge", Some(task), failed, false)?);
                Err(self.error("judge", Some(task), rejection))
            }
        }
    }

    /// Re-sends the whole quality payload: the latest verdict replaces the
    /// previous payload on the instance.
    fn publish_quality(&mut self) {
        if let Ok(state) = self.state("judge") {
            self.services
                .tracer
                .quality(&mut self.store, &judge::quality(&state));
        }
    }

    /// Clears this judge's in-flight mark; the last one out closes the
    /// trace instance if `usage-report` asked for that.
    fn settle_judge(&mut self, task: &TaskId) {
        let remaining = self.store.judge_finished(task).unwrap_or(0);
        let requested = matches!(self.store.get(FINISH_REQUESTED), Ok(Some(_)));
        if remaining == 0 && requested {
            let _ = self.store.remove(FINISH_REQUESTED);
            self.services.tracer.finish(&mut self.store);
        }
    }

    /// Starts `judge` for `task` detached, logging under the run
    /// directory. Best effort: a failure to start leaves nothing pending.
    pub(super) fn spawn_judge(&mut self, task: &TaskId) {
        let root = self.store.root().to_owned();
        let Ok(program) = std::env::current_exe() else {
            return;
        };
        let directory = root.join("judge");
        if std::fs::create_dir_all(&directory).is_err() || self.store.judge_started(task).is_err() {
            return;
        }
        let request = ProcessRequest {
            program: program.to_string_lossy().into_owned(),
            args: vec![
                "judge".into(),
                "--run".into(),
                root.to_string_lossy().into_owned(),
                task.to_string(),
            ],
            cwd: root.clone(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
            timeout_seconds: 0,
        };
        let log = directory.join(format!("{task}.log"));
        if self.services.process.spawn(&request, &log).is_err() {
            let _ = self.store.judge_finished(task);
        }
    }

    /// Starts a judge for every worked task still without a verdict and
    /// returns how many judges are then in flight. Zero means the trace
    /// instance can close now; otherwise the last judge closes it.
    pub(super) fn sweep_judges(&mut self, state: &State) -> u64 {
        let unjudged: Vec<TaskId> = state
            .tasks
            .values()
            .filter(|task| task.judgment.is_none() && judgeable(task))
            .map(|task| task.id.clone())
            .collect();
        for task in &unjudged {
            self.spawn_judge(task);
        }
        let pending = self.store.judges_pending().unwrap_or(0);
        if pending > 0 {
            let _ = self.store.set(FINISH_REQUESTED, "1");
        }
        pending
    }
}

/// A task is worth judging once it has been claimed: everything before
/// that is queue bookkeeping with no delivery, review or friction to grade.
fn judgeable(task: &Task) -> bool {
    !matches!(
        task.phase,
        Phase::Queued | Phase::Blocked(_) | Phase::NeedsInput(_) | Phase::Excluded(_)
    )
}

/// The tier's reviewer model at medium effort: strong enough to weigh a
/// whole task, cheaper than a review.
fn judge_assignment(state: &State, tier: Tier) -> Result<Assignment, String> {
    let profile = state.profile.as_ref().ok_or("profile unavailable")?;
    let roles = match tier {
        Tier::Trivial => &profile.tier.trivial,
        Tier::Lite => &profile.tier.lite,
        Tier::Full => &profile.tier.full,
    };
    Ok(Assignment {
        model: roles.reviewer.model.clone(),
        effort: Effort::Medium,
    })
}

fn parse_report(result: &LaunchResult) -> Result<JudgeReport, Rejection> {
    let report: JudgeReport = serde_json::from_str(result.output.trim())
        .map_err(|error| Rejection::Invalid(format!("malformed judge report: {error}")))?;
    let scores = [
        report.overall,
        report.outcome.score,
        report.efficiency.score,
        report.friction.score,
        report.process.score,
    ];
    if scores.iter().any(|score| *score > 100) {
        return Err(Rejection::Invalid("judge scores must be 0 to 100".into()));
    }
    Ok(report)
}
