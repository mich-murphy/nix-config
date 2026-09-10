mod agent;
mod agent_launch;
mod agent_support;
mod authority;
mod checks;
mod delivery;
mod delivery_support;
mod error;
mod finish;
mod judge;
mod output;
mod progress;
mod proof;
mod publish;
mod queue;
mod recovery_support;
pub mod schema;
mod snapshot;
mod status;
mod task;
mod task_support;
mod verification;
mod worktree;

pub use error::{AgentError, ConflictReason, EvidenceError, Rejection};
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
    pub tracer: &'a dyn domain::ports::Tracer,
}

/// `initialize` runs before any `App` exists, so it cannot use `App::ctx`;
/// `init` never has a task or a next action to report.
fn init_error(why: Rejection) -> AgentError {
    AgentError::new("init", None, why, None)
}

pub fn initialize(
    root: &std::path::Path,
    config: domain::command::RunConfig,
    profile: domain::risk::Profile,
    services: Services<'_>,
) -> Result<Output, AgentError> {
    domain::risk::validate(&profile)
        .map_err(|error| init_error(Rejection::Invalid(format!("invalid profile: {error:?}"))))?;
    if !services
        .vcs
        .ignored(root)
        .map_err(|error| init_error(Rejection::External(error.0)))?
    {
        return Err(init_error(Rejection::Invalid(
            "run directory must be Git-ignored".into(),
        )));
    }
    let mut store =
        Store::create(root).map_err(|error| init_error(Rejection::Internal(error.to_string())))?;
    let capabilities = services.harness.capabilities();
    let event = Event::RunInitialized {
        config: Box::new(config),
        profile: Box::new(profile),
        capabilities,
    };
    let events = store
        .commit(Actor::Coordinator, services.clock.now(), &[event])
        .map_err(|error| init_error(Rejection::Internal(error.to_string())))?;
    services.tracer.record(&mut store, &events);
    // The coordinator transcript location, if the skill's hook wrote one,
    // is kept beside the ledger for the judge; it is not a domain fact.
    if let Some(note) = judge::coordinator_note() {
        let _ = domain::ports::TraceState::set(&mut store, judge::COORDINATOR, &note);
    }
    Ok(Output {
        events,
        next: None,
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

    /// Runs `command` and, when it wrote events, attaches the action
    /// `next` would now name, so one round trip carries both the result
    /// and the following step.
    pub fn execute(&mut self, command: Command, check: bool) -> Result<Output, AgentError> {
        let mut output = self.dispatch(command, check)?;
        if !check && !output.events.is_empty() {
            let now = self.services.clock.now();
            output.next = self.state("next")?.next(now).map(schema::annotate);
        }
        Ok(output)
    }

    fn dispatch(&mut self, command: Command, check: bool) -> Result<Output, AgentError> {
        match command {
            Command::Status => self.status(),
            Command::Next => self.next(),
            Command::UsageReport => self.usage(),
            Command::ReviewSchema => Ok(Output {
                events: Vec::new(),
                next: None,
                result: ResultData::ReviewSchema {
                    schema: schemars::schema_for!(domain::review::ReviewReport),
                },
            }),
            Command::ValidateReview { task, report } => self.validate_review(&task, report),
            Command::Discover {
                tasks,
                planning_order,
            } => self.discover(tasks, planning_order, check),
            Command::Refresh { changed, removed } => self.refresh(changed, removed, check),
            Command::Claim { task } => self.claim(task, check),
            Command::Hold { task, reason } => self.hold(task, reason, check),
            Command::Resume { task } => self.resume(task, check),
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
                progress::CheckpointInput {
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
            Command::Judge { task } => self.judge(task, check),
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
            Command::Grant { task, receipt } => self.grant(task, receipt, check),
            Command::OpenDelivery {
                task,
                kind,
                receipt,
                jira,
            } => self.open_delivery(task, kind, receipt, jira, check),
            Command::NarrowAcceptance {
                task,
                criteria,
                receipt,
            } => self.narrow(task, criteria, receipt, check),
            Command::RecoverOperation { target, terminate } => {
                self.recover_operation(target, terminate, check)
            }
            Command::Init { .. } => Err(self.error("init", None, ConflictReason::RunAlreadyExists)),
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
            next: None,
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
        let records = self
            .store
            .commit(Actor::Coordinator, at, &events)
            .map_err(|error| self.error(command, task, Rejection::Internal(error.to_string())))?;
        self.services.tracer.record(&mut self.store, &records);
        Ok(records)
    }

    /// Convenience for a handler with a single fallible step: the same
    /// per-command context `ctx` gives a handler with several, built and
    /// spent in one call.
    fn error(&self, failed: &str, task: Option<&TaskId>, why: impl Into<Rejection>) -> AgentError {
        self.ctx(failed, task).reject(why)
    }
}
