mod common;

use adapters::sqlite::Store;
use app::{App, ConflictReason, Rejection};
use common::golden::{arrange_passing_review, record_proof_for};
use common::literals::{criterion_id, digest, write_text};
use common::*;
use domain::{
    acceptance::Snapshot,
    command::{AgentRole, Command, JiraRead, NextAction},
    delivery::DeliveryKind,
    ids::{DeliveryId, TaskId},
    task::{Criterion, Phase, Plan},
};
use std::{collections::BTreeMap, str::FromStr};

fn jira_read(requirements: domain::ids::Digest) -> JiraRead {
    JiraRead {
        member: true,
        resolved: false,
        ownership_clear: true,
        requirements,
    }
}

/// Discovers, claims, syncs and briefs GAIN-2 with one automated
/// criterion, then opens its first delivery as verification of a commit
/// already on main, with no receipt.
fn open_first_verification(
    app: &mut App<'_>,
) -> Result<(TaskId, domain::ids::Sha), Box<dyn std::error::Error>> {
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(app, &task)?;
    sync_progress(app, &task)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![Criterion {
                id: criterion_id("AC1"),
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    let commit = sha('a')?;
    let requirements = task_state(app, &task)?.spec.requirements;
    app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: commit.clone() },
            receipt: None,
            jira: jira_read(requirements),
        },
        false,
    )?;
    Ok((task, commit))
}

/// `plan` for the verification delivery: only its baselines matter, since
/// there is no worktree to plan work in.
fn plan_baselines(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "already delivered".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([(criterion_id("AC1"), digest('b'))]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    Ok(())
}

/// Measured AC1 proof against the verified commit (base and head alike),
/// then a PASS review of it.
fn prove_and_review(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    commit: &domain::ids::Sha,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    record_proof_for(
        app,
        task,
        DeliveryId(1),
        "AC1",
        digest('b'),
        Snapshot {
            base: commit.clone(),
            head: commit.clone(),
            requirements,
        },
        directory.join("proof-ac1.txt"),
    )?;
    expect_next(app, |action| matches!(action, NextAction::Review { .. }))?;
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
    Ok(())
}

/// `final-verify` against the verified commit, Jira Done, and `complete`.
fn verify_and_complete(
    app: &mut App<'_>,
    task: &TaskId,
    commit: domain::ids::Sha,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit,
            evidence: write_text(directory, "final-verify.txt", "evidence"),
        },
        false,
    )?;
    sync_status(app, task, "progress", "done", "51")?;
    expect_next(app, |action| matches!(action, NextAction::Complete { .. }))?;
    app.execute(Command::Complete { task: task.clone() }, false)?;
    Ok(())
}

/// Drives an opened verification delivery through to `complete`,
/// asserting `next` names each step first: plan (baselines), proof and
/// review, final verification, Jira Done and completion.
fn drive_to_complete(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    commit: &domain::ids::Sha,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    expect_next(app, |action| matches!(action, NextAction::Plan { .. }))?;
    plan_baselines(app, task)?;
    expect_next(app, |action| {
        matches!(action, NextAction::RecordProof { .. })
    })?;
    prove_and_review(app, fake, task, commit, directory)?;
    expect_next(
        app,
        |action| matches!(action, NextAction::FinalVerify { commit: value, .. } if value == commit),
    )?;
    verify_and_complete(app, task, commit.clone(), directory)
}

/// Work that an earlier run already merged is verified, not re-delivered:
/// the receipt-free verification delivery moves the task to `Merged` at
/// that commit, `next` asks for a plan (to fix the baselines proof is
/// measured against), then proof, review and final verification, and the
/// completed task releases the queue without a slot to clean up. Before
/// this rule the only way through was a zero-diff code delivery that
/// could never open a PR, ending in a needs-human hold.
#[test]
fn merged_work_verifies_without_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let (task, commit) = open_first_verification(&mut app)?;
    let state = task_state(&mut app, &task)?;
    assert!(matches!(state.phase, Phase::Merged { .. }));
    assert!(state.authorities.is_empty());

    drive_to_complete(&mut app, &fake, &task, &commit, directory.path())?;

    let state = task_state(&mut app, &task)?;
    assert!(matches!(state.phase, Phase::Completed { .. }));
    expect_next(
        &mut app,
        |action| matches!(action, NextAction::Report { remaining } if remaining.is_empty()),
    )?;
    Ok(())
}

/// Once a task has a delivery, a receipt-free `open-delivery` is the
/// standing follow-up path, which needs a needs-human hold like any
/// later delivery: on an unheld task it is a conflict and nothing opens.
#[test]
fn later_delivery_still_needs_hold() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let result = app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: sha('a')? },
            receipt: None,
            jira: jira_read(requirements),
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::OpenRequiresNeedsHumanHold),
            ..
        })
    ));
    assert_eq!(task_state(&mut app, &task)?.deliveries.len(), 1);
    Ok(())
}

/// A commit that is not on main cannot be verified as delivered: the
/// receipt-free opening is rejected and the task keeps no delivery.
#[test]
fn verification_commit_must_be_on_main() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    fake.on_main.set(false);
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(&mut app, &task)?;
    sync_progress(&mut app, &task)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![Criterion {
                id: criterion_id("AC1"),
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let result = app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: sha('a')? },
            receipt: None,
            jira: jira_read(requirements),
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::VerificationCommitNotOnMain),
            ..
        })
    ));
    assert!(task_state(&mut app, &task)?.deliveries.is_empty());
    Ok(())
}
