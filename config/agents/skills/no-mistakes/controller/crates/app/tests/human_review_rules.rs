mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for};
use common::literals::{digest, write_text};
use common::*;
use domain::{
    command::{AgentRole, Command, PublishStep, ReviewMode},
    ids::TaskId,
    ports::MergeMethod,
    task::Phase,
};

/// The same fixed `Fake` `initialized()` builds, exposed here so a test
/// can hand it to `initialized_with_review_mode` instead (design Section
/// 14: human-review mode needs its own run config, not the fixed
/// `Autonomous` every other fixture uses).
fn base_fake() -> Fake {
    Fake {
        reserve: true,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(None),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
        last_prompt: std::cell::RefCell::new(None),
        create_fails_once: std::cell::Cell::new(false),
        spawned: std::cell::RefCell::new(Vec::new()),
    }
}

/// `plan_and_implement`, then one recorded AC1 proof and a PASS reviewer
/// launch: the current snapshot has a settled review and no human
/// receipt yet, the starting point both `human_receipt_pins_snapshot` and
/// `human_receipt_gates_merge` need.
fn planned_with_pass_review(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = plan_and_implement(app, directory)?;
    let (delivery, snapshot) = current_delivery(app, &task)?;
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
    Ok(task)
}

/// The current delivery's own human-review receipt, if any, read back
/// through `Command::Status` rather than a 40-field literal.
fn delivery_human_review(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<Option<domain::delivery::HumanReceipt>, Box<dyn std::error::Error>> {
    Ok(task_state(app, task)?
        .deliveries
        .last()
        .ok_or("delivery missing")?
        .human_review
        .clone())
}

/// `human-review` for a snapshot other than the current one is rejected
/// `Evidence`, without ever recording a receipt.
fn assert_stale_snapshot_rejected(
    app: &mut App<'_>,
    task: &TaskId,
    current: &domain::acceptance::Snapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stale = current.clone();
    stale.head = sha('9')?;
    let rejected = app.execute(
        Command::HumanReview {
            task: task.clone(),
            snapshot: stale,
            receipt: digest('c'),
        },
        false,
    );
    assert!(matches!(
        rejected,
        Err(app::AgentError {
            why: Rejection::Evidence(_),
            ..
        })
    ));
    Ok(())
}

/// `human-review` for the current snapshot is recorded onto the
/// delivery, readable back through `Command::Status`.
fn record_and_verify_receipt(
    app: &mut App<'_>,
    task: &TaskId,
    current: &domain::acceptance::Snapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::HumanReview {
            task: task.clone(),
            snapshot: current.clone(),
            receipt: digest('c'),
        },
        false,
    )?;
    let recorded = delivery_human_review(app, task)?.ok_or("human review receipt missing")?;
    assert_eq!(recorded.snapshot, *current);
    assert_eq!(recorded.receipt, digest('c'));
    Ok(())
}

/// A later `snapshot`, taken against a new head, clears the recorded
/// receipt: a human receipt pins one exact snapshot, not the delivery
/// generally.
fn assert_cleared_after_resnapshot(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    fake.head.set('c');
    app.execute(Command::Snapshot { task: task.clone() }, false)?;
    assert!(delivery_human_review(app, task)?.is_none());
    Ok(())
}

/// In `ReviewMode::HumanReview`, `human-review` for a snapshot other than
/// the current one is rejected `Evidence`; the matching snapshot is
/// recorded onto the delivery, readable back through `Command::Status`;
/// and a later `snapshot` (taken against a new head) clears it, since a
/// human receipt pins one exact snapshot, not the delivery generally.
#[test]
fn human_receipt_pins_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_review_mode(base_fake(), ReviewMode::HumanReview)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = planned_with_pass_review(&mut app, &fake, directory.path())?;
    let (_, current) = current_delivery(&mut app, &task)?;

    assert_stale_snapshot_rejected(&mut app, &task, &current)?;
    record_and_verify_receipt(&mut app, &task, &current)?;
    assert_cleared_after_resnapshot(&mut app, &fake, &task)?;
    Ok(())
}

/// Runs the request `publish --step merge` builds, without asserting
/// `next` first: both `human_receipt_gates_merge` calls need the raw
/// result, once rejected and once accepted.
fn attempt_merge(app: &mut App<'_>, task: &TaskId) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Merge,
            title: None,
            body: None,
            method: Some(MergeMethod::Squash),
        },
        false,
    )
}

/// In `ReviewMode::HumanReview`, `publish --step merge` is rejected while
/// the current snapshot carries no human receipt, and succeeds once one
/// is recorded for it.
#[test]
fn human_receipt_gates_merge() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_review_mode(base_fake(), ReviewMode::HumanReview)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = planned_with_pass_review(&mut app, &fake, directory.path())?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory.path(), "pr-body.md", "delivers AC1")),
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

    let rejected = attempt_merge(&mut app, &task);
    assert!(matches!(
        rejected,
        Err(app::AgentError {
            why: Rejection::Evidence(_),
            ..
        })
    ));
    assert!(!matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Merged { .. }
    ));

    let (_, current) = current_delivery(&mut app, &task)?;
    app.execute(
        Command::HumanReview {
            task: task.clone(),
            snapshot: current,
            receipt: digest('c'),
        },
        false,
    )?;
    attempt_merge(&mut app, &task)?;
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Merged { .. }
    ));
    Ok(())
}
