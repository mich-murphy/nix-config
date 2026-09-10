//! Snapshotting the current delivery's head: the events one snapshot
//! produces, shared by the standalone `snapshot` command (an
//! integration-only snapshot, with no implementer launch of its own) and
//! by `checkpoint`, which snapshots the turn it marks in the same
//! command so the coordinator never has to call both.

use crate::task_support::{
    current_work_delivery, latest_checkpointed_launch, sensitive_count, task_ref,
};
use crate::worktree::validate_paths;
use crate::{AgentError, App, ConflictReason, Output, Rejection};
use domain::{
    acceptance::Snapshot,
    event::Event,
    ids::{LaunchId, TaskId},
};

impl App<'_> {
    pub(super) fn snapshot(&mut self, task: TaskId, check: bool) -> Result<Output, AgentError> {
        let state = self.state("snapshot")?;
        let value = task_ref(&state, &task, "snapshot", self)?;
        let covers = latest_checkpointed_launch(&state, &task);
        let events = self.snapshot_events(&state, value, &task, covers)?;
        self.commit("snapshot", Some(&task), events, check)
    }

    /// The events a fresh snapshot of `task`'s current work produces:
    /// the `Snapshotted` record itself, a `ProofInvalidated` when the
    /// head moved since the last one, and a `TierRaised` when the diff's
    /// signals classify above the task's current tier. `covers` names the
    /// implementer launch this snapshot follows, or `None` for an
    /// integration-only snapshot.
    pub(super) fn snapshot_events(
        &self,
        state: &domain::state::State,
        value: &domain::task::Task,
        task: &TaskId,
        covers: Option<LaunchId>,
    ) -> Result<Vec<Event>, AgentError> {
        let (work, delivery) = current_work_delivery(value)
            .ok_or_else(|| self.error("snapshot", Some(task), ConflictReason::NoPlannedDelivery))?;
        let previous_head = work.snapshot.as_ref().map(|snapshot| snapshot.head.clone());
        let head = self.vcs_head(task, &work.branch)?;
        let paths = self
            .services
            .vcs
            .changed_paths(&work.base, &head)
            .map_err(|error| self.error("snapshot", Some(task), Rejection::External(error.0)))?;
        let lines = self
            .services
            .vcs
            .changed_lines(&work.base, &head)
            .map_err(|error| self.error("snapshot", Some(task), Rejection::External(error.0)))?;
        validate_paths(self, task, delivery, &work.base, &head)?;
        let head_changed = previous_head.is_some_and(|previous| previous != head);
        let mut events = vec![Event::Snapshotted {
            task: task.clone(),
            delivery: delivery.id,
            snapshot: Snapshot {
                base: work.base.clone(),
                head,
                requirements: value.spec.requirements.clone(),
            },
            lines,
            files: paths.len() as u32,
            covers,
        }];
        if head_changed {
            events.push(Event::ProofInvalidated {
                task: task.clone(),
                delivery: delivery.id,
                cause: "head changed".into(),
            });
        }
        let tier = domain::risk::classify(&domain::risk::Signals {
            files: paths.len() as u32,
            sensitive_paths: sensitive_count(state, &paths),
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
        });
        if tier > value.tier.current {
            events.push(Event::TierRaised {
                task: task.clone(),
                to: tier,
                reason: "computed diff signals".into(),
                at: self.services.clock.now(),
            });
        }
        Ok(events)
    }

    fn vcs_head(&self, task: &TaskId, branch: &str) -> Result<domain::ids::Sha, AgentError> {
        self.services
            .vcs
            .head(branch)
            .map_err(|error| self.error("snapshot", Some(task), Rejection::External(error.0)))
    }
}
