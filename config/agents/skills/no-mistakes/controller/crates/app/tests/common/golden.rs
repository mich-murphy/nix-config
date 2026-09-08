//! Shared helpers for the golden-path end-to-end tests
//! (`../golden_path.rs`, `../golden_path_verification.rs`,
//! `../golden_path_recovery.rs`). Kept here, not in `mod.rs`, purely to
//! stay under the 400-line file gate.

use super::{Fake, prepare, task_state};
use app::App;
use domain::{
    acceptance::{Measurement, ProofEntry, ProofKind, ProofStatus, Snapshot},
    command::{AgentRole, Command},
    event::Event,
    ids::{DeliveryId, Digest, TaskId},
    review::{Review, Verdict},
};
use std::{collections::BTreeMap, str::FromStr};

pub fn prompt_file(
    directory: &std::path::Path,
    name: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let path = directory.join(name);
    std::fs::write(&path, "bounded task")?;
    Ok(path)
}

/// Predicts the reviewer's `LaunchId` with `--check` (no launch spent) and
/// installs a PASS `Review` for that launch and `snapshot` in the fake
/// harness, ready for the real `run-agent` call.
pub fn arrange_passing_review(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    prompt: &std::path::Path,
    snapshot: Snapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: prompt.to_path_buf(),
            fallback: None,
        },
        true,
    )?;
    let launch = output
        .events
        .iter()
        .find_map(|record| match &record.event {
            Event::LaunchStarted { launch } if launch.role == AgentRole::Reviewer => {
                Some(launch.id)
            }
            _ => None,
        })
        .ok_or("reviewer launch id was not predicted by --check")?;
    let review = Review {
        launch,
        session: "reviewer-session".into(),
        snapshot,
        findings: Vec::new(),
        evidence_gaps: Vec::new(),
        reviewer_opinion: Verdict::Pass,
        verdict: Verdict::Pass,
        dispositions: BTreeMap::new(),
    };
    *fake.reviewer_output.borrow_mut() = serde_json::to_string(&review)?;
    Ok(())
}

/// Discovers and claims GAIN-2, syncs it to Jira `progress`, briefs, binds
/// and plans a single `AC1` criterion, then runs one implementer turn,
/// checkpoints it and snapshots. Every golden-path test that needs a
/// planned, snapshotted delivery starts from here.
pub fn plan_and_implement(
    app: &mut App<'_>,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    prepare(app)?;
    let task = TaskId::from_str("GAIN-2")?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt_file(directory, "implement.md")?,
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced: true,
            observation: "implementation complete".into(),
            next: "record proof".into(),
            outside_paths: Vec::new(),
            scope_reason: None,
        },
        false,
    )?;
    app.execute(Command::Snapshot { task: task.clone() }, false)?;
    Ok(task)
}

/// One proof entry to record, built by `record_proof_for` (measured
/// `Command` evidence).
pub struct ProofSpec<'a> {
    pub criterion: &'a str,
    pub kind: ProofKind,
    pub measurement: Option<Measurement>,
    pub snapshot: Snapshot,
    pub artifact: std::path::PathBuf,
}

pub fn record_proof_entry(
    app: &mut App<'_>,
    task: &TaskId,
    delivery: DeliveryId,
    spec: ProofSpec<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let digest = super::write_artifact(&spec.artifact, "measured result")?;
    app.execute(
        Command::RecordProof {
            task: task.clone(),
            delivery,
            entries: vec![ProofEntry {
                criterion: spec.criterion.parse()?,
                status: ProofStatus::Passed,
                kind: spec.kind,
                artifact: spec.artifact,
                digest,
                snapshot: spec.snapshot,
                measurement: spec.measurement,
                implementation: vec!["src/lib.rs:1".into()],
                description: "exercised public behavior".into(),
            }],
        },
        false,
    )?;
    Ok(())
}

/// Records one passing `Command`-kind proof entry for `criterion` against
/// `snapshot`, measured against `baseline` (a criterion the original plan
/// fixed a baseline for).
pub fn record_proof_for(
    app: &mut App<'_>,
    task: &TaskId,
    delivery: DeliveryId,
    criterion: &str,
    baseline: Digest,
    snapshot: Snapshot,
    artifact: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    record_proof_entry(
        app,
        task,
        delivery,
        ProofSpec {
            criterion,
            kind: ProofKind::Command,
            measurement: Some(Measurement {
                baseline,
                observed: "result now present".into(),
                satisfied: true,
            }),
            snapshot,
            artifact,
        },
    )
}

pub fn task_baseline(
    app: &mut App<'_>,
    task: &TaskId,
    criterion: &str,
) -> Result<Digest, Box<dyn std::error::Error>> {
    task_state(app, task)?
        .baselines
        .get(&criterion.parse()?)
        .cloned()
        .ok_or_else(|| "task baseline missing".into())
}
