//! Closed PR recovery and phase-scoped reuse of one explicit exercise approval.
use crate::{
    engine, followup,
    ids::{Digest, Sha, TaskId},
    model::*,
    runtime,
    store::*,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub pr: u64,
    pub head: Sha,
    pub merge: Sha,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub preparation: u64,
    pub cleanup: Option<u64>,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub requirements: String,
    pub cleanup_scope: String,
    pub cleanup_paths: Vec<String>,
    pub cleanup_criteria: Vec<Criterion>,
    pub outcome: Option<PathBuf>,
    pub outcome_digest: Option<Digest>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub key: TaskId,
    pub source: String,
    pub artifact: PathBuf,
    pub requirements: String,
    pub scope: String,
    pub replacement: Replacement,
    pub cleanup_scope: String,
    pub cleanup_paths: Vec<String>,
    pub cleanup_criteria: Vec<Criterion>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupRequest {
    pub key: TaskId,
    pub preparation: u64,
    pub outcome_artifact: PathBuf,
}
fn held(s: &State, key: &str, requirements: &str, at: u64) -> Result<()> {
    engine::idle(s)?;
    ensure!(
        s.active.is_none(),
        "hold the task before preparing recovery"
    );
    let t = s.tasks.get(key).context("unknown task")?;
    crate::stages::full_acceptance(t)?;
    ensure!(
        t.started_by_run
            && !t.was_terminal
            && !t.final_verified
            && t.delivery == Delivery::NeedsHuman,
        "only held unfinished run-owned tasks may recover"
    );
    ensure!(
        !t.spec.resolved
            && t.spec.jira_status != s.statuses.done
            && t.spec.member
            && t.spec.ownership_clear
            && t.spec.blocker.is_none(),
        "fresh unresolved Jira membership and ownership required"
    );
    ensure!(
        s.refresh_generation > s.selection_generation && at.saturating_sub(s.refreshed) <= 300,
        "refresh full queue before recovery"
    );
    ensure!(
        t.requirements.as_deref() == Some(requirements)
            && hash(&serde_json::to_vec(&t.criteria)?) == requirements
            && !t.criteria.is_empty(),
        "unchanged full criteria required"
    );
    followup::verify_history(s, t)
}
fn unused(t: &Task, digest: &Digest, requirements: &str) -> Result<()> {
    let pair = crate::authority::current_pair(&t.authorities)
        .context("recorded unused preparation allowance required")?;
    ensure!(
        crate::authority::unused_pair(pair),
        "only a wholly unused current pair can transfer"
    );
    ensure!(
        pair.digest == *digest,
        "approval must match recorded preparation receipt"
    );
    crate::authority::validate_requirements(pair, requirements)
}
fn validate(s: &State, r: &Request, at: u64) -> Result<()> {
    held(s, r.key.as_ref(), &r.requirements, at)?;
    let t = &s.tasks[r.key.as_ref()];
    ensure!(
        t.exercise.is_none()
            && !t.main_verified
            && t.merged_commit.is_none()
            && t.pr.is_some()
            && !t.pr_open,
        "recovery requires a closed unmerged current PR with no existing exercise plan"
    );
    ensure!(
        t.pr != Some(r.replacement.pr)
            && !t
                .delivery_history
                .iter()
                .any(|d| d.task.pr == Some(r.replacement.pr)),
        "replacement must be a distinct external PR"
    );
    for text in [&r.source, &r.scope, &r.cleanup_scope] {
        required_text(
            text,
            "actual approval and bounded preparation/cleanup scopes",
        )?;
    }
    ensure!(
        r.artifact.is_absolute(),
        "absolute approval receipt required"
    );
    unused(t, &artifact(&r.artifact)?, &r.requirements)?;
    ensure!(
        !r.cleanup_paths.is_empty(),
        "explicit cleanup paths required"
    );
    ensure!(
        !r.cleanup_criteria.is_empty(),
        "bounded cleanup criteria required"
    );
    let mut ids = std::collections::BTreeSet::new();
    for criterion in &r.cleanup_criteria {
        required_text(&criterion.id, "cleanup criterion id")?;
        required_text(&criterion.text, "cleanup criterion text")?;
        ensure!(
            ids.insert(&criterion.id) && !t.criteria.iter().any(|c| c.id == criterion.id),
            "cleanup criteria must have distinct phase IDs"
        );
    }
    for path in &r.cleanup_paths {
        crate::loops::relative(path)?;
    }
    sha(
        &s.repo,
        &t.snapshot.as_ref().context("historical snapshot")?.head,
    )?;
    Ok(())
}
pub fn closed(s: &State, key: &str, pr: &Value) -> Result<()> {
    runtime::verify_pr(s, key, pr)?;
    ensure!(
        pr["state"] == "CLOSED" && pr["mergeCommit"].is_null(),
        "superseded PR must remain closed and unmerged"
    );
    Ok(())
}
pub fn replacement(s: &State, expected: &Replacement, pr: &Value) -> Result<()> {
    ensure!(
        pr["number"] == expected.pr
            && pr["state"] == "MERGED"
            && pr["baseRefName"] == "main"
            && pr["headRefOid"].as_str() == Some(expected.head.as_ref())
            && pr["mergeCommit"]["oid"].as_str() == Some(expected.merge.as_ref()),
        "external replacement identity/state changed"
    );
    on_main(&s.repo, &expected.merge)
}
pub fn recover(store: &mut Store, r: Request) -> Result<Value> {
    let s = store.read()?;
    validate(&s, &r, now())?;
    let old = runtime::read_pr_number(store, &s.github_repo, s.tasks[r.key.as_ref()].pr.unwrap())?;
    closed(&s, &r.key, &old)?;
    let external = runtime::read_pr_number(store, &s.github_repo, r.replacement.pr)?;
    git(
        &s.repo,
        &[
            "fetch",
            "origin",
            "refs/heads/main:refs/remotes/origin/main",
        ],
    )?;
    replacement(&s, &r.replacement, &external)?;
    store.change("recover-superseded", |s| {
        validate(s, &r, now())?;
        closed(s, &r.key, &old)?;
        replacement(s, &r.replacement, &external)?;
        advance_recovery(s, r, old, external)
    })
}
fn advance_recovery(s: &mut State, r: Request, old: Value, external: Value) -> Result<Value> {
    let at = now();
    let request = followup::Request {
        key: r.key.clone(),
        source: r.source.clone(),
        artifact: r.artifact.clone(),
        scope: r.scope,
        requirements: r.requirements.clone(),
    };
    followup::advance(s, request, old, at)?;
    let t = s.tasks.get_mut(r.key.as_ref()).unwrap();
    let auth = t.followup.as_ref().unwrap();
    if let Some(pair) = t
        .authorities
        .iter_mut()
        .find(|authority| authority.digest == auth.digest)
    {
        pair.recorded = at;
    }
    let archive = t.delivery_history.last_mut().unwrap();
    archive.superseded_by = Some(r.replacement);
    archive.replacement_observation = Some(external);
    t.exercise = Some(Exercise {
        preparation: auth.id,
        cleanup: None,
        source: r.source,
        artifact: r.artifact,
        digest: auth.digest.clone(),
        requirements: r.requirements,
        cleanup_scope: r.cleanup_scope,
        cleanup_paths: r.cleanup_paths,
        cleanup_criteria: r.cleanup_criteria,
        outcome: None,
        outcome_digest: None,
    });
    s.version = s.version.max(9);
    Ok(
        json!({"delivery":auth.id,"preparation_pair_transferred":true,"new_preparation_launches":0,"cleanup_reserved":true,"next":"resume, reserve a different slot, provision, begin-stage and plan"}),
    )
}
fn cleanup_ready(s: &State, r: &CleanupRequest, at: u64) -> Result<Exercise> {
    let t = s.tasks.get(r.key.as_ref()).context("task")?;
    let plan = t
        .exercise
        .as_ref()
        .context("no explicitly approved cleanup phase")?
        .clone();
    held(s, r.key.as_ref(), &plan.requirements, at)?;
    ensure!(
        plan.preparation == r.preparation
            && followup::identity(t) == r.preparation
            && plan.cleanup.is_none(),
        "cleanup phase is not available for this delivery"
    );
    ensure!(
        artifact(&plan.artifact)? == plan.digest,
        "exercise approval changed"
    );
    ensure!(
        t.main_verified && !t.pr_open && t.merged_commit.is_some(),
        "verify preparation merge before cleanup"
    );
    ensure!(
        t.stages
            .last()
            .is_some_and(|stage| stage.delivery == r.preparation && stage.ended.is_some()),
        "end the approved preparation stage before cleanup"
    );
    ensure!(
        t.authorities.iter().any(|authority| {
            authority.digest == plan.digest
                && authority
                    .used
                    .as_ref()
                    .is_some_and(|used| used.review.is_some())
        }),
        "preparation pair has not been consumed"
    );
    ensure!(
        r.outcome_artifact.is_absolute(),
        "absolute terminal exercise/outcome receipt required"
    );
    outcome(&r.outcome_artifact)?;
    Ok(plan)
}
pub fn cleanup(store: &mut Store, r: CleanupRequest) -> Result<Value> {
    let s = store.read()?;
    cleanup_ready(&s, &r, now())?;
    let pr = runtime::read_pr_number(
        store,
        &s.github_repo,
        s.tasks[r.key.as_ref()].pr.context("preparation PR")?,
    )?;
    runtime::verify_merged_observation(&s, &r.key, &pr)?;
    store.change("prepare-approved-cleanup", |s| {
        let plan = cleanup_ready(s, &r, now())?;
        runtime::verify_pr(s, &r.key, &pr)?;
        advance_cleanup(s, r, plan, pr)
    })
}
fn advance_cleanup(
    s: &mut State,
    r: CleanupRequest,
    mut plan: Exercise,
    pr: Value,
) -> Result<Value> {
    let at = now();
    followup::advance(
        s,
        followup::Request {
            key: r.key.clone(),
            source: plan.source.clone(),
            artifact: plan.artifact.clone(),
            scope: plan.cleanup_scope.clone(),
            requirements: plan.requirements.clone(),
        },
        pr,
        at,
    )?;
    let t = s.tasks.get_mut(r.key.as_ref()).unwrap();
    let auth = t.followup.as_ref().unwrap();
    if let Some(pair) = t
        .authorities
        .iter_mut()
        .find(|authority| authority.digest == plan.digest)
    {
        pair.used = None;
        pair.recorded = at;
    }
    plan.cleanup = Some(auth.id);
    plan.outcome_digest = Some(artifact(&r.outcome_artifact)?);
    plan.outcome = Some(r.outcome_artifact);
    t.exercise = Some(plan);
    s.version = s.version.max(9);
    Ok(
        json!({"delivery":auth.id,"phase":"cleanup","additional_implementation_turns":1,"additional_reviews":1,"authority":"existing explicit exercise approval","next":"resume and provision a different owned slot; full task criteria remain required"}),
    )
}
pub fn cleanup_active(t: &Task) -> bool {
    t.exercise
        .as_ref()
        .is_some_and(|plan| plan.cleanup == Some(followup::identity(t)))
}
pub fn scope(s: &State, t: &Task, snap: &Snapshot) -> Result<()> {
    if !cleanup_active(t) {
        return Ok(());
    }
    let plan = t.exercise.as_ref().unwrap();
    ensure!(
        artifact(&plan.artifact)? == plan.digest,
        "exercise approval changed"
    );
    ensure!(
        artifact(plan.outcome.as_ref().context("outcome receipt")?)?
            == *plan.outcome_digest.as_ref().context("outcome digest")?,
        "exercise outcome receipt changed"
    );
    let commits = git(
        &s.repo,
        &[
            "rev-list",
            "--reverse",
            &format!("{}..{}", snap.base, snap.head),
        ],
    )?;
    for commit in commits.lines() {
        let paths = git(
            &s.repo,
            &[
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-only",
                "--no-renames",
                "-r",
                "-m",
                commit,
            ],
        )?;
        ensure!(
            paths
                .lines()
                .all(|path| plan.cleanup_paths.iter().any(|allowed| allowed == path)),
            "cleanup changes exceed approved file scope, including intermediate commits"
        );
    }
    Ok(())
}

fn outcome(path: &std::path::Path) -> Result<()> {
    let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(
        value["terminal"] == true,
        "exercise must be terminal before cleanup preparation"
    );
    required_text(
        value["dispatch"].as_str().unwrap_or(""),
        "actual run identity or explicit no-dispatch outcome",
    )?;
    required_text(
        value["recovery_observation"].as_str().unwrap_or(""),
        "observed outcome and recovery evidence",
    )?;
    Ok(())
}

pub fn stage_scope(t: &Task, r: &crate::stages::Request) -> Result<()> {
    if !cleanup_active(t) {
        return Ok(());
    }
    let plan = t.exercise.as_ref().unwrap();
    ensure!(
        artifact(&r.artifact)? == plan.digest
            && r.scope == plan.cleanup_scope
            && serde_json::to_value(&r.criteria)? == serde_json::to_value(&plan.cleanup_criteria)?,
        "cleanup stage must match the recorded phase scope, criteria and original approval"
    );
    Ok(())
}
