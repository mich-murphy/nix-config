mod common;

use adapters::sqlite::Store;
use app::App;
use common::golden::{plan_and_implement, task_baseline};
use common::literals::{criterion_id, digest, write_text};
use common::script::{
    Step, execute, final_verify, merged_commit, record_proof, record_proof_with, review,
    review_with, run_steps, step, unchecked,
};
use common::*;
use domain::{
    acceptance::Snapshot,
    authority::{Authority, AuthorityUse, Grant},
    command::{Command, JiraRead, NextAction, PublishStep},
    delivery::DeliveryKind,
    ids::AuthorityId,
    ports::MergeMethod,
    task::{Hold, HoldReason, Phase, Plan},
};
use std::collections::BTreeMap;

/// Drives the narrowed-to-`STAGE1` delivery from `plan_and_implement`
/// through a merged, held task: `narrow-acceptance`, the re-plan that adds
/// a baseline for STAGE1 (design Section 5: replanning cannot change a
/// baseline target, but may add one for a criterion that had none), a
/// fresh snapshot (narrowing clears the one taken before it), measured
/// proof, a PASS review, and publish create/ready/merge.
fn narrow_record_review_and_merge(
    app: &mut App<'_>,
    fake: &Fake,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let receipt = directory.join("narrow-receipt.txt");
    let narrow_digest = write_artifact(&receipt, "narrow to STAGE1 for early merge")?;
    let ac1_baseline = task_baseline(app, task, "AC1")?;
    let stage1_baseline = digest('e');

    run_steps(
        app,
        fake,
        task,
        vec![
            unchecked(execute(Command::NarrowAcceptance {
                task: task.clone(),
                criteria: vec![criterion_id("STAGE1")],
                authority: Box::new(Authority {
                    id: AuthorityId(1),
                    source: "current user".into(),
                    artifact: receipt,
                    digest: narrow_digest,
                    requirements,
                    recorded: 10,
                    grant: Grant::Narrowing {
                        criteria: vec![criterion_id("STAGE1")],
                        operational_proof: "narrowed merge lands behind a flag".into(),
                        cleanup_plan: "finish AC1 through open-delivery after merge".into(),
                    },
                    used: AuthorityUse::default(),
                }),
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
        ],
    )
}

/// Builds the `open-delivery` step that resumes full AC1 acceptance
/// against the narrowed merge, once the coordinator has a fresh Jira read
/// and an authority receipt for it.
fn open_verification_delivery(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<Step, Box<dyn std::error::Error>> {
    let commit = merged_commit(app, task)?;
    let requirements = task_state(app, task)?.spec.requirements;
    let receipt = directory.join("open-delivery-receipt.txt");
    let receipt_digest =
        write_artifact(&receipt, "resume full acceptance after the narrowed merge")?;
    Ok(step(
        |action| matches!(action, NextAction::OpenDelivery { .. }),
        execute(Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: commit.clone() },
            authority: Box::new(Authority {
                id: AuthorityId(2),
                source: "current user".into(),
                artifact: receipt,
                digest: receipt_digest,
                requirements: requirements.clone(),
                recorded: 20,
                grant: Grant::Delivery {
                    kind: DeliveryKind::Verification { of: commit },
                    criteria: vec![criterion_id("AC1")],
                    paths: None,
                },
                used: AuthorityUse::default(),
            }),
            jira: JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        }),
    ))
}

/// Asserts the task is held on `NeedsHuman`, the hold `narrow-acceptance`
/// leaves behind once its delivery has merged with acceptance still open.
fn assert_held_for_human(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    let held = matches!(
        state.hold,
        Some(Hold {
            reason: HoldReason::NeedsHuman { .. },
            ..
        })
    );
    if held {
        Ok(())
    } else {
        Err(format!("expected a NeedsHuman hold, found {:?}", state.hold).into())
    }
}

/// Builds the rest of the script once the narrowed merge is held on
/// `NeedsHuman`: `open-delivery` against it, then measured proof, a PASS
/// review and `final-verify` against the same snapshot (the merge commit
/// as both base and head, since a `Verification` delivery reopens no new
/// commits).
fn verification_script(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<Vec<Step>, Box<dyn std::error::Error>> {
    let open_delivery = open_verification_delivery(app, task, directory)?;
    let commit = merged_commit(app, task)?;
    let requirements = task_state(app, task)?.spec.requirements;
    let snapshot = Snapshot {
        base: commit.clone(),
        head: commit,
        requirements,
    };
    let ac1_baseline = task_baseline(app, task, "AC1")?;
    Ok(vec![
        open_delivery,
        step(
            |action| matches!(action, NextAction::RecordProof { .. }),
            record_proof_with(
                "AC1",
                ac1_baseline,
                snapshot.clone(),
                directory.join("proof-final-ac1.txt"),
            ),
        ),
        step(
            |action| matches!(action, NextAction::Review { .. }),
            review_with(
                write_text(directory, "review-final.md", "bounded task"),
                snapshot,
            ),
        ),
        step(
            |action| matches!(action, NextAction::FinalVerify { .. }),
            final_verify(directory.join("final-verify.txt")),
        ),
    ])
}

#[test]
fn verification_delivery_reaches_verified() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;

    // Same flow as `code_delivery_reaches_cleanup`, through `record-proof`
    // only: the coordinator then narrows to STAGE1 instead of reviewing
    // the full AC1 delivery, so this recorded proof is superseded, not
    // reused, by `narrow-acceptance` clearing it.
    run_steps(
        &mut app,
        &fake,
        &task,
        vec![unchecked(record_proof(
            "AC1",
            digest('b'),
            directory.path().join("proof-ac1.txt"),
        ))],
    )?;
    narrow_record_review_and_merge(&mut app, &fake, &task, directory.path())?;
    assert_held_for_human(&mut app, &task)?;

    let script = verification_script(&mut app, &task, directory.path())?;
    run_steps(&mut app, &fake, &task, script)?;

    let state = task_state(&mut app, &task)?;
    assert!(matches!(state.phase, Phase::Verified { .. }));
    Ok(())
}
