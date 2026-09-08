mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::*;
use domain::{
    command::Command,
    ids::{SlotId, TaskId},
    ports::SlotState,
};
use std::str::FromStr;

fn app_with_slot(slot: SlotState) -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    initialized_with(Fake {
        reserve: true,
        slot,
        observation: std::cell::RefCell::new(None),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
    })
}

fn bind(app: &mut App<'_>) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::BindSlot {
            task: TaskId::from_str("GAIN-2").map_err(|error| app::AgentError {
                failed: "fixture".into(),
                phase: None,
                why: Rejection::Invalid(error.to_string()),
                next: None,
            })?,
            slot: SlotId::from_str("worker1").map_err(|error| app::AgentError {
                failed: "fixture".into(),
                phase: None,
                why: Rejection::Invalid(error.to_string()),
                next: None,
            })?,
            branch: "agent/worker1/GAIN-2".into(),
            authority: None,
        },
        false,
    )
}

#[test]
fn unauthorized_reuse_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = app_with_slot(SlotState::Checkout {
        clean: true,
        head: sha('a')?,
    })?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    discover_claim(&mut app)?;
    sync_progress(&mut app, &TaskId::from_str("GAIN-2")?)?;
    brief_only(&mut app)?;
    assert!(matches!(
        bind(&mut app),
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    let ResultData::State { state } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    assert!(
        state.tasks[&TaskId::from_str("GAIN-2")?]
            .deliveries
            .is_empty()
    );
    Ok(())
}

fn unsafe_slot_rejected(reason: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = app_with_slot(SlotState::Unsafe(reason.into()))?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    discover_claim(&mut app)?;
    sync_progress(&mut app, &TaskId::from_str("GAIN-2")?)?;
    brief_only(&mut app)?;
    assert!(matches!(
        bind(&mut app),
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    Ok(())
}

#[test]
fn symlink_slot_rejected() -> Result<(), Box<dyn std::error::Error>> {
    unsafe_slot_rejected("symlink")
}

#[test]
fn foreign_slot_rejected() -> Result<(), Box<dyn std::error::Error>> {
    unsafe_slot_rejected("foreign checkout")
}

#[test]
fn dirty_slot_rejected() -> Result<(), Box<dyn std::error::Error>> {
    unsafe_slot_rejected("dirty checkout")
}

#[test]
fn cleanup_rejects_unfinished_slot() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let result = app.execute(
        Command::Cleanup {
            task: TaskId::from_str("GAIN-2")?,
            delete: false,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    Ok(())
}

fn discover_claim(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    app.execute(
        Command::Claim {
            task: TaskId::from_str("GAIN-2")?,
        },
        false,
    )?;
    Ok(())
}

fn brief_only(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Brief {
            task: TaskId::from_str("GAIN-2")?,
            criteria: vec![domain::task::Criterion {
                id: "AC1".parse()?,
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    Ok(())
}
