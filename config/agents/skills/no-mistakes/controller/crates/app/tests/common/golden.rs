//! Shared helpers for the golden-path end-to-end tests
//! (`../golden_path.rs`, `../golden_path_verification.rs`,
//! `../golden_path_recovery.rs`). Kept here, not in `mod.rs`, purely to
//! stay under the 400-line file gate.

use super::literals::{criterion_id, digest, write_text};
use super::script::{execute, move_head, record_proof, review, run_steps, unchecked};
use super::{Fake, prepare, sha, task_state, write_artifact};
use adapters::sqlite::Store;
use app::App;
use domain::{
    acceptance::{Measurement, ProofEntry, ProofKind, ProofStatus, Snapshot},
    authority::{AuthorityReceipt, Grant},
    command::{AgentRole, Command, PublishStep},
    delivery::{PrState, PullRequest},
    event::{Actor, Event},
    ids::{DeliveryId, Digest, PrNumber, TaskId},
    ports::MergeMethod,
    review::{ReviewReport, Verdict},
    task::Plan,
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

/// Installs a PASS `ReviewReport` in the fake harness, ready for the real
/// `run-agent` call: the reviewer emits only findings, evidence gaps and
/// its own opinion (design Section 5, batch B item 4); the controller
/// supplies the launch, session, snapshot, computed verdict and empty
/// dispositions itself, so no `--check` prediction is needed here.
pub fn arrange_passing_review(fake: &Fake) -> Result<(), Box<dyn std::error::Error>> {
    let report = ReviewReport {
        findings: Vec::new(),
        evidence_gaps: Vec::new(),
        reviewer_opinion: Verdict::Pass,
    };
    *fake.reviewer_output.borrow_mut() = serde_json::to_string(&report)?;
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

/// `plan_and_implement`, then an external merge observed with no proof or
/// review ever recorded: a `Merged` delivery whose acceptance is left
/// open, holding `NeedsHuman` (`external_merge_holds`'s shape), the
/// starting point every follow-up-delivery rule needs.
pub fn plan_implement_and_merge_externally(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = plan_and_implement(app, directory)?;
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: task.clone(),
            delivery: DeliveryId(1),
            pr: PullRequest {
                number: PrNumber(1),
                state: PrState::Open,
                draft: false,
                head: sha('a')?,
                merge: None,
                checks: Vec::new(),
            },
        }],
    )?;
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: PrNumber(1),
        state: PrState::Merged,
        draft: false,
        head: sha('a')?,
        merge: Some(sha('a')?),
        checks: Vec::new(),
    });
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;
    Ok(task)
}

/// Drives the narrowed-to-`STAGE1` delivery from `plan_and_implement`
/// through a merged, held task: `narrow-acceptance`, the re-plan that adds
/// a baseline for STAGE1 (design Section 5: replanning cannot change a
/// baseline target, but may add one for a criterion that had none), a
/// fresh snapshot (narrowing clears the one taken before it), measured
/// proof, a PASS review, and publish create/ready/merge. When `new_head`
/// is set, the fake's head moves to it right after the re-plan, so the
/// narrowed snapshot (and the review and merge that follow) are taken
/// against that new commit rather than the one `plan_and_implement` left
/// behind.
pub fn narrow_record_review_and_merge(
    app: &mut App<'_>,
    fake: &Fake,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
    new_head: Option<char>,
) -> Result<(), Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let receipt = directory.join("narrow-receipt.txt");
    let narrow_digest = write_artifact(&receipt, "narrow to STAGE1 for early merge")?;
    let ac1_baseline = task_baseline(app, task, "AC1")?;
    let stage1_baseline = digest('e');

    let mut steps = vec![
        unchecked(execute(Command::NarrowAcceptance {
            task: task.clone(),
            criteria: vec![criterion_id("STAGE1")],
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact: receipt,
                digest: narrow_digest,
                requirements,
                grant: Grant::Narrowing {
                    criteria: vec![criterion_id("STAGE1")],
                    operational_proof: "narrowed merge lands behind a flag".into(),
                    cleanup_plan: "finish AC1 through open-delivery after merge".into(),
                },
            },
        })),
        unchecked(execute(Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "narrow to STAGE1 for early merge".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([
                    (criterion_id("AC1"), ac1_baseline),
                    (criterion_id("STAGE1"), stage1_baseline.clone()),
                ]),
                lesson_families: vec!["controller".into()],
            },
        })),
    ];
    if let Some(head) = new_head {
        steps.push(unchecked(move_head(head)));
    }
    steps.extend([
        // Narrowing clears the snapshot taken before it (design
        // Section 5): nothing changed on disk, so retaking it
        // reproduces the same base and head, but a snapshot must
        // exist for the reviewer to have a target.
        unchecked(execute(Command::Snapshot { task: task.clone() })),
        unchecked(record_proof(
            "STAGE1",
            stage1_baseline,
            directory.join("proof-stage1.txt"),
        )),
        unchecked(review(write_text(
            directory,
            "review-stage1.md",
            "bounded task",
        ))),
        unchecked(execute(Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Narrow to STAGE1".into()),
            body: Some(write_text(directory, "pr-body.md", "narrows to STAGE1")),
            method: None,
        })),
        unchecked(execute(Command::Publish {
            task: task.clone(),
            step: PublishStep::Ready,
            title: None,
            body: None,
            method: None,
        })),
        unchecked(execute(Command::Publish {
            task: task.clone(),
            step: PublishStep::Merge,
            title: None,
            body: None,
            method: Some(MergeMethod::Squash),
        })),
    ]);
    run_steps(app, fake, task, steps)
}

/// `plan_and_implement`, then a straight (non-narrowed) AC1 delivery
/// through a PASS review and publish create/ready/merge: the starting
/// point `final_verify_checks_main_head` and `verified_task` both need, a
/// task sitting in `Phase::Merged` with a recorded review to check the
/// final-verify commit and head against.
pub fn plan_review_and_merge(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = plan_and_implement(app, directory)?;
    let (delivery, snapshot) = super::current_delivery(app, &task)?;
    record_proof_for(
        app,
        &task,
        delivery,
        "AC1",
        digest('b'),
        snapshot,
        directory.join("proof-ac1.txt"),
    )?;
    arrange_passing_review(fake)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: write_text(directory, "review.md", "bounded task"),
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory, "pr-body.md", "delivers AC1")),
            method: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Ready,
            title: None,
            body: None,
            method: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Merge,
            title: None,
            body: None,
            method: Some(MergeMethod::Squash),
        },
        false,
    )?;
    Ok(task)
}

/// `plan_review_and_merge`, then a successful `final-verify` against the
/// recorded merge commit: the task reaches `Phase::Verified`, the starting
/// point `done_revalidates_evidence` and `complete_requires_verified_and_done`
/// both need.
pub fn verified_task(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = plan_review_and_merge(app, fake, directory)?;
    let commit = super::script::merged_commit(app, &task)?;
    let evidence = write_text(directory, "final-verify.txt", "final verification evidence");
    app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit,
            evidence,
        },
        false,
    )?;
    Ok(task)
}
