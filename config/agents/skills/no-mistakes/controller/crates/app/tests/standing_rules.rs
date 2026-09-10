mod common;

use adapters::sqlite::Store;
use app::{App, ConflictReason, Rejection};
use common::golden::plan_implement_and_merge_externally;
use common::literals::slot_id;
use common::*;
use domain::{
    command::{Command, JiraRead},
    delivery::DeliveryKind,
    ids::TaskId,
    task::Phase,
};

fn open_without_receipt(
    app: &mut App<'_>,
    task: &TaskId,
    kind: DeliveryKind,
) -> Result<app::Output, Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    Ok(app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind,
            receipt: None,
            jira: JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        },
        false,
    )?)
}

/// A task held on `NeedsHuman` after an external merge reopens as a
/// fresh code delivery under the run config's standing authorization:
/// no receipt file, no authority record, the hold released, and
/// `bind-slot` accepted for the new delivery. Before this rule the only
/// way on was a human-authored receipt for every follow-up.
#[test]
fn held_task_reopens_under_standing_grant() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_merge_externally(&mut app, &fake, directory.path())?;

    open_without_receipt(&mut app, &task, DeliveryKind::Code { pr: None })?;

    let state = task_state(&mut app, &task)?;
    assert_eq!(state.deliveries.len(), 2);
    assert!(state.hold.is_none());
    assert!(matches!(state.phase, Phase::Claimed));
    assert!(state.authorities.is_empty());
    assert!(state.deliveries[1].authority.is_none());
    app.execute(
        Command::BindSlot {
            task: task.clone(),
            slot: slot_id("worker2"),
            branch: "agent/worker2/GAIN-2".into(),
            authority: None,
        },
        false,
    )?;
    Ok(())
}

/// The standing authorization is a count: with none configured, a
/// receipt-free code follow-up is rejected and the task stays held with
/// its single delivery.
#[test]
fn standing_follow_ups_are_counted() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_follow_ups(0)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_merge_externally(&mut app, &fake, directory.path())?;

    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let result = app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Code { pr: None },
            receipt: None,
            jira: JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        },
        false,
    );

    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::ReceiptRequired),
            ..
        })
    ));
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.deliveries.len(), 1);
    assert!(state.hold.is_some());
    Ok(())
}

/// A verification follow-up writes no code, so it never counts against
/// the standing grant: with none configured it still opens, against the
/// merged commit, and the task moves to `Merged` for proof and review.
#[test]
fn verification_follow_up_is_free() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_follow_ups(0)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_merge_externally(&mut app, &fake, directory.path())?;

    open_without_receipt(
        &mut app,
        &task,
        DeliveryKind::Verification { of: sha('a')? },
    )?;

    let state = task_state(&mut app, &task)?;
    assert_eq!(state.deliveries.len(), 2);
    assert!(state.hold.is_none());
    assert!(matches!(state.phase, Phase::Merged { .. }));
    assert!(matches!(
        state.deliveries[1].kind,
        DeliveryKind::Verification { .. }
    ));
    Ok(())
}
