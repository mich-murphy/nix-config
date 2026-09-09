//! Grant handling: registering a fresh `Authority` from the coordinator's
//! `AuthorityReceipt`, repurposing an unused pair as the next delivery's
//! grant, and opening or narrowing a delivery against one. The controller
//! owns `id`, `recorded` and `used`; the coordinator never supplies them
//! (design Section 5, batch B item 4).

use crate::delivery_support::{validate_history, validate_open};
use crate::error::Ctx;
use crate::task_support::{next_authority, next_delivery, task_ref};
use crate::{AgentError, App, ConflictReason, Output, Rejection};
use domain::{
    acceptance,
    authority::{self, Authority, AuthorityError, AuthorityReceipt, AuthorityUse, Grant},
    command::JiraRead,
    delivery::{Delivery, DeliveryKind, Outcome},
    ids::{CriterionId, TaskId, UseId},
    task::Task,
};

impl App<'_> {
    pub(super) fn grant(
        &mut self,
        task: TaskId,
        receipt: AuthorityReceipt,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("grant", Some(&task));
        let state = self.state("grant")?;
        let value = task_ref(&state, &task, "grant", self)?;
        let idle = !state
            .launches
            .iter()
            .any(|launch| launch.task == task && launch.outcome.is_none())
            && !state.operations.iter().any(|operation| {
                operation.task == task
                    && matches!(
                        operation.status,
                        domain::event::OperationStatus::Running
                            | domain::event::OperationStatus::Unknown
                    )
            });
        if matches!(receipt.grant, Grant::Pair { .. }) && value.hold.is_none() {
            return Err(ctx.reject(AuthorityError::PairRequiresHold));
        }
        let authority = fresh_authority(self, value, receipt);
        authority::register(&value.authorities, &authority, idle)
            .map_err(|error| ctx.reject(error))?;
        validate_authority_file(&authority).map_err(|error| ctx.reject(error))?;
        validate_grant_paths(&ctx, self, &authority)?;
        if authority.requirements != value.spec.requirements {
            return Err(ctx.reject(AuthorityError::Requirements));
        }
        self.commit(
            "grant",
            Some(&task),
            vec![domain::event::Event::AuthorityRegistered {
                task: task.clone(),
                authority: Box::new(authority),
            }],
            check,
        )
    }

    pub(super) fn open_delivery(
        &mut self,
        task: TaskId,
        kind: DeliveryKind,
        receipt: AuthorityReceipt,
        jira: JiraRead,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("open-delivery", Some(&task));
        let state = self.state("open-delivery")?;
        let value = task_ref(&state, &task, "open-delivery", self)?;
        validate_open(value, &jira).map_err(|reason| ctx.reject(reason))?;
        domain::delivery::can_open(&value.deliveries).map_err(|error| ctx.reject(error))?;
        let (authority, repurposed) = resolved_authority(self, value, &receipt);
        validate_open_authority(&ctx, value, &kind, &authority, repurposed)?;
        validate_history(self, value).map_err(|reason| ctx.reject(reason))?;
        let delivery = new_delivery(self, &ctx, value, kind, &authority)?;
        let events = open_events(&task, authority, delivery, repurposed);
        self.commit("open-delivery", Some(&task), events, check)
    }

    pub(super) fn narrow(
        &mut self,
        task: TaskId,
        criteria: Vec<CriterionId>,
        receipt: AuthorityReceipt,
        check: bool,
    ) -> Result<Output, AgentError> {
        let ctx = self.ctx("narrow-acceptance", Some(&task));
        let state = self.state("narrow-acceptance")?;
        let value = task_ref(&state, &task, "narrow-acceptance", self)?;
        let delivery = value
            .deliveries
            .last()
            .ok_or_else(|| ctx.reject(ConflictReason::NoDelivery))?;
        domain::delivery::can_narrow(delivery, &criteria).map_err(|error| ctx.reject(error))?;
        if overlaps_full_criteria(value, &criteria) {
            return Err(ctx.reject(Rejection::Invalid(
                "narrowed criterion IDs must be distinct".into(),
            )));
        }
        let authority = fresh_authority(self, value, receipt);
        validate_narrowing_authority(&ctx, value, &criteria, &authority)?;
        let events = vec![
            domain::event::Event::AuthorityRegistered {
                task: task.clone(),
                authority: Box::new(authority.clone()),
            },
            domain::event::Event::GrantUsed {
                task: task.clone(),
                authority: authority.id,
                by: UseId(delivery.id.0),
            },
            domain::event::Event::AcceptanceNarrowed {
                task: task.clone(),
                delivery: delivery.id,
                criteria,
                authority: authority.id,
            },
        ];
        self.commit("narrow-acceptance", Some(&task), events, check)
    }
}

/// Builds a brand-new `Authority` from a receipt: the controller assigns
/// `id` from the task's own history, records `recorded` from its clock,
/// and starts `used` at its default.
fn fresh_authority(app: &App<'_>, task: &Task, receipt: AuthorityReceipt) -> Authority {
    Authority {
        id: next_authority(task),
        source: receipt.source,
        artifact: receipt.artifact,
        digest: receipt.digest,
        requirements: receipt.requirements,
        recorded: app.services.clock.now(),
        grant: receipt.grant,
        used: AuthorityUse::default(),
    }
}

/// A receipt naming an unused `Pair` grant repurposes it as the next
/// delivery's grant (design Section 5) rather than registering a second,
/// duplicate authority under the same digest: the returned `bool` is
/// `true` exactly when the caller must emit `AuthorityRepurposed` instead
/// of `AuthorityRegistered`. The repurposed authority keeps its original
/// `id`, `recorded` and `used`; only `grant` changes.
fn resolved_authority(app: &App<'_>, task: &Task, receipt: &AuthorityReceipt) -> (Authority, bool) {
    let existing = task
        .authorities
        .iter()
        .find(|entry| entry.digest == receipt.digest && authority::unused_pair(entry));
    match existing {
        Some(entry) => {
            let mut updated = entry.clone();
            updated.grant = receipt.grant.clone();
            (updated, true)
        }
        None => (fresh_authority(app, task, receipt.clone()), false),
    }
}

fn validate_authority_file(authority: &Authority) -> Result<(), Rejection> {
    let digest =
        crate::task_support::digest_file(&authority.artifact).map_err(Rejection::External)?;
    authority::validate_receipt(authority, &digest)?;
    Ok(())
}

fn validate_grant_paths(ctx: &Ctx, app: &App<'_>, authority: &Authority) -> Result<(), AgentError> {
    let Some(paths) = authority::paths(&authority.grant) else {
        return Ok(());
    };
    let repo = app
        .state("grant")?
        .config
        .ok_or_else(|| ctx.reject(Rejection::Internal("config missing".into())))?
        .repo;
    let valid = paths.iter().all(|path| {
        let file = repo.join(path);
        file.is_file() && !file.is_symlink()
    });
    if valid {
        Ok(())
    } else {
        Err(ctx.reject(AuthorityError::MissingMarkdownFile))
    }
}

fn validate_delivery_grant(
    authority: &Authority,
    kind: &DeliveryKind,
) -> Result<(), AuthorityError> {
    match &authority.grant {
        Grant::Delivery {
            kind: granted,
            criteria,
            ..
        } if std::mem::discriminant(granted) == std::mem::discriminant(kind)
            && !criteria.is_empty() =>
        {
            Ok(())
        }
        _ => Err(AuthorityError::DeliveryGrantMismatch),
    }
}

fn validate_open_authority(
    ctx: &Ctx,
    task: &Task,
    kind: &DeliveryKind,
    authority: &Authority,
    repurposed: bool,
) -> Result<(), AgentError> {
    if !repurposed {
        authority::register(&task.authorities, authority, true)
            .map_err(|error| ctx.reject(error))?;
    }
    validate_authority_file(authority).map_err(|error| ctx.reject(error))?;
    validate_delivery_grant(authority, kind).map_err(|error| ctx.reject(error))?;
    authority::validate_use(authority, &task.spec.requirements).map_err(|error| ctx.reject(error))
}

fn new_delivery(
    app: &App<'_>,
    ctx: &Ctx,
    task: &Task,
    kind: DeliveryKind,
    authority: &Authority,
) -> Result<Delivery, AgentError> {
    let base = app
        .services
        .vcs
        .head("origin/main")
        .map_err(|error| ctx.reject(Rejection::External(error.0)))?;
    let criteria = match &authority.grant {
        Grant::Delivery { criteria, .. } => criteria.clone(),
        _ => Vec::new(),
    };
    Ok(Delivery {
        id: next_delivery(task),
        kind,
        authority: Some(authority.id),
        criteria,
        paths: authority::paths(&authority.grant).map(<[String]>::to_vec),
        base,
        work: None,
        proof: acceptance::Proof::default(),
        review: None,
        human_review: None,
        check_deadlines: std::collections::BTreeMap::new(),
        outcome: Outcome::Open,
        launches: Vec::new(),
        operations: Vec::new(),
    })
}

fn open_events(
    task: &TaskId,
    authority: Authority,
    delivery: Delivery,
    repurposed: bool,
) -> Vec<domain::event::Event> {
    let mut events = Vec::new();
    if repurposed {
        events.push(domain::event::Event::AuthorityRepurposed {
            task: task.clone(),
            authority: authority.id,
            grant: authority.grant.clone(),
        });
    } else {
        events.push(domain::event::Event::AuthorityRegistered {
            task: task.clone(),
            authority: Box::new(authority.clone()),
        });
    }
    let delivery_id = delivery.id;
    events.extend([
        domain::event::Event::DeliveryOpened {
            task: task.clone(),
            delivery: Box::new(delivery),
        },
        domain::event::Event::GrantUsed {
            task: task.clone(),
            authority: authority.id,
            by: UseId(delivery_id.0),
        },
        domain::event::Event::Resumed { task: task.clone() },
    ]);
    events
}

fn validate_narrowing_authority(
    ctx: &Ctx,
    task: &Task,
    criteria: &[CriterionId],
    authority: &Authority,
) -> Result<(), AgentError> {
    authority::register(&task.authorities, authority, true).map_err(|error| ctx.reject(error))?;
    validate_authority_file(authority).map_err(|error| ctx.reject(error))?;
    let matches = authority.requirements == task.spec.requirements
        && matches!(&authority.grant, Grant::Narrowing { criteria: granted, .. } if granted == criteria);
    if matches {
        Ok(())
    } else {
        Err(ctx.reject(AuthorityError::NarrowingMismatch))
    }
}

fn overlaps_full_criteria(task: &Task, criteria: &[CriterionId]) -> bool {
    criteria.iter().any(|id| {
        task.spec
            .criteria
            .iter()
            .any(|criterion| criterion.id == *id)
    })
}
