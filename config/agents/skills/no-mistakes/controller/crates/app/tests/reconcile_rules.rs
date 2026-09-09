mod common;

use adapters::sqlite::Store;
use app::{App, ConflictReason, Rejection, ResultData};
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for};
use common::literals::{digest, jira_status, write_text};
use common::*;
use domain::{
    command::{AgentRole, Command, NextAction, PublishStep, RecoveryTarget},
    delivery::{DeliveryKind, Outcome, PrState, PullRequest},
    event::{Actor, Event, GitHubAction, Operation, OperationStatus},
    ids::{OperationId, PrNumber, TaskId},
    ports::MergeMethod,
};

/// `plan_and_implement`, a recorded AC1 proof and a PASS review, then a
/// `publish --step create` that reaches GitHub and fails: the
/// coordinator's own `OperationStarted` is written first, so the failure
/// (a simulated network error, `fake.create_fails_once`) leaves it
/// unsettled rather than never having happened — the shape a real
/// failure after the write takes. Returns the task and the operation
/// `next`, `recover-operation` and `observe-pr` all act on next.
fn create_fails_after_write(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<(TaskId, OperationId), Box<dyn std::error::Error>> {
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
    fake.create_fails_once.set(true);
    let result = attempt_create(app, &task, directory);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::External(_),
            ..
        })
    ));
    let operation = pending_operation(app, &task)?;
    Ok((task, operation))
}

fn attempt_create(
    app: &mut App<'_>,
    task: &TaskId,
    directory: &std::path::Path,
) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory, "pr-body.md", "delivers AC1")),
            method: None,
        },
        false,
    )
}

fn pending_operation(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<OperationId, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .operations
        .iter()
        .find(|operation| operation.task == *task)
        .map(|operation| operation.id)
        .ok_or_else(|| "pending create operation missing".into())
}

fn operation_status(
    app: &mut App<'_>,
    operation: OperationId,
) -> Result<OperationStatus, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .operations
        .iter()
        .find(|item| item.id == operation)
        .map(|item| item.status)
        .ok_or_else(|| "operation missing".into())
}

/// Reports the open draft PR GitHub actually created for `pr` against
/// `head`, the observation `observe-pr` reconciles the stalled create
/// operation against.
fn observe_created_pr(
    fake: &Fake,
    pr: PrNumber,
    head: domain::ids::Sha,
) -> Result<(), Box<dyn std::error::Error>> {
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: pr,
        state: PrState::Open,
        draft: true,
        head,
        merge: None,
        checks: Vec::new(),
    });
    Ok(())
}

fn assert_settle_operation_next(
    app: &mut App<'_>,
    operation: OperationId,
) -> Result<(), Box<dyn std::error::Error>> {
    expect_next(
        app,
        |action| matches!(action, NextAction::SettleOperation { operation: value } if *value == operation),
    )?;
    Ok(())
}

/// `recover-operation` on the stalled create is rejected: it ran
/// synchronously, so it owns no process for the coordinator to
/// terminate.
fn assert_recovery_has_no_process(
    app: &mut App<'_>,
    operation: OperationId,
) -> Result<(), Box<dyn std::error::Error>> {
    let recovery = app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Operation { operation },
            terminate: true,
        },
        false,
    );
    assert!(matches!(
        recovery,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::OperationHasNoProcess),
            ..
        })
    ));
    Ok(())
}

/// Reports the PR GitHub actually created for the current snapshot's
/// head, then reconciles the stalled create operation against it.
fn observe_pending_create(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, snapshot) = current_delivery(app, task)?;
    observe_created_pr(fake, PrNumber(1), snapshot.head)?;
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;
    Ok(())
}

fn delivery_pr_number(delivery: &domain::delivery::Delivery) -> Result<PrNumber, String> {
    match &delivery.kind {
        DeliveryKind::Code { pr: Some(pr) } => Ok(pr.number),
        other => Err(format!("expected PR recorded, found {other:?}")),
    }
}

fn assert_pr_recorded(
    app: &mut App<'_>,
    task: &TaskId,
    pr: PrNumber,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert_eq!(delivery_pr_number(delivery)?, pr);
    Ok(())
}

/// A `publish --step create` that failed after GitHub was called (design
/// Section 14: a launch or operation reaching the gate is charged, and
/// settling it is a separate step) leaves an unsettled operation `next`
/// surfaces as `SettleOperation`, not `Publish` again. `recover-operation`
/// on it is rejected: the create ran synchronously, so it owns no
/// process for the coordinator to terminate — the message instead points
/// at observing GitHub. Once the coordinator learns GitHub did create the
/// PR (`observe-pr`), the PR is recorded on the delivery and the stalled
/// operation settles `Confirmed` from that same observation, so `next`
/// moves the task on rather than leaving it stuck behind its own failed
/// write.
#[test]
fn reconcile_reobserves_unknown_only() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let (task, operation) = create_fails_after_write(&mut app, &fake, directory.path())?;

    assert_settle_operation_next(&mut app, operation)?;
    assert_recovery_has_no_process(&mut app, operation)?;
    observe_pending_create(&mut app, &fake, &task)?;

    assert_pr_recorded(&mut app, &task, PrNumber(1))?;
    assert_eq!(
        operation_status(&mut app, operation)?,
        OperationStatus::Confirmed
    );
    expect_next(
        &mut app,
        |action| matches!(action, NextAction::SyncStatus { target, .. } if *target == jira_status("review")),
    )?;
    Ok(())
}

/// Continuing the same stalled-create scenario: a second
/// `publish --step create` while the first remains unsettled is rejected
/// as a conflict, never dispatching a duplicate GitHub call; once
/// `observe-pr` records the PR GitHub actually produced, `next` moves
/// through Jira Review and on to `Publish { step: Ready }` — never
/// `Create` again.
#[test]
fn publish_failure_no_duplicate() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let (task, _operation) = create_fails_after_write(&mut app, &fake, directory.path())?;

    let retry = attempt_create(&mut app, &task, directory.path());
    assert!(matches!(
        retry,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::ActiveOperationUnsettled),
            ..
        })
    ));

    let (_, snapshot) = current_delivery(&mut app, &task)?;
    observe_created_pr(&fake, PrNumber(1), snapshot.head)?;
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;

    sync_status(&mut app, &task, "progress", "review", "51")?;
    let action = expect_next(
        &mut app,
        |action| matches!(action, NextAction::Publish { step, .. } if *step == PublishStep::Ready),
    )?;
    assert_eq!(
        action,
        NextAction::Publish {
            task: task.clone(),
            step: PublishStep::Ready,
        }
    );
    Ok(())
}

/// Records an unsettled `Merge` operation for `task`'s current delivery
/// directly, the shape a merge that failed after its own write leaves.
fn record_unsettled_merge(
    directory: &std::path::Path,
    task: &TaskId,
    head: domain::ids::Sha,
) -> Result<OperationId, Box<dyn std::error::Error>> {
    let operation = OperationId(99);
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::OperationStarted {
            operation: Operation {
                id: operation,
                task: task.clone(),
                delivery: domain::ids::DeliveryId(1),
                action: GitHubAction::Merge {
                    pr: PrNumber(1),
                    head,
                    method: MergeMethod::Squash,
                },
                status: OperationStatus::Running,
                process: None,
                timeout_seconds: 300,
            },
        }],
    )?;
    Ok(operation)
}

/// A pending merge whose PR is observed still open did not take effect:
/// `observe-pr` settles it `Failed`, never `Confirmed`, and the delivery
/// stays open.
#[test]
fn unmet_merge_settles_failed() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let (task, _create) = create_fails_after_write(&mut app, &fake, directory.path())?;
    observe_pending_create(&mut app, &fake, &task)?;
    let (_, snapshot) = current_delivery(&mut app, &task)?;
    drop(app);
    let merge = record_unsettled_merge(directory.path(), &task, snapshot.head)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;

    assert_eq!(operation_status(&mut app, merge)?, OperationStatus::Failed);
    assert!(delivery_open(&mut app, &task)?);
    Ok(())
}

fn delivery_open(app: &mut App<'_>, task: &TaskId) -> Result<bool, Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    Ok(matches!(delivery.outcome, Outcome::Open))
}
