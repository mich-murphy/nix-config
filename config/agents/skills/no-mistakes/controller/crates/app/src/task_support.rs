use crate::{AgentError, App, Rejection};
use domain::{
    acceptance::Snapshot,
    authority::{self, Grant},
    budget::BudgetKind,
    command::{AgentRole, Fallback},
    delivery::{Delivery, DeliveryKind},
    ids::{AuthorityId, DeliveryId, Digest, LaunchId, OperationId, SlotId, TaskId},
    risk::{Assignment, Tier},
    task::Phase,
};
use sha2::{Digest as _, Sha256};
use std::fs;

pub(super) fn task_ref<'a>(
    state: &'a domain::state::State,
    task: &TaskId,
    command: &str,
    app: &App<'_>,
) -> Result<&'a domain::task::Task, AgentError> {
    state.tasks.get(task).ok_or_else(|| {
        app.error(
            command,
            Some(task),
            Rejection::Invalid("unknown task".into()),
        )
    })
}

pub(super) fn next_delivery(task: &domain::task::Task) -> DeliveryId {
    DeliveryId(
        task.deliveries
            .iter()
            .map(|delivery| delivery.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

pub(super) fn next_launch(state: &domain::state::State) -> LaunchId {
    LaunchId(
        state
            .launches
            .iter()
            .map(|launch| launch.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

pub(super) fn next_operation(state: &domain::state::State) -> OperationId {
    OperationId(
        state
            .operations
            .iter()
            .map(|operation| operation.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

pub(super) fn current_work_delivery(
    task: &domain::task::Task,
) -> Option<(&domain::task::PlannedWork, &Delivery)> {
    let work = match &task.phase {
        Phase::Planned { work } | Phase::InFlight { work, .. } => work,
        _ => return None,
    };
    Some((work, task.deliveries.last()?))
}

pub(super) fn current_snapshot(task: &domain::task::Task) -> Option<&Snapshot> {
    current_work_delivery(task).and_then(|(work, _)| work.snapshot.as_ref())
}

pub(super) fn review_target(task: &domain::task::Task, delivery: &Delivery) -> Option<Snapshot> {
    current_snapshot(task)
        .cloned()
        .or_else(|| match &delivery.kind {
            DeliveryKind::Verification { of } => Some(Snapshot {
                base: of.clone(),
                head: of.clone(),
                requirements: task.spec.requirements.clone(),
            }),
            DeliveryKind::Code { .. } => None,
        })
}

pub(super) fn validate_slot(
    state: &domain::state::State,
    task: &TaskId,
    slot: &SlotId,
    authority: Option<AuthorityId>,
) -> Result<(), String> {
    let historical = state.tasks.values().any(|value| {
        value.id != *task
            && value.deliveries.iter().any(|delivery| {
                delivery
                    .work
                    .as_ref()
                    .is_some_and(|work| work.slot.slot == *slot)
            })
    });
    if historical && authority.is_none() {
        return Err("historical slot needs a slot-reuse grant".into());
    }
    if let Some(id) = authority {
        let value = state.tasks.get(task).ok_or("unknown task")?;
        let grant = value
            .authorities
            .iter()
            .find(|entry| entry.id == id)
            .ok_or("unknown authority")?;
        if !matches!(&grant.grant, Grant::SlotReuse { slot: granted, .. } if granted == slot) {
            return Err("slot-reuse grant does not match".into());
        }
    }
    Ok(())
}

pub(super) fn sensitive_count(state: &domain::state::State, paths: &[String]) -> u32 {
    state.config.as_ref().map_or(0, |config| {
        paths
            .iter()
            .filter(|path| {
                config
                    .risk
                    .sensitive
                    .iter()
                    .any(|pattern| glob(pattern, path))
            })
            .count() as u32
    })
}

pub(super) fn glob(pattern: &str, path: &str) -> bool {
    pattern == path
        || pattern
            .strip_suffix("/**")
            .is_some_and(|prefix| path.starts_with(prefix))
        || pattern
            .strip_prefix("**/")
            .is_some_and(|suffix| path.ends_with(suffix))
}

pub(super) fn digest_json(value: &impl serde::Serialize) -> Result<Digest, String> {
    digest_bytes(&serde_json::to_vec(value).map_err(|error| error.to_string())?)
}

pub(super) fn digest_file(path: &std::path::Path) -> Result<Digest, String> {
    digest_bytes(&fs::read(path).map_err(|error| error.to_string())?)
}

pub(super) fn digest_bytes(bytes: &[u8]) -> Result<Digest, String> {
    use std::str::FromStr;
    Digest::from_str(&format!("{:x}", Sha256::digest(bytes))).map_err(|error| error.to_string())
}

pub(super) fn guard_role(
    task: &domain::task::Task,
    delivery: &Delivery,
    role: AgentRole,
) -> Result<(), String> {
    let verification = matches!(delivery.kind, DeliveryKind::Verification { .. });
    if role == AgentRole::Implementer && verification {
        return Err("verification delivery rejects implementers".into());
    }
    let planned = matches!(task.phase, Phase::Planned { .. } | Phase::InFlight { .. });
    let verifying = verification && matches!(task.phase, Phase::Merged { .. });
    if !planned && !verifying {
        return Err("agent requires planned or verification work".into());
    }
    Ok(())
}

pub(super) fn assignment(
    state: &domain::state::State,
    tier: Tier,
    role: AgentRole,
    fallback: Option<&Fallback>,
) -> Result<(Assignment, BudgetKind), String> {
    let profile = state.profile.as_ref().ok_or("profile unavailable")?;
    if role == AgentRole::Escalation {
        return Ok((profile.escalation.reviewer.clone(), BudgetKind::Escalation));
    }
    let roles = match tier {
        Tier::Trivial => &profile.tier.trivial,
        Tier::Lite => &profile.tier.lite,
        Tier::Full => &profile.tier.full,
    };
    let original = if role == AgentRole::Implementer {
        &roles.implementer
    } else {
        &roles.reviewer
    };
    let budget = if role == AgentRole::Implementer {
        BudgetKind::Implementation
    } else {
        BudgetKind::Review
    };
    let Some(fallback) = fallback else {
        return Ok((original.clone(), budget));
    };
    if fallback.reason.trim().is_empty() {
        return Err("fallback needs a reason".into());
    }
    let model = profile
        .models
        .get(&fallback.model)
        .ok_or("fallback model is undeclared")?;
    if !model.fallback_for.contains(&original.model) {
        return Err("model is not an allowed fallback".into());
    }
    Ok((
        Assignment {
            model: fallback.model.clone(),
            effort: original.effort,
        },
        budget,
    ))
}

pub(super) fn pair_for(
    task: &domain::task::Task,
    role: AgentRole,
) -> Option<&domain::authority::Authority> {
    task.authorities
        .iter()
        .rev()
        .find(|authority| authority::pair_available(authority, role == AgentRole::Implementer))
}

pub(super) fn previous_session(
    state: &domain::state::State,
    task: &TaskId,
    role: AgentRole,
) -> Option<String> {
    state
        .launches
        .iter()
        .rev()
        .find(|launch| launch.task == *task && launch.role == role)
        .and_then(|launch| launch.session.clone())
}
