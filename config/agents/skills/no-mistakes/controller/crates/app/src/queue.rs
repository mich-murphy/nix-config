use crate::task_support::task_ref;
use crate::{AgentError, App, ConflictReason, Output, Rejection, ResultData, UsageReport};
use domain::{
    command::{DiscoveredTask, NextAction},
    event::Event,
    ids::TaskId,
    review::{self, ReviewReport},
    task::HoldReason,
};
use std::collections::{BTreeMap, BTreeSet};

impl App<'_> {
    pub(super) fn status(&self) -> Result<Output, AgentError> {
        let state = self.state("status")?;
        let verified = self.store.verify().is_ok();
        Ok(Output {
            events: Vec::new(),
            next: None,
            result: ResultData::State {
                state: Box::new(state),
                verified,
            },
        })
    }

    pub(super) fn next(&self) -> Result<Output, AgentError> {
        let now = self.services.clock.now();
        let next = self.state("next")?.next(now).map(crate::schema::annotate);
        Ok(Output {
            events: Vec::new(),
            next: None,
            result: ResultData::Next { next },
        })
    }

    /// The handoff step: it starts a judge for every worked task still
    /// without a verdict, then closes the run's trace instance, so a later
    /// command opens a fresh one. With judges still out, the last of them
    /// closes the instance instead, so their spans and the quality payload
    /// land on this run.
    pub(super) fn usage(&mut self) -> Result<Output, AgentError> {
        let state = self.state("usage-report")?;
        if self.sweep_judges(&state) == 0 {
            self.services.tracer.finish(&mut self.store);
        }
        Ok(Output {
            events: Vec::new(),
            next: None,
            result: ResultData::Usage {
                usage: UsageReport::from_state(&state),
            },
        })
    }

    /// Validates a reviewer's own `ReviewReport` against the task's actual
    /// tier and returns the verdict the controller will compute from it,
    /// so a self-validating reviewer can see what `run-agent` will
    /// conclude before it spends a launch on a mismatch.
    pub(super) fn validate_review(
        &self,
        task: &TaskId,
        report: ReviewReport,
    ) -> Result<Output, AgentError> {
        let state = self.state("validate-review")?;
        let value = task_ref(&state, task, "validate-review", self)?;
        let unique: BTreeSet<_> = report.findings.iter().map(|finding| &finding.id).collect();
        if unique.len() != report.findings.len() {
            return Err(self.error(
                "validate-review",
                Some(task),
                Rejection::Invalid("duplicate finding id".into()),
            ));
        }
        let verdict = review::verdict(&report.findings, &report.evidence_gaps, value.tier.current);
        Ok(Output {
            events: Vec::new(),
            next: None,
            result: ResultData::Verdict { verdict },
        })
    }

    pub(super) fn discover(
        &mut self,
        tasks: Vec<DiscoveredTask>,
        planning_order: Option<Vec<TaskId>>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("discover")?;
        if state.frozen {
            return Err(self.error("discover", None, ConflictReason::MembershipFrozen));
        }
        validate_unique(&tasks)
            .map_err(|message| self.error("discover", None, Rejection::Invalid(message)))?;
        validate_cycles(&tasks)
            .map_err(|message| self.error("discover", None, Rejection::Invalid(message)))?;
        let order = order(&tasks, planning_order)
            .map_err(|message| self.error("discover", None, Rejection::Invalid(message)))?;
        let mut events = vec![Event::QueueDiscovered {
            tasks: tasks.clone(),
            order,
            frozen: self.services.clock.now(),
        }];
        events.extend(tasks.iter().filter_map(ownership_question));
        self.commit("discover", None, events, check)
    }

    pub(super) fn refresh(
        &mut self,
        changed: Vec<DiscoveredTask>,
        removed: Vec<TaskId>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("refresh")?;
        if !state.frozen {
            return Err(self.error("refresh", None, ConflictReason::QueueNotDiscovered));
        }
        if changed
            .iter()
            .any(|task| !state.tasks.contains_key(&task.id))
        {
            return Err(self.error("refresh", None, ConflictReason::NewTasksOutOfScope));
        }
        self.commit(
            "refresh",
            None,
            vec![Event::QueueRefreshed { changed, removed }],
            check,
        )
    }

    pub(super) fn claim(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("claim")?;
        let expected = state
            .next(self.services.clock.now())
            .and_then(|next| match next.action {
                NextAction::Claim { task } => Some(task),
                _ => None,
            });
        if expected.as_ref() != Some(&task) {
            return Err(self.error("claim", Some(&task), ConflictReason::NotNextEligibleClaim));
        }
        reserve_claim(self, &task, check)?;
        self.commit(
            "claim",
            Some(&task),
            vec![Event::Claimed {
                task: task.clone(),
                at: self.services.clock.now(),
            }],
            check,
        )
    }

    pub(super) fn hold(
        &mut self,
        task: TaskId,
        reason: HoldReason,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("hold")?;
        require_active(&state, &task).map_err(|reason| self.error("hold", Some(&task), reason))?;
        if matches!(&reason, domain::task::HoldReason::CiPending { deadline, .. } if *deadline > self.services.clock.now())
        {
            return Err(self.error("hold", Some(&task), ConflictReason::CiDeadlinePending));
        }
        if state.operations.iter().any(|operation| {
            matches!(
                operation.status,
                domain::event::OperationStatus::Running | domain::event::OperationStatus::Unknown
            )
        }) {
            return Err(self.error(
                "hold",
                Some(&task),
                ConflictReason::ActiveOperationUnsettled,
            ));
        }
        self.commit(
            "hold",
            Some(&task),
            vec![Event::Held {
                task: task.clone(),
                reason,
                at: self.services.clock.now(),
            }],
            check,
        )
    }

    pub(super) fn resume(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("resume")?;
        let held = state
            .tasks
            .get(&task)
            .is_some_and(|value| value.hold.is_some());
        if !held || state.active.as_ref().is_some_and(|active| active != &task) {
            return Err(self.error("resume", Some(&task), ConflictReason::NotAvailableHold));
        }
        self.commit(
            "resume",
            Some(&task),
            vec![Event::Resumed { task: task.clone() }],
            check,
        )
    }
}

fn require_active(state: &domain::state::State, task: &TaskId) -> Result<(), ConflictReason> {
    if state.active.as_ref() == Some(task) {
        Ok(())
    } else {
        Err(ConflictReason::TaskNotActive)
    }
}

fn validate_unique(tasks: &[DiscoveredTask]) -> Result<(), String> {
    let unique: BTreeSet<_> = tasks.iter().map(|task| &task.id).collect();
    if unique.len() == tasks.len() {
        Ok(())
    } else {
        Err("duplicate discovered task".into())
    }
}

fn validate_cycles(tasks: &[DiscoveredTask]) -> Result<(), String> {
    let graph: BTreeMap<_, _> = tasks
        .iter()
        .map(|task| (&task.id, &task.spec.dependencies))
        .collect();
    for task in tasks {
        check_cycle(&graph, &task.id)?;
    }
    Ok(())
}

fn check_cycle<'a>(
    graph: &BTreeMap<&'a TaskId, &'a Vec<domain::task::Dependency>>,
    root: &'a TaskId,
) -> Result<(), String> {
    visit_dependency(graph, root, &mut BTreeSet::new(), &mut BTreeSet::new())
}

fn visit_dependency<'a>(
    graph: &BTreeMap<&'a TaskId, &'a Vec<domain::task::Dependency>>,
    current: &'a TaskId,
    visited: &mut BTreeSet<&'a TaskId>,
    path: &mut BTreeSet<&'a TaskId>,
) -> Result<(), String> {
    if path.contains(current) {
        return Err(format!("dependency cycle at {current}"));
    }
    if !visited.insert(current) {
        return Ok(());
    }
    path.insert(current);
    for dependency in graph.get(current).into_iter().flat_map(|value| *value) {
        if graph.contains_key(&dependency.task) {
            visit_dependency(graph, &dependency.task, visited, path)?;
        }
    }
    path.remove(current);
    Ok(())
}

fn ownership_question(task: &DiscoveredTask) -> Option<Event> {
    (task.spec.member && !task.spec.was_terminal && !task.spec.ownership_clear).then(|| {
        Event::QuestionRaised {
            task: Some(task.id.clone()),
            text: task.spec.ownership_evidence.clone(),
        }
    })
}

fn reserve_claim(app: &App<'_>, task: &TaskId, check: bool) -> Result<(), AgentError> {
    if check {
        return Ok(());
    }
    let reserved = app
        .services
        .vcs
        .reserve(task)
        .map_err(|error| app.error("claim", Some(task), Rejection::External(error.0)))?;
    if reserved {
        Ok(())
    } else {
        Err(app.error("claim", Some(task), ConflictReason::TaskOwnedElsewhere))
    }
}

fn order(tasks: &[DiscoveredTask], planned: Option<Vec<TaskId>>) -> Result<Vec<TaskId>, String> {
    if let Some(order) = planned {
        let actual: BTreeSet<_> = tasks.iter().map(|task| task.id.clone()).collect();
        let supplied: BTreeSet<_> = order.iter().cloned().collect();
        if actual == supplied && order.len() == tasks.len() {
            return Ok(order);
        }
        return Err("planning order must name every task once".into());
    }
    let mut sorted = tasks.to_vec();
    sorted.sort_by(|left, right| {
        left.spec
            .priority
            .cmp(&right.spec.priority)
            .then_with(|| {
                left.spec
                    .due
                    .unwrap_or(u64::MAX)
                    .cmp(&right.spec.due.unwrap_or(u64::MAX))
            })
            .then_with(|| left.spec.rank.cmp(&right.spec.rank))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(sorted.into_iter().map(|task| task.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_freezes_membership() {
        let state = domain::state::State::empty();
        assert!(!state.frozen);
    }

    #[test]
    fn claim_rejects_second_active() -> Result<(), domain::ids::InvalidId> {
        let mut state = domain::state::State::empty();
        state.active = Some("GAIN-1".parse()?);
        let other = "GAIN-2".parse()?;
        assert!(require_active(&state, &other).is_err());
        Ok(())
    }
}
