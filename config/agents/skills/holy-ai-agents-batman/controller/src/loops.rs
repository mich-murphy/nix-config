//! Progress measurements and steering for the delivery controller.
//! Narrative judgments and human receipts remain trusted coordinator inputs.
use crate::{
    ids::{Digest, Sha},
    model::*,
    store::*,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    #[default]
    Autonomous,
    HumanReview,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoopPolicy {
    pub max_open_prs: usize,
    pub max_stalled_checkpoints: u32,
    pub max_implementation_turns: usize,
    pub review_mode: ReviewMode,
    pub feedback_file: Option<String>,
}
impl Default for LoopPolicy {
    fn default() -> Self {
        Self {
            max_open_prs: 2,
            max_stalled_checkpoints: 2,
            max_implementation_turns: 8,
            review_mode: ReviewMode::Autonomous,
            feedback_file: None,
        }
    }
}
pub fn validate_policy(p: &LoopPolicy) -> Result<()> {
    ensure!(
        p.max_open_prs > 0 && p.max_stalled_checkpoints > 0 && p.max_implementation_turns > 0,
        "loop limits must be positive"
    );
    if let Some(path) = &p.feedback_file {
        relative(path)?;
    }
    Ok(())
}
pub(crate) fn relative(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && !value.contains(['\\', '\n', '\r'])
            && Path::new(value)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "expected a repository-relative path without traversal"
    );
    Ok(())
}
pub fn policy(s: &State) -> Result<&LoopPolicy> {
    s.loop_policy.as_ref().context(
        "configure-loop is required for this existing run; retain its original state and budgets",
    )
}
pub fn unfinished(s: &State) -> Vec<String> {
    s.tasks
        .values()
        .filter(|t| t.pr_open)
        .map(|t| t.spec.key.to_string())
        .collect()
}
pub fn capacity(s: &State) -> Result<()> {
    ensure!(
        unfinished(s).len() < policy(s)?.max_open_prs,
        "unfinished PR limit reached; observe and recover existing PRs before claiming more work"
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineInput {
    pub criterion: String,
    pub starting_condition: String,
    pub target: String,
    pub method: String,
    pub artifact: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanInput {
    pub baseline_commit: String,
    pub family: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lesson_families: Vec<String>,
    pub deliverable: String,
    pub components: Vec<String>,
    pub examples: Vec<String>,
    pub baselines: Vec<BaselineInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub id: String,
    pub families: Vec<String>,
    pub source: String,
    pub instruction: String,
    pub example: String,
    pub status: LessonStatus,
    pub acceptance_source: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LessonStatus {
    Proposed,
    Accepted,
    Retired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedLesson {
    pub key: String,
    pub main_commit: Option<Sha>,
    pub lesson: Lesson,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub file: Option<String>,
    pub commit: Sha,
    pub digest: Digest,
    pub lessons: Vec<Lesson>,
    pub run_lessons: Vec<LearnedLesson>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub input: PlanInput,
    pub snapshot: Snapshot,
    pub baseline_digests: BTreeMap<String, Digest>,
    pub feedback: Feedback,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub snapshot: Snapshot,
    pub observed: String,
    pub satisfied: bool,
    pub artifact: PathBuf,
    pub digest: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSize {
    pub files: usize,
    pub added: u64,
    pub deleted: u64,
    pub binary_files: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub size: ChangeSize,
    pub launch: Option<u64>,
    pub snapshot: Snapshot,
    pub advanced: bool,
    pub finding: String,
    pub next_step: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub paths: Vec<String>,
    pub outside_plan: Vec<String>,
    pub scope_reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReview {
    pub snapshot: Snapshot,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskLoop {
    pub plan: Option<Plan>,
    pub measurements: BTreeMap<String, Measurement>,
    pub checkpoints: Vec<Checkpoint>,
    pub stalled: u32,
    pub human_review: Option<HumanReview>,
}

pub fn configure(s: &mut State, p: LoopPolicy) -> Result<Value> {
    crate::engine::idle(s)?;
    validate_policy(&p)?;
    ensure!(
        s.loop_policy.is_none() || !s.tasks.values().any(|t| t.started_by_run),
        "run policy is frozen after the first claim; do not reset limits"
    );
    s.loop_policy = Some(p);
    Ok(json!({"policy":s.loop_policy,"existing_counters_preserved":true}))
}
fn feedback(s: &State, families: &[&str]) -> Result<Feedback> {
    let p = policy(s)?;
    let commit = sha(&s.repo, "origin/main")?;
    let mut result = Feedback {
        file: p.feedback_file.clone(),
        commit: commit.clone(),
        digest: hash(b"[]").parse().map_err(anyhow::Error::msg)?,
        lessons: vec![],
        run_lessons: vec![],
    };
    if let Some(file) = &p.feedback_file {
        relative(file)?;
        let raw = git(&s.repo, &["show", &format!("{commit}:{file}")])?;
        let lessons: Vec<Lesson> = serde_json::from_str(&raw)
            .context("feedback must be a JSON array of scoped lessons")?;
        let mut ids = std::collections::BTreeSet::new();
        for lesson in lessons {
            ensure!(ids.insert(lesson.id.clone()), "duplicate feedback id");
            for text in [
                &lesson.id,
                &lesson.source,
                &lesson.instruction,
                &lesson.example,
            ] {
                required_text(text, "feedback field")?;
            }
            ensure!(
                !lesson.families.is_empty() && lesson.families.iter().all(|x| !x.trim().is_empty()),
                "feedback family required"
            );
            if lesson.status == LessonStatus::Accepted {
                required_text(
                    lesson.acceptance_source.as_deref().unwrap_or(""),
                    "acceptance disposition source for reusable guidance",
                )?;
                if lesson
                    .families
                    .iter()
                    .any(|x| families.contains(&x.as_str()) || x == "*")
                {
                    result.lessons.push(lesson);
                }
            }
        }
        result.digest = hash(raw.as_bytes()).parse().map_err(anyhow::Error::msg)?;
    }
    result.run_lessons = s
        .lessons
        .values()
        .filter(|r| {
            r.lesson.status == LessonStatus::Accepted
                && r.lesson
                    .families
                    .iter()
                    .any(|f| families.contains(&f.as_str()) || f == "*")
        })
        .cloned()
        .collect();
    result.digest = hash(&serde_json::to_vec(&(&result.digest, &result.run_lessons))?)
        .parse()
        .map_err(anyhow::Error::msg)?;
    Ok(result)
}
pub fn record_lesson(s: &mut State, key: &str, lesson: Lesson) -> Result<Value> {
    crate::engine::idle(s)?;
    policy(s)?;
    let t = s.tasks.get(key).context("unknown source task")?;
    ensure!(
        t.started_by_run,
        "lesson source must be a task owned by this run"
    );
    for value in [
        &lesson.id,
        &lesson.source,
        &lesson.instruction,
        &lesson.example,
    ] {
        required_text(value, "lesson field")?;
    }
    ensure!(
        !lesson.families.is_empty() && lesson.families.iter().all(|v| !v.trim().is_empty()),
        "scoped lesson family required"
    );
    if lesson.status != LessonStatus::Proposed {
        ensure!(
            t.final_verified && t.main_verified,
            "accept or retire guidance only after the source task's reviewed delivery and final verification"
        );
        crate::engine::reviewed(s, t)?;
        on_main(
            &s.repo,
            t.merged_commit
                .as_ref()
                .context("verified source main commit required")?,
        )?;
        human_review(s, t)?;
        required_text(
            lesson.acceptance_source.as_deref().unwrap_or(""),
            "evidence-based acceptance or retirement disposition",
        )?;
    }
    if let Some(old) = s.lessons.get(&lesson.id) {
        ensure!(
            old.lesson.status != LessonStatus::Accepted || lesson.status == LessonStatus::Retired,
            "accepted lessons are immutable; retire the old id and propose a corrected lesson"
        );
    }
    let learned = LearnedLesson {
        key: key.into(),
        main_commit: t.merged_commit.clone(),
        lesson,
    };
    s.lessons.insert(learned.lesson.id.clone(), learned.clone());
    Ok(json!(learned))
}
pub fn plan(s: &State, t: &mut Task, input: PlanInput) -> Result<Value> {
    let snap = fresh(s, t)?;
    ensure!(
        &sha(&s.repo, &input.baseline_commit)? == crate::followup::original_base(t)?,
        "baseline must identify the original task base commit; inspect that revision when resuming existing work"
    );
    required_text(&input.family, "task family")?;
    required_text(&input.deliverable, "smallest complete deliverable")?;
    ensure!(
        !input.components.is_empty(),
        "expected affected components required"
    );
    for p in &input.components {
        relative(p)?;
    }
    for example in &input.examples {
        required_text(example, "approved example reference")?;
    }
    ensure!(
        input.baselines.len() == crate::stages::criteria(t).len(),
        "baseline required for every criterion"
    );
    let mut digests = BTreeMap::new();
    for b in &input.baselines {
        ensure!(
            crate::stages::criteria(t)
                .iter()
                .any(|c| c.id == b.criterion),
            "unknown baseline criterion"
        );
        for text in [&b.starting_condition, &b.target, &b.method] {
            required_text(text, "baseline field")?;
        }
        ensure!(
            digests
                .insert(b.criterion.clone(), artifact(&b.artifact)?)
                .is_none(),
            "duplicate baseline criterion"
        );
    }
    if let Some(old) = t.loop_state.plan.as_ref().or_else(|| {
        t.delivery_history
            .first()
            .and_then(|d| d.task.loop_state.plan.as_ref())
    }) && old.snapshot.requirements == snap.requirements
    {
        ensure!(
            serde_json::to_value(&old.input.baselines)? == serde_json::to_value(&input.baselines)?
                && old.baseline_digests == digests,
            "starting baseline is frozen for these criteria; changed requirements need a revised brief"
        );
    }
    let mut families = vec![input.family.as_str()];
    for family in &input.lesson_families {
        required_text(family, "additional lesson family")?;
        families.push(family);
    }
    let feedback = feedback(s, &families)?;
    // Replanning changes the acceptance context, never the launch or stall counters.
    t.loop_state.plan = Some(Plan {
        input,
        snapshot: snap,
        baseline_digests: digests,
        feedback,
    });
    t.loop_state.measurements.clear();
    t.loop_state.human_review = None;
    t.evidence.clear();
    t.review = None;
    t.final_verified = false;
    Ok(json!({"plan":t.loop_state.plan,"next":"implement or measure existing behavior"}))
}
pub fn planned<'a>(s: &State, t: &'a Task) -> Result<&'a Plan> {
    policy(s)?;
    let p = t
        .loop_state
        .plan
        .as_ref()
        .context("loop-plan required before agent work or acceptance")?;
    ensure!(
        p.snapshot.requirements == crate::stages::revision(t)?,
        "requirements changed; revise loop-plan without resetting counters"
    );
    for b in &p.input.baselines {
        ensure!(
            p.baseline_digests.get(&b.criterion) == Some(&artifact(&b.artifact)?),
            "baseline artifact changed"
        );
    }
    Ok(p)
}
pub fn measure(
    s: &State,
    t: &mut Task,
    criterion: String,
    observed: String,
    satisfied: bool,
    path: PathBuf,
) -> Result<Value> {
    planned(s, t)?;
    ensure!(
        crate::stages::criteria(t).iter().any(|c| c.id == criterion),
        "unknown measurement criterion"
    );
    required_text(&observed, "observed result and comparison with baseline")?;
    let m = Measurement {
        snapshot: fresh(s, t)?,
        observed,
        satisfied,
        digest: artifact(&path)?,
        artifact: path,
    };
    t.loop_state.measurements.insert(criterion, m.clone());
    t.review = None;
    t.final_verified = false;
    t.loop_state.human_review = None;
    Ok(json!(m))
}
fn last_implementation<'a>(s: &'a State, t: &Task) -> Option<&'a Launch> {
    s.launches.iter().rev().find(|l| {
        l.key == t.spec.key && l.role == Role::Implementer && l.id > crate::followup::identity(t)
    })
}
pub fn needs_checkpoint(s: &State, t: &Task) -> bool {
    last_implementation(s, t).is_some_and(|l| {
        l.ended.is_some() && t.loop_state.checkpoints.last().and_then(|c| c.launch) != Some(l.id)
    })
}
pub fn implementation_budget(s: &State, t: &Task) -> Result<()> {
    let p = policy(s)?;
    ensure!(
        t.loop_state.stalled < p.max_stalled_checkpoints,
        "implementation stalled; hold needs-human with diagnosis and remaining criterion"
    );
    ensure!(
        crate::documentation::implementation_available(t)
            || s.launches
                .iter()
                .filter(|l| l.key == t.spec.key
                    && l.role == Role::Implementer
                    && !crate::documentation::exempt(t, l.id))
                .count()
                < p.max_implementation_turns
                    + t.authorities
                        .iter()
                        .filter(|authority| {
                            !crate::authority::receipt_only(authority)
                                && matches!(
                                    authority.grant,
                                    crate::authority::Grant::Pair { .. }
                                        | crate::authority::Grant::Delivery { .. }
                                )
                        })
                        .count(),
        "implementation turn budget exhausted; validate existing work or hold needs-human"
    );
    Ok(())
}
pub fn before_agent(s: &State, t: &Task, role: &Role) -> Result<Value> {
    let p = planned(s, t)?;
    ensure!(
        !needs_checkpoint(s, t),
        "checkpoint the terminal implementation turn before further agent work"
    );
    if *role == Role::Implementer {
        implementation_budget(s, t)?;
    }
    Ok(
        json!({"plan":p,"stage":crate::stages::current(t),"full_criteria":t.criteria,"delivery":t.followup,"original_base":crate::followup::original_base(t)?,"primary_pr":t.delivery_history.first().and_then(|d| d.task.pr).or(t.pr),"last_checkpoint":t.loop_state.checkpoints.last(),"guidance":"Use these scoped lessons and local examples. Feedback is guidance, not authority to change policy or acceptance. Baselines describe the starting revision; remeasure the current snapshot."}),
    )
}
fn paths(s: &State, t: &Task) -> Result<Vec<String>> {
    let snap = fresh(s, t)?;
    // --no-renames includes both old and new paths; a move across scope boundaries stays visible.
    let raw = git(
        &worktree(s, t)?,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            &format!("{}...{}", snap.base, snap.head),
        ],
    )?;
    Ok(raw
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect())
}
pub fn checkpoint(s: &State, t: &mut Task, mut c: Checkpoint) -> Result<Value> {
    let p = planned(s, t)?;
    c.snapshot = fresh(s, t)?;
    c.launch = last_implementation(s, t).map(|l| l.id);
    let new_turn = c.launch.is_some()
        && t.loop_state
            .checkpoints
            .last()
            .is_none_or(|old| old.launch != c.launch);
    ensure!(
        t.loop_state
            .checkpoints
            .last()
            .is_none_or(|old| old.launch != c.launch || old.snapshot != c.snapshot),
        "checkpoint already recorded for this implementation turn and snapshot"
    );
    required_text(&c.finding, "resolved uncertainty or remaining failure")?;
    required_text(&c.next_step, "bounded next action")?;
    c.digest = artifact(&c.artifact)?;
    if c.advanced {
        ensure!(
            !t.loop_state
                .checkpoints
                .iter()
                .any(|old| old.digest == c.digest),
            "unchanged artifact cannot establish new progress"
        );
    }
    assess_scope(s, t, p, &mut c)?;
    // A scope assessment cannot grant new business scope; its reasoning is reviewed independently.
    if new_turn {
        t.loop_state.stalled = if c.advanced {
            0
        } else {
            t.loop_state.stalled.saturating_add(1)
        };
    }
    t.loop_state.checkpoints.push(c.clone());
    t.review = None;
    t.final_verified = false;
    t.loop_state.human_review = None;
    Ok(
        json!({"checkpoint":c,"stalled":t.loop_state.stalled,"remaining_implementation_turns":policy(s)?.max_implementation_turns.saturating_sub(s.launches.iter().filter(|l| l.key == t.spec.key && l.role == Role::Implementer && !crate::documentation::exempt(t, l.id)).count())}),
    )
}
pub fn acceptance(s: &State, t: &Task) -> Result<()> {
    planned(s, t)?;
    ensure!(!needs_checkpoint(s, t), "implementation checkpoint missing");
    let snap = fresh(s, t)?;
    for c in crate::stages::criteria(t) {
        let m = t
            .loop_state
            .measurements
            .get(&c.id)
            .context("before-and-after measurement missing")?;
        ensure!(
            m.snapshot == snap && m.satisfied && artifact(&m.artifact)? == m.digest,
            "criterion measurement is stale, changed or unsatisfied"
        );
    }
    if let Some(checkpoint) = t.loop_state.checkpoints.last() {
        ensure!(
            artifact(&checkpoint.artifact)? == checkpoint.digest,
            "progress checkpoint artifact changed"
        );
    }
    let actual = paths(s, t)?;
    if !actual.is_empty() {
        let checkpoint = t
            .loop_state
            .checkpoints
            .last()
            .context("scope checkpoint required for changed code")?;
        ensure!(
            checkpoint.snapshot == snap
                && checkpoint.paths == actual
                && artifact(&checkpoint.artifact)? == checkpoint.digest,
            "scope/progress checkpoint is stale or changed"
        );
    }
    Ok(())
}
pub fn human_review(s: &State, t: &Task) -> Result<()> {
    if policy(s)?.review_mode == ReviewMode::HumanReview {
        let r =
            t.loop_state.human_review.as_ref().context(
                "explicit human review of this snapshot is required by the selected mode",
            )?;
        ensure!(
            r.snapshot == fresh(s, t)? && artifact(&r.artifact)? == r.digest,
            "human review is stale or its receipt changed"
        );
    }
    Ok(())
}
pub fn record_human(
    s: &State,
    t: &mut Task,
    snapshot: Snapshot,
    source: String,
    path: PathBuf,
) -> Result<Value> {
    crate::engine::reviewed(s, t)?;
    ensure!(
        snapshot == fresh(s, t)?,
        "human review must name the current reviewed snapshot"
    );
    required_text(
        &source,
        "actual human identity and review message or receipt",
    )?;
    let r = HumanReview {
        snapshot,
        source,
        digest: artifact(&path)?,
        artifact: path,
    };
    t.loop_state.human_review = Some(r.clone());
    Ok(json!(r))
}

fn assess_scope(s: &State, t: &Task, p: &Plan, c: &mut Checkpoint) -> Result<()> {
    c.paths = paths(s, t)?;
    let snap = fresh(s, t)?;
    let raw = git(
        &worktree(s, t)?,
        &[
            "diff",
            "--no-renames",
            "--numstat",
            "-z",
            &format!("{}...{}", snap.base, snap.head),
        ],
    )?;
    c.size.files = c.paths.len();
    for record in raw.split('\0').filter(|v| !v.is_empty()) {
        let mut columns = record.splitn(3, '\t');
        let added = columns.next().context("diff added count")?;
        let deleted = columns.next().context("diff deleted count")?;
        if added == "-" {
            c.size.binary_files += 1;
        } else {
            c.size.added += added.parse::<u64>()?;
            c.size.deleted += deleted.parse::<u64>()?;
        }
    }
    if !c.paths.is_empty() {
        required_text(
            &c.scope_reason,
            "assess actual change size and whether it remains a small complete deliverable",
        )?;
    }
    c.outside_plan = c
        .paths
        .iter()
        .filter(|path| {
            !p.input
                .components
                .iter()
                .any(|scope| *path == scope || path.starts_with(&format!("{scope}/")))
        })
        .cloned()
        .collect();
    if !c.outside_plan.is_empty() {
        required_text(
            &c.scope_reason,
            "explain why changed paths still belong to the accepted deliverable, or hold and reconcile scope",
        )?;
    }
    Ok(())
}
