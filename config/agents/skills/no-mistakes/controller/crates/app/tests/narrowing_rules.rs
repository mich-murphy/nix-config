mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for, task_baseline};
use common::literals::{criterion_id, digest, write_text};
use common::*;
use domain::{
    authority::{AuthorityReceipt, Grant},
    command::{AgentRole, Command, JiraRead, PublishStep},
    delivery::DeliveryKind,
    ids::{Digest, TaskId},
    ports::MergeMethod,
    task::{HoldReason, Plan},
};

fn open_delivery_command(task: &TaskId, receipt: AuthorityReceipt, jira: JiraRead) -> Command {
    Command::OpenDelivery {
        task: task.clone(),
        kind: DeliveryKind::Code { pr: None },
        receipt: Some(receipt),
        jira,
    }
}

/// A `Grant::Delivery` receipt for resuming AC1 as a fresh code delivery,
/// against `requirements`.
fn ac1_receipt(
    directory: &std::path::Path,
    requirements: &Digest,
) -> Result<AuthorityReceipt, Box<dyn std::error::Error>> {
    let artifact = directory.join("open-receipt.txt");
    let digest = write_artifact(&artifact, "resume AC1")?;
    Ok(AuthorityReceipt {
        source: "current user".into(),
        artifact,
        digest,
        requirements: requirements.clone(),
        grant: Grant::Delivery {
            kind: DeliveryKind::Code { pr: None },
            criteria: vec![criterion_id("AC1")],
            paths: None,
        },
    })
}

fn ac1_jira_read(requirements: &Digest) -> JiraRead {
    JiraRead {
        member: true,
        resolved: false,
        ownership_clear: true,
        requirements: requirements.clone(),
    }
}

/// Narrows the open code delivery to STAGE1, the shape
/// `narrowing_blocks_open_delivery` needs before it can attempt a
/// follow-up delivery against it.
fn narrow_to_stage1(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
    requirements: &Digest,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact = directory.join("narrow-receipt.txt");
    let digest = write_artifact(&artifact, "narrow to STAGE1")?;
    app.execute(
        Command::NarrowAcceptance {
            task: task.clone(),
            criteria: vec![criterion_id("STAGE1")],
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact,
                digest,
                requirements: requirements.clone(),
                grant: Grant::Narrowing {
                    criteria: vec![criterion_id("STAGE1")],
                    operational_proof: "narrowed merge lands behind a flag".into(),
                    cleanup_plan: "finish AC1 through open-delivery after merge".into(),
                },
            },
        },
        false,
    )?;
    Ok(())
}

/// Plans, records proof for and reviews the narrowed STAGE1 delivery, then
/// publishes and merges it, leaving full AC1 acceptance open (design
/// Section 5: a narrowed merge never completes acceptance).
fn plan_record_review_and_merge_stage1(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let ac1_baseline = task_baseline(app, task, "AC1")?;
    let stage1_baseline = digest('e');
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "narrow to STAGE1 for early merge".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: std::collections::BTreeMap::from([
                    (criterion_id("AC1"), ac1_baseline),
                    (criterion_id("STAGE1"), stage1_baseline.clone()),
                ]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    app.execute(Command::Snapshot { task: task.clone() }, false)?;
    let (delivery, snapshot) = current_delivery(app, task)?;
    record_proof_for(
        app,
        task,
        delivery,
        "STAGE1",
        stage1_baseline,
        snapshot,
        directory.join("proof-stage1.txt"),
    )?;
    arrange_passing_review(fake)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: write_text(directory, "review-stage1.md", "bounded task"),
            fallback: None,
        },
        false,
    )?;
    publish_stage1(app, directory, task)
}

/// Asserts `task` is held `NeedsHuman`, the hold a narrowed merge with
/// acceptance still open leaves behind.
fn assert_held_needs_human(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let held = task_state(app, task)?;
    assert!(matches!(
        held.hold.as_ref().map(|hold| &hold.reason),
        Some(HoldReason::NeedsHuman { .. })
    ));
    Ok(())
}

fn publish_stage1(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Narrow to STAGE1".into()),
            body: Some(write_text(directory, "pr-body.md", "narrows to STAGE1")),
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
    Ok(())
}

/// Attempts `open-delivery` for AC1 and asserts it is rejected on conflict,
/// leaving the delivery count unchanged: the still-open narrowed delivery
/// has no `NeedsHuman` hold to authorize it.
fn assert_open_blocked(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
    requirements: &Digest,
) -> Result<(), Box<dyn std::error::Error>> {
    let deliveries_before = task_state(app, task)?.deliveries.len();
    let blocked = app.execute(
        open_delivery_command(
            task,
            ac1_receipt(directory, requirements)?,
            ac1_jira_read(requirements),
        ),
        false,
    );
    assert!(matches!(
        blocked,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    assert_eq!(task_state(app, task)?.deliveries.len(), deliveries_before);
    Ok(())
}

/// Attempts `open-delivery` for AC1 again, once the narrowed delivery has
/// merged, and asserts it now succeeds.
fn assert_open_accepted(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
    requirements: &Digest,
) -> Result<(), Box<dyn std::error::Error>> {
    let accepted = app.execute(
        open_delivery_command(
            task,
            ac1_receipt(directory, requirements)?,
            ac1_jira_read(requirements),
        ),
        false,
    );
    assert!(accepted.is_ok());
    Ok(())
}

/// After `narrow-acceptance` on the open code delivery, `open-delivery` is
/// rejected (the delivery has no `NeedsHuman` hold to authorize it while
/// still open); once the narrowed delivery merges, leaving that hold
/// behind, `open-delivery` succeeds.
#[test]
fn narrowing_blocks_open_delivery() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    narrow_to_stage1(&mut app, directory.path(), &task, &requirements)?;
    assert_open_blocked(&mut app, directory.path(), &task, &requirements)?;

    plan_record_review_and_merge_stage1(&mut app, &fake, directory.path(), &task)?;
    assert_held_needs_human(&mut app, &task)?;
    assert_open_accepted(&mut app, directory.path(), &task, &requirements)?;
    Ok(())
}
