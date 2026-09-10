use crate::task_support::{
    committed_lessons, digest_json, next_delivery, selected_lessons, task_ref,
};
use crate::worktree::{bind_worktree, validate_slot};
use crate::{AgentError, App, ConflictReason, EvidenceError, Output, Rejection};
use domain::{
    acceptance,
    delivery::{Delivery, DeliveryKind, Outcome},
    event::Event,
    ids::{AuthorityId, CriterionId, IssueKey, JiraStatus, SlotId, TaskId, UseId},
    risk::Tier,
    task::{Criterion, Phase, Plan, SlotBinding, SlotOrigin, Subtask},
};
use std::path::PathBuf;

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
        let active = matches!(
            value.phase,
            Phase::Claimed | Phase::Planned | Phase::InFlight
        );
        if !active || criteria.is_empty() {
            return Err(self.error(
                "brief",
                Some(&task),
                ConflictReason::BriefRequiresClaimedTask,
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
        let changed = requirements != value.spec.requirements;
        let mut events = vec![Event::Briefed {
            task: task.clone(),
            criteria,
            requirements,
            tier,
            provisional: true,
        }];
        if changed {
            events.extend(
                value
                    .deliveries
                    .last()
                    .map(|delivery| Event::ProofInvalidated {
                        task: task.clone(),
                        delivery: delivery.id,
                        cause: "requirements changed".into(),
                    }),
            );
        }
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
        let verifying = matches!(value.phase, Phase::Merged { .. })
            && value
                .current_delivery()
                .is_some_and(|delivery| matches!(delivery.kind, DeliveryKind::Verification { .. }));
        if !matches!(value.phase, Phase::Planned | Phase::InFlight) && !verifying {
            return Err(self.error("plan", Some(&task), ConflictReason::PlanRequiresSlot));
        }
        let rewrites_fixed_baseline = value
            .baselines
            .iter()
            .any(|(criterion, digest)| plan.baselines.get(criterion) != Some(digest));
        if rewrites_fixed_baseline {
            return Err(self.error(
                "plan",
                Some(&task),
                EvidenceError::Other(
                    "replanning cannot change baselines fixed for this task".into(),
                ),
            ));
        }
        let feedback = self.plan_feedback(&state, &task, &plan.lesson_families)?;
        self.commit(
            "plan",
            Some(&task),
            vec![Event::Planned {
                task: task.clone(),
                plan,
                feedback,
            }],
            check,
        )
    }

    /// This task's own recorded lessons matching `families`, plus any
    /// accepted lessons `RunConfig.feedback_file` pins for them: cross-run
    /// guidance the coordinator can commit without replaying it through
    /// `lesson-record` for every run.
    fn plan_feedback(
        &self,
        state: &domain::state::State,
        task: &TaskId,
        families: &[String],
    ) -> Result<Vec<domain::command::Lesson>, AgentError> {
        let mut feedback = selected_lessons(state, families);
        let path = state
            .config
            .as_ref()
            .and_then(|config| config.feedback_file.as_ref());
        if let Some(path) = path {
            let committed = committed_lessons(path, families)
                .map_err(|message| self.error("plan", Some(task), Rejection::Invalid(message)))?;
            feedback.extend(committed);
        }
        Ok(feedback)
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
                ConflictReason::BindRequiresClaimedTask,
            ));
        }
        let reuse = validate_slot(&state, &task, &slot, authority)
            .map_err(|error| self.error("bind-slot", Some(&task), error))?;
        let base =
            self.services.vcs.head("origin/main").map_err(|error| {
                self.error("bind-slot", Some(&task), Rejection::External(error.0))
            })?;
        bind_worktree(self, &task, &slot, &branch, reuse, check)?;
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
            feedback: Vec::new(),
            snapshot_launch: None,
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
            human_review: None,
            check_deadlines: std::collections::BTreeMap::new(),
            prior_review: None,
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
        if let Some(authority) = authority {
            events.push(Event::GrantUsed {
                task: task.clone(),
                authority,
                by: UseId(next_delivery(value).0),
            });
        }
        if value.deliveries.is_empty() {
            events.push(Event::DeliveryOpened {
                task: task.clone(),
                delivery: Box::new(delivery),
            });
        }
        self.commit("bind-slot", Some(&task), events, check)
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
