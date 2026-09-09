use crate::{AgentError, App, Rejection};
use domain::{
    authority,
    budget::BudgetKind,
    command::{AgentRole, Fallback},
    delivery::{Delivery, DeliveryKind},
    event::LaunchOutcome,
    ids::{AuthorityId, DeliveryId, Digest, LaunchId, OperationId, TaskId},
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

/// The launch a new snapshot should record as covered: the most recent
/// settled implementer launch, if it has been checkpointed. `None` when
/// no implementer launch has run yet, the current one is still in
/// flight, or it has not been checkpointed yet (an integration-only
/// snapshot is a valid reason for `None` too).
pub(super) fn latest_checkpointed_launch(
    state: &domain::state::State,
    task: &TaskId,
) -> Option<LaunchId> {
    state
        .launches
        .iter()
        .rev()
        .find(|launch| {
            launch.task == *task
                && launch.role == AgentRole::Implementer
                && launch.outcome.is_some()
        })
        .filter(|launch| launch.checkpointed)
        .map(|launch| launch.id)
}

pub(super) fn next_authority(task: &domain::task::Task) -> AuthorityId {
    AuthorityId(
        task.authorities
            .iter()
            .map(|authority| authority.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

/// A task is done and recorded, never bypassable by an in-progress one:
/// the only fact `Phase::Completed` and `Phase::Verified` disagree on.
pub(super) fn completed(task: &domain::task::Task) -> bool {
    matches!(task.phase, Phase::Completed { .. })
}

pub(super) fn current_work_delivery(
    task: &domain::task::Task,
) -> Option<(&domain::task::PlannedWork, &Delivery)> {
    match task.phase {
        Phase::Planned | Phase::InFlight => {
            let delivery = task.deliveries.last()?;
            Some((delivery.work.as_ref()?, delivery))
        }
        _ => None,
    }
}

pub(super) fn launch_prompt(task: &domain::task::Task, prompt: String) -> String {
    let Some((work, _)) = current_work_delivery(task) else {
        return prompt;
    };
    if work.feedback.is_empty() {
        return prompt;
    }
    let instructions = work
        .feedback
        .iter()
        .map(|lesson| format!("- {}", lesson.text))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{prompt}\n\nPinned run guidance:\n{instructions}")
}

pub(super) fn review_target(
    task: &domain::task::Task,
    delivery: &Delivery,
) -> Option<domain::acceptance::Snapshot> {
    crate::delivery_support::current_snapshot(task)
        .cloned()
        .or_else(|| match &delivery.kind {
            DeliveryKind::Verification { of } => Some(domain::acceptance::Snapshot {
                base: of.clone(),
                head: of.clone(),
                requirements: task.spec.requirements.clone(),
            }),
            DeliveryKind::Code { .. } => None,
        })
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
                    .any(|pattern| domain::delivery::glob(pattern, path))
            })
            .count() as u32
    })
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
    if !domain::delivery::admits(delivery, role) {
        return Err("verification delivery rejects implementers".into());
    }
    let planned = matches!(task.phase, Phase::Planned | Phase::InFlight);
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
    let selected = fallback.map_or_else(
        || Ok(original.clone()),
        |value| fallback_assignment(profile, original, value),
    )?;
    Ok((selected, budget))
}

fn fallback_assignment(
    profile: &domain::risk::Profile,
    original: &Assignment,
    fallback: &Fallback,
) -> Result<Assignment, String> {
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
    Ok(Assignment {
        model: fallback.model.clone(),
        effort: original.effort,
    })
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

/// The session id a harness adapter should resume, from the most recent
/// launch of `role`. A completed launch reports its real session; a
/// failed or cancelled one reports an empty string, exactly as its
/// `LaunchOutcome` recorded it, so a caller that only wants a genuine
/// prior session can still filter it out (`!session.is_empty()`).
pub(super) fn previous_session(
    state: &domain::state::State,
    task: &TaskId,
    role: AgentRole,
) -> Option<String> {
    let outcome = state
        .launches
        .iter()
        .rev()
        .find(|launch| launch.task == *task && launch.role == role)?
        .outcome
        .as_ref()?;
    Some(match outcome {
        LaunchOutcome::Completed { session, .. } => session.clone(),
        LaunchOutcome::Failed { .. } | LaunchOutcome::Cancelled => String::new(),
    })
}
