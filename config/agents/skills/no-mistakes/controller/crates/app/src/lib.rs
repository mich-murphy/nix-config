mod agent;
mod delivery;
mod delivery_support;
mod error;
mod finish;
mod output;
mod proof;
mod publish;
mod queue;
mod status;
mod task;
mod task_support;

pub use error::{AgentError, Rejection};
pub use output::{Output, ResultData, UsageReport};

use adapters::{Process, sqlite::Store};
use domain::{
    command::Command,
    event::{Actor, Event, EventRecord},
    ids::TaskId,
    ports::{Clock, GitHub, Harness, Vcs},
    state::State,
};

pub struct Services<'a> {
    pub vcs: &'a dyn Vcs,
    pub github: &'a dyn GitHub,
    pub harness: &'a dyn Harness,
    pub process: &'a dyn Process,
    pub clock: &'a dyn Clock,
}

pub fn initialize(
    root: &std::path::Path,
    config: domain::command::RunConfig,
    profile: domain::risk::Profile,
    services: Services<'_>,
) -> Result<Output, AgentError> {
    domain::risk::validate(&profile).map_err(|error| AgentError {
        failed: "init".into(),
        phase: None,
        why: Rejection::Invalid(format!("invalid profile: {error:?}")),
        next: None,
    })?;
    if !services.vcs.ignored(root).map_err(|error| AgentError {
        failed: "init".into(),
        phase: None,
        why: Rejection::External(error.0),
        next: None,
    })? {
        return Err(AgentError {
            failed: "init".into(),
            phase: None,
            why: Rejection::Invalid("run directory must be Git-ignored".into()),
            next: None,
        });
    }
    let mut store = Store::create(root).map_err(|error| AgentError {
        failed: "init".into(),
        phase: None,
        why: Rejection::Internal(error.to_string()),
        next: None,
    })?;
    let capabilities = services.harness.capabilities();
    let event = Event::RunInitialized {
        config: Box::new(config),
        profile: Box::new(profile),
        capabilities,
    };
    let events = store
        .commit(Actor::Coordinator, services.clock.now(), &[event])
        .map_err(|error| AgentError {
            failed: "init".into(),
            phase: None,
            why: Rejection::Internal(error.to_string()),
            next: None,
        })?;
    Ok(Output {
        events,
        result: ResultData::Initialized { capabilities },
    })
}

pub struct App<'a> {
    store: Store,
    services: Services<'a>,
}

impl<'a> App<'a> {
    pub fn new(store: Store, services: Services<'a>) -> Self {
        Self { store, services }
    }

    pub fn execute(&mut self, command: Command, check: bool) -> Result<Output, AgentError> {
        match command {
            Command::Status => self.status(),
            Command::Next => self.next(),
            Command::UsageReport => self.usage(),
            Command::ReviewSchema => Ok(Output {
                events: Vec::new(),
                result: ResultData::ReviewSchema {
                    schema: review_schema(),
                },
            }),
            Command::ValidateReview { review } => self.validate_review(&review),
            Command::Discover {
                tasks,
                planning_order,
            } => self.discover(tasks, planning_order, check),
            Command::Refresh { changed, removed } => self.refresh(changed, removed, check),
            Command::Claim { task } => self.claim(task, check),
            Command::Hold { task, reason } => self.hold(task, reason, check),
            Command::Resume {
                task,
                final_revisit,
            } => self.resume(task, final_revisit, check),
            Command::Brief { task, criteria } => self.brief(task, criteria, check),
            Command::Plan { task, plan } => self.plan(task, plan, check),
            Command::Checkpoint {
                task,
                advanced,
                observation,
                next,
                outside_paths,
                scope_reason,
            } => self.checkpoint(
                task,
                task::CheckpointInput {
                    advanced,
                    observation,
                    next,
                    outside_paths,
                    scope_reason,
                },
                check,
            ),
            Command::Snapshot { task } => self.snapshot(task, check),
            Command::SubtaskRecord {
                task,
                issue,
                criteria,
                owned,
                was_terminal,
                status,
            } => self.subtask_record(
                task,
                task::SubtaskInput {
                    issue,
                    criteria,
                    owned,
                    was_terminal,
                    status,
                },
                check,
            ),
            Command::EscalateTier { task, to, reason } => {
                self.escalate_tier(task, to, reason, check)
            }
            Command::BindSlot {
                task,
                slot,
                branch,
                authority,
            } => self.bind_slot(task, slot, branch, authority, check),
            Command::Cleanup { task, delete } => self.cleanup(task, delete, check),
            Command::RecordProof {
                task,
                delivery,
                entries,
            } => self.record_proof(task, delivery, entries, check),
            Command::RunCheck {
                task,
                criteria,
                argv,
                cwd,
                timeout_seconds,
                implementation,
            } => self.run_check(
                task,
                task::CheckInput {
                    criteria,
                    argv,
                    cwd,
                    timeout_seconds,
                    implementation,
                },
                check,
            ),
            Command::RunAgent {
                task,
                role,
                prompt,
                fallback,
            } => self.run_agent(task, role, prompt, fallback, check),
            Command::Disposition {
                task,
                finding,
                disposition,
            } => self.disposition(task, finding, disposition, check),
            Command::HumanReview {
                task,
                snapshot,
                receipt,
            } => self.human_review(task, snapshot, receipt, check),
            Command::LessonRecord { task, lesson } => self.lesson(task, lesson, check),
            Command::Publish {
                task,
                step,
                title,
                body,
                method,
            } => self.publish(
                task,
                delivery::PublishInput {
                    step,
                    title,
                    body,
                    method,
                },
                check,
            ),
            Command::ObservePr { task, pr } => self.observe_pr(task, pr, check),
            Command::PollChecks { task, wait } => self.poll_checks(task, wait, check),
            Command::FinalVerify {
                task,
                commit,
                evidence,
            } => self.final_verify(task, commit, evidence, check),
            Command::Complete { task } => self.complete(task, check),
            Command::SetStatus {
                task,
                issue,
                current,
                target,
                transitions,
            } => self.set_status(task, issue, current, target, transitions, check),
            Command::ObserveStatus {
                task,
                issue,
                status,
                evidence,
            } => self.observe_status(task, issue, status, evidence, check),
            Command::Grant { task, authority } => self.grant(task, *authority, check),
            Command::OpenDelivery {
                task,
                kind,
                authority,
                jira,
            } => self.open_delivery(task, kind, *authority, jira, check),
            Command::NarrowAcceptance {
                task,
                criteria,
                authority,
            } => self.narrow(task, criteria, *authority, check),
            Command::RecoverOperation { target, terminate } => {
                self.recover_operation(target, terminate, check)
            }
            Command::Init { .. } => Err(self.error(
                "init",
                None,
                Rejection::Conflict("run already exists".into()),
            )),
        }
    }

    fn state(&self, command: &str) -> Result<State, AgentError> {
        self.store
            .state()
            .map_err(|error| self.error(command, None, Rejection::Internal(error.to_string())))
    }

    fn commit(
        &mut self,
        command: &str,
        task: Option<&TaskId>,
        events: Vec<Event>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let records = self.write(command, task, events, check)?;
        Ok(Output {
            events: records,
            result: ResultData::Applied,
        })
    }

    fn write(
        &mut self,
        command: &str,
        task: Option<&TaskId>,
        events: Vec<Event>,
        check: bool,
    ) -> Result<Vec<EventRecord>, AgentError> {
        let at = self.services.clock.now();
        if check {
            let start = self
                .store
                .events()
                .map_err(|error| self.error(command, task, Rejection::Internal(error.to_string())))?
                .len() as u64
                + 1;
            return Ok(events
                .into_iter()
                .enumerate()
                .map(|(offset, event)| EventRecord {
                    sequence: start + offset as u64,
                    at,
                    actor: Actor::Coordinator,
                    event,
                })
                .collect());
        }
        self.store
            .commit(Actor::Coordinator, at, &events)
            .map_err(|error| self.error(command, task, Rejection::Internal(error.to_string())))
    }

    fn error(&self, failed: &str, task: Option<&TaskId>, why: Rejection) -> AgentError {
        let state = self.store.state().ok();
        let phase = task
            .and_then(|id| state.as_ref()?.tasks.get(id))
            .map(|task| Box::new(task.phase.clone()));
        let next = state.and_then(|value| value.next()).map(Box::new);
        AgentError {
            failed: failed.into(),
            phase,
            why,
            next,
        }
    }
}

fn review_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["launch", "session", "snapshot", "findings", "evidence_gaps", "reviewer_opinion", "verdict", "dispositions"],
        "additionalProperties": false
    })
}
