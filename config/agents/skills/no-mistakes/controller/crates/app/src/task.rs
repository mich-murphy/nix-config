use crate::task_support::{
    bind_worktree, current_work_delivery, digest_json, next_delivery, sensitive_count, task_ref,
    validate_paths, validate_slot,
};
use crate::{AgentError, App, Output, Rejection};
use domain::{
    acceptance::{self, Snapshot},
    budget::BudgetKind,
    command::AgentRole,
    delivery::{Delivery, DeliveryKind, Outcome},
    event::Event,
    ids::{AuthorityId, CriterionId, IssueKey, JiraStatus, SlotId, TaskId},
    risk::Tier,
    task::{Criterion, Phase, Plan, SlotBinding, SlotOrigin, Subtask},
};
use std::path::PathBuf;

pub(super) struct CheckpointInput {
    pub advanced: bool,
    pub observation: String,
    pub next: String,
    pub outside_paths: Vec<String>,
    pub scope_reason: Option<String>,
}

pub(super) struct SubtaskInput {
    pub issue: IssueKey,
    pub criteria: Vec<CriterionId>,
    pub owned: bool,
    pub was_terminal: bool,
    pub status: JiraStatus,
}

pub(super) struct CheckInput {
    pub criteria: Vec<CriterionId>,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub timeout_seconds: u64,
    pub implementation: Vec<String>,
}

impl App<'_> {
    pub(super) fn brief(
        &mut self,
        task: TaskId,
        criteria: Vec<Criterion>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("brief")?;
        let value = task_ref(&state, &task, "brief", self)?;
        if !matches!(value.phase, Phase::Claimed) || criteria.is_empty() {
            return Err(self.error(
                "brief",
                Some(&task),
                Rejection::Conflict("brief requires a claimed task and criteria".into()),
            ));
        }
        let requirements = digest_json(&criteria)
            .map_err(|message| self.error("brief", Some(&task), Rejection::Internal(message)))?;
        let signals = domain::risk::Signals {
            human_only_criteria: criteria
                .iter()
                .filter(|criterion| criterion.human_only)
                .count() as u32,
            dependencies: value.spec.dependencies.len() as u32,
            ..domain::risk::Signals::default()
        };
        let tier = domain::risk::classify(&signals);
        let events = vec![Event::Briefed {
            task: task.clone(),
            criteria,
            requirements,
            tier,
            provisional: true,
        }];
        self.commit("brief", Some(&task), events, check)
    }

    pub(super) fn plan(
        &mut self,
        task: TaskId,
        plan: Plan,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("plan")?;
        let value = task_ref(&state, &task, "plan", self)?;
        let old = match &value.phase {
            Phase::Planned { work } | Phase::InFlight { work, .. } => work.plan.as_ref(),
            _ => {
                return Err(self.error(
                    "plan",
                    Some(&task),
                    Rejection::Conflict("bind a slot before planning".into()),
                ));
            }
        };
        if old.is_some_and(|old| old.baselines != plan.baselines) {
            return Err(self.error(
                "plan",
                Some(&task),
                Rejection::Evidence("replanning cannot change baselines".into()),
            ));
        }
        self.commit(
            "plan",
            Some(&task),
            vec![Event::Planned {
                task: task.clone(),
                plan,
            }],
            check,
        )
    }

    pub(super) fn checkpoint(
        &mut self,
        task: TaskId,
        input: CheckpointInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("checkpoint")?;
        let value = task_ref(&state, &task, "checkpoint", self)?;
        let launch = state
            .launches
            .iter()
            .rev()
            .find(|launch| {
                launch.task == task
                    && launch.role == AgentRole::Implementer
                    && launch.session.is_some()
            })
            .map(|launch| launch.id)
            .ok_or_else(|| {
                self.error(
                    "checkpoint",
                    Some(&task),
                    Rejection::Conflict("checkpoint requires a terminal implementer turn".into()),
                )
            })?;
        if state.checkpoints.get(&task) == Some(&launch) {
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
        let stalled = if input.advanced {
            0
        } else {
            value.budgets.stalled_checkpoints.saturating_add(1)
        };
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

    pub(super) fn bind_slot(
        &mut self,
        task: TaskId,
        slot: SlotId,
        branch: String,
        authority: Option<AuthorityId>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("bind-slot")?;
        let value = task_ref(&state, &task, "bind-slot", self)?;
        if !matches!(value.phase, Phase::Claimed) || !branch.contains(task.as_ref()) {
            return Err(self.error(
                "bind-slot",
                Some(&task),
                Rejection::Conflict("bind requires the claimed task and a task branch".into()),
            ));
        }
        validate_slot(&state, &task, &slot, authority).map_err(|message| {
            self.error("bind-slot", Some(&task), Rejection::Authority(message))
        })?;
        let base =
            self.services.vcs.head("origin/main").map_err(|error| {
                self.error("bind-slot", Some(&task), Rejection::External(error.0))
            })?;
        bind_worktree(self, &task, &slot, &branch, check)?;
        let binding = SlotBinding {
            slot,
            origin: authority.map_or(SlotOrigin::Fresh, |authority| SlotOrigin::Reused {
                authority,
            }),
        };
        let work = domain::task::PlannedWork {
            slot: binding.clone(),
            branch: branch.clone(),
            base: base.clone(),
            snapshot: None,
            plan: None,
        };
        let delivery = Delivery {
            id: next_delivery(value),
            kind: DeliveryKind::Code { pr: None },
            authority: None,
            criteria: value
                .spec
                .criteria
                .iter()
                .map(|criterion| criterion.id.clone())
                .collect(),
            paths: None,
            base: base.clone(),
            work: Some(work),
            proof: acceptance::Proof::default(),
            review: None,
            outcome: Outcome::Open,
            launches: Vec::new(),
            operations: Vec::new(),
        };
        let mut events = vec![Event::SlotBound {
            task: task.clone(),
            binding,
            branch,
            base,
        }];
        if value.deliveries.is_empty() {
            events.push(Event::DeliveryOpened {
                task: task.clone(),
                delivery: Box::new(delivery),
            });
        }
        self.commit("bind-slot", Some(&task), events, check)
    }

    pub(super) fn snapshot(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("snapshot")?;
        let value = task_ref(&state, &task, "snapshot", self)?;
        let (work, delivery) = current_work_delivery(value).ok_or_else(|| {
            self.error(
                "snapshot",
                Some(&task),
                Rejection::Conflict("task has no planned delivery".into()),
            )
        })?;
        let head =
            self.services.vcs.head(&work.branch).map_err(|error| {
                self.error("snapshot", Some(&task), Rejection::External(error.0))
            })?;
        let paths = self
            .services
            .vcs
            .changed_paths(&work.base, &head)
            .map_err(|error| self.error("snapshot", Some(&task), Rejection::External(error.0)))?;
        let lines = self
            .services
            .vcs
            .changed_lines(&work.base, &head)
            .map_err(|error| self.error("snapshot", Some(&task), Rejection::External(error.0)))?;
        validate_paths(self, &task, delivery, &work.base, &head)?;
        let snapshot = Snapshot {
            base: work.base.clone(),
            head,
            requirements: value.spec.requirements.clone(),
        };
        let mut events = vec![Event::Snapshotted {
            task: task.clone(),
            delivery: delivery.id,
            snapshot,
            lines,
            files: paths.len() as u32,
        }];
        let signals = domain::risk::Signals {
            files: paths.len() as u32,
            sensitive_paths: sensitive_count(&state, &paths),
            manifest_or_lockfile: paths.iter().any(|path| {
                path.ends_with("Cargo.toml")
                    || path.ends_with("Cargo.lock")
                    || path.ends_with("package.json")
            }),
            human_only_criteria: value
                .spec
                .criteria
                .iter()
                .filter(|criterion| criterion.human_only)
                .count() as u32,
            dependencies: value.spec.dependencies.len() as u32,
            lines,
        };
        let tier = domain::risk::classify(&signals);
        if tier > value.tier.current {
            events.push(Event::TierRaised {
                task: task.clone(),
                to: tier,
                reason: "computed diff signals".into(),
                at: self.services.clock.now(),
            });
        }
        self.commit("snapshot", Some(&task), events, check)
    }

    pub(super) fn subtask_record(
        &mut self,
        task: TaskId,
        input: SubtaskInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("subtask-record")?;
        task_ref(&state, &task, "subtask-record", self)?;
        let subtask = Subtask {
            criteria: input.criteria,
            owned: input.owned,
            was_terminal: input.was_terminal,
            status: input.status,
            sync: domain::sync::Sync::Pending,
        };
        self.commit(
            "subtask-record",
            Some(&task),
            vec![Event::SubtaskRecorded {
                task: task.clone(),
                issue: input.issue,
                subtask,
            }],
            check,
        )
    }

    pub(super) fn escalate_tier(
        &mut self,
        task: TaskId,
        to: Tier,
        reason: String,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("escalate-tier")?;
        let value = task_ref(&state, &task, "escalate-tier", self)?;
        if reason.trim().is_empty() || to <= value.tier.current {
            return Err(self.error(
                "escalate-tier",
                Some(&task),
                Rejection::Invalid("tier escalation must raise the tier with a reason".into()),
            ));
        }
        self.commit(
            "escalate-tier",
            Some(&task),
            vec![Event::TierRaised {
                task: task.clone(),
                to,
                reason,
                at: self.services.clock.now(),
            }],
            check,
        )
    }
}
