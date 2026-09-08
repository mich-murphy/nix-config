//! Explicit merge-stage acceptance, separate from immutable full task criteria.
use crate::{
    authority::{Authority, Grant, UseId},
    engine,
    ids::{Digest, Sha, TaskId},
    model::*,
    store::*,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub key: TaskId,
    pub source: String,
    pub artifact: PathBuf,
    pub requirements: String,
    pub scope: String,
    pub operational_proof: String,
    pub cleanup_plan: String,
    pub criteria: Vec<Criterion>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub id: u64,
    pub request: Request,
    pub digest: Digest,
    pub revision: String,
    pub delivery: u64,
    pub started: u64,
    pub ended: Option<u64>,
    pub merge: Option<Sha>,
    pub review: Option<Review>,
    pub evidence: BTreeMap<String, Evidence>,
}
pub fn current(t: &Task) -> Option<&Stage> {
    t.stages.last().filter(|stage| stage.ended.is_none())
}
pub fn criteria(t: &Task) -> &[Criterion] {
    current(t).map_or(&t.criteria, |stage| &stage.request.criteria)
}
pub fn revision(t: &Task) -> Result<String> {
    current(t).map_or_else(
        || t.requirements.clone().context("missing requirements"),
        |stage| Ok(stage.revision.clone()),
    )
}
pub fn validate(t: &Task) -> Result<()> {
    if let Some(stage) = current(t) {
        ensure!(
            artifact(&stage.request.artifact)? == stage.digest,
            "stage authorization changed"
        );
        ensure!(
            t.requirements.as_ref() == Some(&stage.request.requirements),
            "full stage requirements changed"
        );
        ensure!(
            stage.delivery == crate::followup::identity(t),
            "stage belongs to another delivery"
        );
    }
    Ok(())
}
pub fn guard_brief(t: &Task, revision: &str) -> Result<()> {
    ensure!(
        t.stages.is_empty() || t.requirements.as_deref() == Some(revision),
        "staged task must preserve full criteria"
    );
    ensure!(
        current(t).is_none(),
        "end the explicit stage before briefing full acceptance"
    );
    Ok(())
}
pub fn full_acceptance(t: &Task) -> Result<()> {
    ensure!(
        current(t).is_none(),
        "stage PASS permits only its merge; end-stage and prove full task acceptance"
    );
    Ok(())
}
pub fn final_review_allowed(t: &Task, role: &Role) -> bool {
    *role == Role::Reviewer
        && !t.final_verified
        && current(t).is_none()
        && t.stages.last().is_some_and(|stage| {
            stage.delivery == crate::followup::identity(t) && stage.ended.is_some()
        })
}
fn invalidate(t: &mut Task) {
    t.review = None;
    t.evidence.clear();
    t.loop_state.plan = None;
    t.loop_state.measurements.clear();
    t.loop_state.human_review = None;
    t.final_verified = false;
}
fn valid_request(t: &Task, r: &Request) -> Result<Digest> {
    ensure!(
        t.started_by_run && !t.was_terminal && !t.main_verified && !t.final_verified,
        "stage requires unfinished unmerged run-owned work"
    );
    ensure!(current(t).is_none(), "stage already active");
    crate::superseded::stage_scope(t, r)?;
    ensure!(
        !crate::documentation::pending(t),
        "documentation exception cannot authorize staging"
    );
    ensure!(
        t.requirements.as_deref() == Some(&r.requirements)
            && hash(&serde_json::to_vec(&t.criteria)?) == r.requirements,
        "unchanged full requirements required"
    );
    for text in [&r.source, &r.scope, &r.operational_proof, &r.cleanup_plan] {
        required_text(text, "explicit staged sequencing and remaining proof")?;
    }
    ensure!(
        r.artifact.is_absolute(),
        "absolute authorization artifact required"
    );
    let digest = artifact(&r.artifact)?;
    let exercise_receipt = t
        .exercise
        .as_ref()
        .is_some_and(|exercise| exercise.digest == digest);
    ensure!(
        !t.authorities
            .iter()
            .any(|authority| authority.digest == digest && !exercise_receipt),
        "stage authorization replay"
    );
    ensure!(!r.criteria.is_empty(), "stage criteria required");
    let mut ids = BTreeSet::new();
    for c in &r.criteria {
        required_text(&c.id, "stage criterion id")?;
        required_text(&c.text, "stage criterion text")?;
        ensure!(
            ids.insert(&c.id) && !t.criteria.iter().any(|full| full.id == c.id),
            "stage criteria need distinct IDs from full criteria"
        );
    }
    Ok(digest)
}
pub fn begin(s: &mut State, request: Request, at: u64) -> Result<Value> {
    engine::idle(s)?;
    ensure!(
        s.active.as_deref() == Some(&request.key),
        "stage task must be active"
    );
    let mut t = s.tasks.get(request.key.as_ref()).context("task")?.clone();
    fresh(s, &t)?;
    let digest = valid_request(&t, &request)?;
    let id = s.id();
    let authority = Authority {
        id,
        source: request.source.clone(),
        artifact: request.artifact.clone(),
        digest: digest.clone(),
        requirements: request.requirements.parse()?,
        recorded: at,
        grant: Grant::Pair { paths: None },
        used: Some(UseId {
            implementation: None,
            review: None,
            receipt_only: true,
        }),
    };
    let register = !t
        .authorities
        .iter()
        .any(|entry| entry.digest == authority.digest);
    if register {
        crate::authority::register(&t.authorities, &authority, true)?;
    }
    let stage = Stage {
        id,
        revision: hash(&serde_json::to_vec(
            &json!({"request":request,"digest":digest,"delivery":crate::followup::identity(&t)}),
        )?),
        digest,
        request: request.clone(),
        delivery: crate::followup::identity(&t),
        started: at,
        ended: None,
        merge: None,
        review: None,
        evidence: BTreeMap::new(),
    };
    t.snapshot.as_mut().unwrap().requirements = stage.revision.clone();
    t.stages.push(stage.clone());
    if register {
        t.authorities.push(authority);
    }
    invalidate(&mut t);
    s.tasks.insert(request.key.to_string(), t);
    s.version = s.version.max(8);
    Ok(
        json!({"stage":stage,"additional_launches":0,"next":"loop-plan for the stage, checkpoint and evidence before independent review"}),
    )
}
pub fn end(s: &mut State, key: &str, at: u64) -> Result<Value> {
    engine::idle(s)?;
    ensure!(
        s.active.as_deref() == Some(key),
        "stage task must be active"
    );
    let mut t = s.tasks.get(key).context("task")?.clone();
    validate(&t)?;
    ensure!(
        current(&t).is_some() && t.main_verified && !t.pr_open,
        "verify the stage merge first"
    );
    engine::reviewed(s, &t)?;
    crate::loops::human_review(s, &t)?;
    on_main(&s.repo, t.merged_commit.as_deref().context("stage merge")?)?;
    let stage = t.stages.last_mut().unwrap();
    stage.ended = Some(at);
    stage.merge = t.merged_commit.clone();
    stage.review = t.review.clone();
    stage.evidence = t.evidence.clone();
    invalidate(&mut t);
    t.snapshot.as_mut().unwrap().requirements = t.requirements.clone().unwrap();
    t.next_action = "Prove unchanged full task criteria and cleanup; use authorized prepare-followup for more code. Stage review is historical only.".into();
    s.tasks.insert(key.into(), t);
    Ok(json!({"stage_ended":true,"full_acceptance_required":true,"additional_launches":0}))
}

pub fn guard_launch(t: &Task, role: &Role) -> Result<()> {
    ensure!(
        !t.main_verified || final_review_allowed(t, role),
        "merged work cannot start another implementation/review cycle"
    );
    Ok(())
}
