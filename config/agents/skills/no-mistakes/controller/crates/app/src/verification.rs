//! Deliveries opened without a receipt. A task's first delivery may be a
//! `Verification` of a commit already on `origin/main` (typically work an
//! earlier run merged before this ledger existed): no code will be
//! written, so no authority is needed. A later delivery may reopen a held
//! task under the run config's standing authorization
//! (`follow_up_deliveries`), which the user granted at init; verification
//! follow-ups are always free for the same reason as the first.

use crate::delivery_support::{validate_history, validate_open};
use crate::task_support::{next_delivery, task_ref};
use crate::{AgentError, App, ConflictReason, Output, Rejection};
use domain::{
    acceptance,
    command::JiraRead,
    delivery::{Delivery, DeliveryKind, Outcome},
    event::Event,
    ids::{Sha, TaskId},
    task::{Phase, Task},
};

impl App<'_> {
    pub(super) fn open_without_receipt(
        &mut self,
        task: TaskId,
        kind: DeliveryKind,
        jira: JiraRead,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("open-delivery", Some(&task));
        let state = self.state("open-delivery")?;
        let value = task_ref(&state, &task, "open-delivery", self)?;
        if value.deliveries.is_empty() {
            return self.open_verification(task, kind, jira, check);
        }
        validate_open(value, &jira).map_err(|reason| ctx.reject(reason))?;
        domain::delivery::can_open(&value.deliveries).map_err(|error| ctx.reject(error))?;
        validate_history(self, value).map_err(|reason| ctx.reject(reason))?;
        standing_allows(&state, value, &kind).map_err(|reason| ctx.reject(reason))?;
        let base = self.follow_up_base(&ctx, &kind)?;
        let delivery = unscoped_delivery(value, kind, base);
        self.commit(
            "open-delivery",
            Some(&task),
            vec![
                Event::DeliveryOpened {
                    task: task.clone(),
                    delivery: Box::new(delivery),
                },
                Event::Resumed { task: task.clone() },
            ],
            check,
        )
    }

    /// Where a follow-up starts: the verified commit itself, or current
    /// `origin/main` for new code.
    fn follow_up_base(
        &self,
        ctx: &crate::error::Ctx,
        kind: &DeliveryKind,
    ) -> Result<Sha, AgentError> {
        match kind {
            DeliveryKind::Verification { of } => Ok(of.clone()),
            DeliveryKind::Code { .. } => self
                .services
                .vcs
                .head("origin/main")
                .map_err(|error| ctx.reject(Rejection::External(error.0))),
        }
    }

    pub(super) fn open_verification(
        &mut self,
        task: TaskId,
        kind: DeliveryKind,
        jira: JiraRead,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("open-delivery", Some(&task));
        let state = self.state("open-delivery")?;
        let value = task_ref(&state, &task, "open-delivery", self)?;
        let of = first_verification_target(value, &kind).map_err(|reason| ctx.reject(reason))?;
        validate_jira(value, &jira).map_err(|reason| ctx.reject(reason))?;
        let on_main = self
            .services
            .vcs
            .on_main(of)
            .map_err(|error| ctx.reject(Rejection::External(error.0)))?;
        if !on_main {
            return Err(ctx.reject(ConflictReason::VerificationCommitNotOnMain));
        }
        let delivery = unscoped_delivery(value, kind.clone(), of.clone());
        self.commit(
            "open-delivery",
            Some(&task),
            vec![Event::DeliveryOpened {
                task: task.clone(),
                delivery: Box::new(delivery),
            }],
            check,
        )
    }
}

/// Whether the run config's standing authorization still covers one more
/// code follow-up for this task. Verification follow-ups write no code
/// and are never counted.
fn standing_allows(
    state: &domain::state::State,
    task: &Task,
    kind: &DeliveryKind,
) -> Result<(), ConflictReason> {
    if matches!(kind, DeliveryKind::Verification { .. }) {
        return Ok(());
    }
    let used = task
        .deliveries
        .iter()
        .skip(1)
        .filter(|delivery| {
            delivery.authority.is_none() && matches!(delivery.kind, DeliveryKind::Code { .. })
        })
        .count() as u32;
    let allowed = state
        .config
        .as_ref()
        .map_or(0, |config| config.follow_up_deliveries);
    if used < allowed {
        Ok(())
    } else {
        Err(ConflictReason::ReceiptRequired)
    }
}

/// The commit a receipt-free `open-delivery` verifies: only a task's
/// first delivery, only after `brief`, and only as `Verification`. Any
/// other shape is a later or code delivery and needs a receipt.
fn first_verification_target<'a>(
    task: &Task,
    kind: &'a DeliveryKind,
) -> Result<&'a Sha, ConflictReason> {
    let DeliveryKind::Verification { of } = kind else {
        return Err(ConflictReason::ReceiptRequired);
    };
    let first = task.deliveries.is_empty()
        && matches!(task.phase, Phase::Claimed)
        && !task.spec.criteria.is_empty();
    if first {
        Ok(of)
    } else {
        Err(ConflictReason::ReceiptRequired)
    }
}

fn validate_jira(task: &Task, jira: &JiraRead) -> Result<(), ConflictReason> {
    if !jira.member || jira.resolved || !jira.ownership_clear {
        return Err(ConflictReason::JiraOwnershipUnresolved);
    }
    if jira.requirements != task.spec.requirements {
        return Err(ConflictReason::CriteriaChanged);
    }
    Ok(())
}

/// A delivery with no authority behind it: full criteria, no path scope,
/// no worktree yet. `bind-slot` attaches the worktree for a code delivery.
fn unscoped_delivery(task: &Task, kind: DeliveryKind, base: Sha) -> Delivery {
    Delivery {
        id: next_delivery(task),
        kind,
        authority: None,
        criteria: task
            .spec
            .criteria
            .iter()
            .map(|criterion| criterion.id.clone())
            .collect(),
        paths: None,
        base,
        work: None,
        proof: acceptance::Proof::default(),
        review: None,
        prior_review: None,
        human_review: None,
        check_deadlines: std::collections::BTreeMap::new(),
        outcome: Outcome::Open,
        launches: Vec::new(),
        operations: Vec::new(),
    }
}
