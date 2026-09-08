mod common;

use adapters::sqlite::Store;
use app::App;
use common::golden::{plan_and_implement, record_proof_for};
use common::literals::{digest, write_text};
use common::script::{
    Step, adjust_clock, execute, nothing, pending_checks, record_proof, review, run_steps, step,
    unchecked,
};
use common::*;
use domain::{
    command::{Command, NextAction, PublishStep},
    delivery::{Check, CheckState, DeliveryKind},
    ids::{Digest, TaskId},
    task::HoldReason,
};
use std::str::FromStr;

#[test]
fn stale_proof_requires_new_proof() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;
    let (delivery, snapshot) = current_delivery(&mut app, &task)?;
    record_proof_for(
        &mut app,
        &task,
        delivery,
        "AC1",
        Digest::from_str(&"b".repeat(64))?,
        snapshot,
        directory.path().join("proof-ac1.txt"),
    )?;

    fake.head.set('b');
    app.execute(Command::Snapshot { task: task.clone() }, false)?;

    expect_next(&mut app, |action| {
        matches!(action, NextAction::RecordProof { .. })
    })?;
    let state = task_state(&mut app, &task)?;
    assert!(
        state
            .deliveries
            .last()
            .ok_or("delivery missing")?
            .proof
            .entries
            .is_empty()
    );
    Ok(())
}

/// Builds the `publish create`/`ready` half of `ci_hold_resumes_at_deadline`:
/// a passing proof and review, then a create/ready publish with a pending
/// required check baked into the fake PR from `ready` on, the way a real
/// GitHub adapter's own state would show CI still running. None of these
/// steps assert `next` first; the deadline behaviour after `poll-checks
/// --wait` is what this scenario tests.
fn publish_with_pending_checks(directory: &std::path::Path, task: &TaskId) -> Vec<Step> {
    vec![
        unchecked(record_proof(
            "AC1",
            digest('b'),
            directory.join("proof-ac1.txt"),
        )),
        unchecked(review(write_text(directory, "review.md", "bounded task"))),
        unchecked(execute(Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory, "pr-body.md", "delivers AC1")),
            method: None,
        })),
        unchecked(pending_checks(true)),
        unchecked(execute(Command::Publish {
            task: task.clone(),
            step: PublishStep::Ready,
            title: None,
            body: None,
            method: None,
        })),
        unchecked(execute(Command::PollChecks {
            task: task.clone(),
            wait: true,
        })),
    ]
}

/// Builds the deadline half: nudge the fake clock before and after the
/// deadline, resume the held task once past it, then observe checks
/// passing and confirm `next` is ready to merge.
fn resume_past_deadline(task: &TaskId) -> Vec<Step> {
    let resumed = task.clone();
    vec![
        // Before the deadline, with nothing else claimable, `next` waits
        // rather than reporting idle.
        unchecked(adjust_clock(-1)),
        step(
            |action| matches!(action, NextAction::AwaitChecks { .. }),
            nothing(),
        ),
        // Past the deadline, `next` asks to resume the held task.
        unchecked(adjust_clock(2)),
        step(
            move |action| matches!(action, NextAction::Resume { task: id } if *id == resumed),
            execute(Command::Resume {
                task: task.clone(),
                final_revisit: false,
            }),
        ),
        // The checks now report passing; a fresh observation is required
        // before `next` reflects it (`next` reads recorded state, not a
        // live GitHub call).
        unchecked(pending_checks(false)),
        unchecked(execute(Command::PollChecks {
            task: task.clone(),
            wait: false,
        })),
        step(
            |action| matches!(action, NextAction::Publish { step, .. } if *step == PublishStep::Merge),
            nothing(),
        ),
    ]
}

/// Drives GAIN-2 through `publish create` and `ready` with a pending
/// required check baked into the fake PR, then a bounded `poll-checks
/// --wait`, which loops the fake clock out to the per-head deadline,
/// records the `CiPending` hold and releases the active task.
#[test]
fn ci_hold_resumes_at_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;

    run_steps(
        &mut app,
        &fake,
        &task,
        publish_with_pending_checks(directory.path(), &task),
    )?;
    // Once a PR exists, `next` gates every further publish step on Jira
    // reflecting Review (design Section 11).
    sync_status(&mut app, &task, "progress", "review", "41")?;

    let state = task_state(&mut app, &task)?;
    assert!(matches!(
        state.hold.as_ref().map(|hold| &hold.reason),
        Some(HoldReason::CiPending { .. })
    ));

    run_steps(&mut app, &fake, &task, resume_past_deadline(&task))?;

    let state = task_state(&mut app, &task)?;
    let pr = state
        .deliveries
        .last()
        .ok_or("delivery missing")?
        .kind
        .clone();
    let DeliveryKind::Code { pr: Some(pr) } = pr else {
        return Err("expected an open code delivery".into());
    };
    assert!(
        pr.checks
            .iter()
            .all(|check: &Check| check.state == CheckState::Pass)
    );
    Ok(())
}
