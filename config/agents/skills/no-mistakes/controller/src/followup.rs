//! A delivery attempt is distinct from cumulative task acceptance. Preparation
//! archives facts; only the existing budget command can authorize new launches.
use crate::{
    authority::{Authority, Grant},
    engine,
    ids::{Digest, Sha, SlotId, TaskId},
    model::*,
    store::*,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    pub id: u64,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub scope: String,
    pub requirements: String,
    pub base: Sha,
    pub recorded: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedDelivery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<crate::superseded::Replacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_observation: Option<Value>,
    // Historical tasks have an empty history to avoid recursive duplication.
    pub task: Box<Task>,
    pub observation: Value,
    pub archived: u64,
    pub operations: Vec<u64>,
    pub launches: Vec<u64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub key: TaskId,
    pub source: String,
    pub artifact: PathBuf,
    pub scope: String,
    pub requirements: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalSlotReuseRequest {
    pub historical_key: TaskId,
    pub source: String,
    pub artifact: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalSlotReuseAuthorization {
    pub delivery_id: u64,
    pub key: TaskId,
    pub slot: SlotId,
    pub historical_key: TaskId,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub requirements: String,
    pub previous_prepared_base: Sha,
    pub refreshed_base: Sha,
    pub recorded: u64,
}
pub struct HistoricalSlotBinding<'a> {
    pub key: &'a TaskId,
    pub slot: &'a SlotId,
    pub base: &'a Sha,
    pub request: HistoricalSlotReuseRequest,
    pub recorded: u64,
}

pub fn identity(t: &Task) -> u64 {
    t.followup.as_ref().map_or(0, |f| f.id)
}
pub fn original_base(t: &Task) -> Result<&Sha> {
    let original = t.delivery_history.first().map_or(t, |d| &d.task);
    Ok(&original
        .snapshot
        .as_ref()
        .context("original snapshot")?
        .base)
}
pub fn requirements(t: &Task, revision: &str) -> Result<()> {
    if let Some(f) = &t.followup {
        ensure!(
            revision == f.requirements,
            "follow-up must retain full task requirements"
        );
    }
    Ok(())
}
pub fn current_operation(s: &State, op: &Operation) -> Result<()> {
    let t = s.tasks.get(&op.key).context("operation task")?;
    ensure!(
        op.id > identity(t),
        "historical operation cannot act on the current delivery"
    );
    Ok(())
}
pub fn guard_input(s: &State, input: &Input) -> Result<()> {
    let id = match input {
        Input::Dispatch { operation }
        | Input::JiraObserve { operation, .. }
        | Input::JiraRetry { operation, .. }
        | Input::SyncFailed { operation, .. } => *operation,
        _ => return Ok(()),
    };
    current_operation(
        s,
        s.operations
            .iter()
            .find(|o| o.id == id)
            .context("operation")?,
    )
}

pub fn validate(s: &State, request: &Request, at: u64) -> Result<()> {
    engine::idle(s)?;
    crate::stages::full_acceptance(s.tasks.get(request.key.as_ref()).context("unknown task")?)?;
    ensure!(
        s.active.is_none(),
        "hold the task and leave the controller idle first"
    );
    let t = s.tasks.get(request.key.as_ref()).context("unknown task")?;
    ensure!(
        t.started_by_run
            && !t.was_terminal
            && !t.final_verified
            && t.delivery == Delivery::NeedsHuman
            && t.main_verified,
        "follow-up requires held run-owned work with a verified merge and unfinished acceptance"
    );
    ensure!(
        !t.spec.resolved
            && t.spec.jira_status != s.statuses.done
            && t.spec.member
            && t.spec.ownership_clear
            && t.spec.blocker.is_none(),
        "fresh unresolved Jira ownership and membership required"
    );
    ensure!(
        s.refresh_generation > s.selection_generation && at.saturating_sub(s.refreshed) <= 300,
        "refresh the full frozen queue before preparation"
    );
    ensure!(
        t.requirements.as_deref() == Some(request.requirements.as_str())
            && hash(&serde_json::to_vec(&t.criteria)?) == request.requirements
            && !t.criteria.is_empty(),
        "unchanged full requirements required"
    );
    ensure!(
        t.pr.is_some() && !t.pr_open && t.merged_commit.is_some(),
        "historical merged PR required"
    );
    ensure!(
        !t.authorities.iter().any(crate::authority::unused_pair),
        "consume the previous pair first"
    );
    required_text(&request.source, "actual current-user authorization source")?;
    required_text(
        &request.scope,
        "bounded follow-up scope and unchanged acceptance",
    )?;
    ensure!(
        request.artifact.is_absolute(),
        "authorization artifact must be absolute"
    );
    let digest = artifact(&request.artifact)?;
    ensure!(
        !t.authorities
            .iter()
            .any(|authority| authority.digest == digest),
        "authority digest already registered"
    );
    verify_history(s, t)?;
    let snapshot = t.snapshot.as_ref().context("historical snapshot")?;
    ensure!(
        snapshot.requirements == request.requirements,
        "historical requirements drift"
    );
    ensure!(
        sha(&s.repo, &snapshot.head)? == snapshot.head.clone(),
        "historical head unavailable"
    );
    on_main(&s.repo, t.merged_commit.as_ref().unwrap())?;
    Ok(())
}
pub fn prepare(s: &mut State, request: Request, observation: Value, at: u64) -> Result<Value> {
    validate(s, &request, at)?;
    advance(s, request, observation, at)
}
pub(crate) fn advance(
    s: &mut State,
    request: Request,
    observation: Value,
    at: u64,
) -> Result<Value> {
    let mut t = s.tasks[request.key.as_ref()].clone();
    let id = s.id();
    let auth = Authorization {
        id,
        source: request.source,
        digest: artifact(&request.artifact)?,
        artifact: request.artifact,
        scope: request.scope,
        requirements: request.requirements,
        base: sha(&s.repo, "origin/main")?,
        recorded: at,
    };
    let authority = Authority {
        id,
        source: auth.source.clone(),
        artifact: auth.artifact.clone(),
        digest: auth.digest.clone(),
        requirements: auth.requirements.parse()?,
        recorded: at,
        grant: Grant::Pair { paths: None },
        used: Some(crate::authority::UseId {
            implementation: None,
            review: None,
            receipt_only: true,
        }),
    };
    let exercise_receipt = t
        .exercise
        .as_ref()
        .is_some_and(|exercise| exercise.digest == authority.digest);
    let register = !t.authorities.iter().any(|entry| {
        entry.digest == authority.digest
            && (crate::authority::unused_pair(entry) || exercise_receipt)
    });
    if register {
        crate::authority::register(&t.authorities, &authority, true)?;
    }
    let mut previous = t.clone();
    previous.delivery_history.clear();
    t.delivery_history.push(ArchivedDelivery {
        superseded_by: None,
        replacement_observation: None,
        operations: s
            .operations
            .iter()
            .filter(|o| o.key == request.key.as_ref() && o.id > identity(&t))
            .map(|o| o.id)
            .collect(),
        launches: s
            .launches
            .iter()
            .filter(|l| l.key == request.key.as_ref() && l.id > identity(&t))
            .map(|l| l.id)
            .collect(),
        task: Box::new(previous),
        observation,
        archived: at,
    });
    t.followup = Some(auth.clone());
    if register {
        t.authorities.push(authority);
    }
    t.historical_slot_reuse = None;
    t.implementer_session = None;
    t.reviewer_session = None;
    t.slot = None;
    t.branch = None;
    t.snapshot = Some(Snapshot {
        base: auth.base.clone(),
        head: auth.base.clone(),
        requirements: auth.requirements.parse()?,
    });
    t.pr = None;
    t.pr_open = false;
    t.draft = true;
    t.merged_commit = None;
    t.main_verified = false;
    t.review = None;
    t.evidence.clear();
    t.loop_state.plan = None;
    t.loop_state.measurements.clear();
    t.loop_state.human_review = None;
    t.checks.clear();
    t.checks_head = None;
    // Keep deadlines, checkpoints, findings, usage and all counters.
    // Prior role sessions remain in the archive and every original launch.
    t.next_action = "Authorize the actual follow-up pair, resume, bind a new reserved slot and prepare a cumulative loop-plan. Full task acceptance still applies.".into();
    s.tasks.insert(request.key.to_string(), t);
    s.version = s.version.max(7);
    Ok(json!({"followup":auth,"additional_launches":0,"staged_acceptance_supported":true}))
}

pub fn binding(s: &State, t: &Task, slot: &str) -> Result<()> {
    if t.followup.is_none() {
        return Ok(());
    }
    validate_receipt(t)?;
    ensure!(
        !t.delivery_history
            .iter()
            .any(|d| d.task.slot.as_deref() == Some(slot)),
        "preserve historical slots; reserve a different numbered slot for the follow-up"
    );
    ensure!(
        !s.tasks
            .values()
            .any(|other| other
                .delivery_history
                .iter()
                .any(|d| d.task.slot.as_deref() == Some(slot))),
        "slot belongs to historical delivery work"
    );
    Ok(())
}
pub fn authorize_historical_slot_reuse(
    s: &State,
    t: &mut Task,
    binding: HistoricalSlotBinding<'_>,
) -> Result<()> {
    let followup = validate_reuse_request(s, t, binding.key, binding.slot, &binding.request)?;
    let digest = artifact(&binding.request.artifact)?;
    ensure!(
        !s.tasks.values().any(|task| {
            task.historical_slot_reuse
                .as_ref()
                .is_some_and(|receipt| receipt.digest == digest)
        }),
        "historical-slot authorization replay"
    );
    let previous_prepared_base = followup.base.clone();
    refresh_prepared_base(s, t, binding.base)?;
    let receipt = Authority {
        id: binding.recorded,
        source: binding.request.source.clone(),
        artifact: binding.request.artifact.clone(),
        digest: digest.clone(),
        requirements: followup.requirements.parse()?,
        recorded: binding.recorded,
        grant: Grant::Pair { paths: None },
        used: Some(crate::authority::UseId {
            implementation: None,
            review: None,
            receipt_only: true,
        }),
    };
    crate::authority::register(&t.authorities, &receipt, true)?;
    t.authorities.push(receipt);
    t.historical_slot_reuse = Some(HistoricalSlotReuseAuthorization {
        delivery_id: followup.id,
        key: binding.key.to_owned(),
        slot: binding.slot.to_owned(),
        historical_key: binding.request.historical_key,
        source: binding.request.source,
        artifact: binding.request.artifact,
        digest,
        requirements: followup.requirements,
        previous_prepared_base,
        refreshed_base: binding.base.to_owned(),
        recorded: binding.recorded,
    });
    Ok(())
}

fn validate_reuse_request(
    s: &State,
    t: &Task,
    key: &TaskId,
    slot: &SlotId,
    request: &HistoricalSlotReuseRequest,
) -> Result<Authorization> {
    validate_receipt(t)?;
    crate::stages::full_acceptance(t)?;
    ensure!(
        s.tasks
            .get(key.as_ref())
            .is_some_and(|stored| stored.slot.is_none())
            && t.historical_slot_reuse.is_none(),
        "historical-slot reuse applies only before binding"
    );
    ensure!(
        request.historical_key != *key
            && historical_slot_owner(s, request.historical_key.as_ref(), slot.as_ref()),
        "named task does not own completed historical work in this slot"
    );
    ensure!(
        current_delivery_unused(s, t),
        "historical-slot reuse requires an unstarted prepared delivery without evidence"
    );
    required_text(
        &request.source,
        "actual current-user slot-reuse authorization",
    )?;
    ensure!(
        request.artifact.is_absolute(),
        "slot-reuse authorization artifact must be absolute"
    );
    t.followup.clone().context("prepared follow-up required")
}

fn historical_slot_owner(s: &State, key: &str, slot: &str) -> bool {
    s.tasks.get(key).is_some_and(|task| {
        let completed_current = task.slot.as_deref() == Some(slot)
            && matches!(task.delivery, Delivery::Merged | Delivery::AlreadySatisfied)
            && task.sync == Sync::Confirmed
            && task.main_verified
            && task.merged_commit.is_some();
        completed_current
            || task.delivery_history.iter().any(|delivery| {
                delivery.task.slot.as_deref() == Some(slot)
                    && delivery.task.main_verified
                    && delivery.task.merged_commit.is_some()
            })
    })
}

fn current_delivery_unused(s: &State, t: &Task) -> bool {
    let Some(followup) = &t.followup else {
        return false;
    };
    !s.launches
        .iter()
        .any(|launch| launch.key == t.spec.key && launch.id > followup.id)
        && !s
            .operations
            .iter()
            .any(|operation| operation.key == t.spec.key && operation.id > followup.id)
        && t.evidence.is_empty()
        && t.review.is_none()
        && t.loop_state.plan.is_none()
        && t.loop_state.measurements.is_empty()
        && t.loop_state.human_review.is_none()
        && t.checks.is_empty()
        && t.checks_head.is_none()
        && t.pr.is_none()
        && !t.main_verified
        && !t.final_verified
}

fn refresh_prepared_base(s: &State, t: &mut Task, base: &Sha) -> Result<()> {
    let followup = t.followup.as_mut().context("prepared follow-up required")?;
    on_main(&s.repo, &followup.base)?;
    let expected = Snapshot {
        base: followup.base.clone(),
        head: followup.base.clone(),
        requirements: followup.requirements.parse()?,
    };
    ensure!(
        t.snapshot.as_ref() == Some(&expected),
        "prepared snapshot changed before authorized binding"
    );
    followup.base = base.to_owned();
    t.snapshot = Some(Snapshot {
        base: base.to_owned(),
        head: base.to_owned(),
        requirements: expected.requirements,
    });
    Ok(())
}

fn validate_receipt(t: &Task) -> Result<()> {
    let f = t.followup.as_ref().context("follow-up")?;
    ensure!(
        artifact(&f.artifact)? == f.digest,
        "follow-up authorization receipt changed"
    );
    requirements(t, t.requirements.as_deref().context("requirements")?)
}
fn validate_slot_reuse_receipt(t: &Task) -> Result<()> {
    let Some(receipt) = &t.historical_slot_reuse else {
        return Ok(());
    };
    ensure!(
        artifact(&receipt.artifact)? == receipt.digest,
        "historical-slot authorization receipt changed"
    );
    let followup = t.followup.as_ref().context("prepared follow-up required")?;
    ensure!(
        receipt.key == t.spec.key
            && receipt.delivery_id == followup.id
            && t.slot.as_deref() == Some(receipt.slot.as_ref())
            && receipt.requirements == followup.requirements
            && receipt.refreshed_base == followup.base,
        "historical-slot authorization no longer matches this delivery"
    );
    Ok(())
}
pub fn authorization_snapshot(s: &State, t: &Task, receipt: &Path) -> Result<Snapshot> {
    let Some(f) = &t.followup else {
        return fresh(s, t);
    };
    validate_receipt(t)?;
    // The first pair must use the preparation receipt. Subsequent pairs need
    // fresh authority and remain bound to this delivery's base and requirements.
    let used = t.authorities.iter().any(|authority| {
        authority.id == f.id
            && authority.digest == f.digest
            && authority
                .used
                .as_ref()
                .is_some_and(|use_id| use_id.review.is_some())
    });
    ensure!(
        used || artifact(receipt)? == f.digest,
        "first follow-up pair must use its preparation receipt"
    );
    if t.slot.is_some() {
        return fresh(s, t);
    }
    ensure!(
        sha(&s.repo, "origin/main")? == f.base,
        "prepared origin/main base changed"
    );
    Ok(Snapshot {
        base: f.base.clone(),
        head: f.base.clone(),
        requirements: f.requirements.clone(),
    })
}
pub fn before_launch(s: &State, t: &Task, role: &Role) -> Result<()> {
    let Some(f) = &t.followup else {
        return Ok(());
    };
    validate_receipt(t)?;
    validate_slot_reuse_receipt(t)?;
    fresh(s, t)?;
    ensure!(
        *role != Role::Escalation,
        "follow-up authority covers an implementation/review pair only"
    );
    let pair = crate::authority::current_pair(&t.authorities)
        .context("authorize the follow-up implementation/review pair first")?;
    ensure!(
        pair.id == f.id || pair.recorded >= f.recorded,
        "historical pair does not authorize this delivery"
    );
    ensure!(
        !crate::documentation::pending(t),
        "historical documentation allowance cannot authorize follow-up work"
    );
    Ok(())
}
pub fn verify_history(s: &State, t: &Task) -> Result<()> {
    for d in &t.delivery_history {
        if let Some(replacement) = &d.superseded_by {
            ensure!(
                !d.task.main_verified
                    && d.task.merged_commit.is_none()
                    && d.observation["state"] == "CLOSED",
                "closed delivery history was rewritten as a merge"
            );
            crate::superseded::replacement(
                s,
                replacement,
                d.replacement_observation
                    .as_ref()
                    .context("replacement observation")?,
            )?;
        } else {
            on_main(
                &s.repo,
                d.task
                    .merged_commit
                    .as_deref()
                    .context("historical merge")?,
            )?;
        }
        ensure!(
            d.task.requirements == t.requirements,
            "cumulative task requirements changed"
        );
    }
    Ok(())
}

pub fn acceptance(t: &Task) -> Result<()> {
    if t.followup.is_some() {
        validate_receipt(t)?;
    }
    Ok(())
}
