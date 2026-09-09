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
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
    })
}

fn bind(app: &mut App<'_>) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::BindSlot {
            task: TaskId::from_str("GAIN-2").map_err(|error| {
                app::AgentError::new("fixture", None, Rejection::Invalid(error.to_string()), None)
            })?,
            slot: SlotId::from_str("worker1").map_err(|error| {
                app::AgentError::new("fixture", None, Rejection::Invalid(error.to_string()), None)
            })?,
            branch: "agent/worker1/GAIN-2".into(),
            authority: None,
        },
        false,
    )
}

/// "Existing directory of unknown ownership is occupied": a slot with a
/// real checkout on disk, but no task in this run's own state recorded as
/// its owner, is not bindable fresh. Formerly named
/// `unauthorized_reuse_rejected`; renamed to the inventory name.
#[test]
fn reserve_rejects_unknown_directory() -> Result<(), Box<dyn std::error::Error>> {
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
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    assert!(
        state.tasks[&TaskId::from_str("GAIN-2")?]
            .deliveries
            .is_empty()
    );
    assert!(fake.vcs_calls.borrow().is_empty());
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
    // `bind_rejects_unsafe_checkout`: a symlink, dirty or foreign checkout
    // is rejected without effects, proven here for all three cases by the
    // fake `Vcs` never having recorded a `bind_slot`/`reuse_slot` call.
    assert!(fake.vcs_calls.borrow().is_empty());
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

/// GAIN-2 claims and binds slot `worker1`, leaving an unfinished delivery
/// there. GAIN-3 is then forced into `Claimed` by a direct event commit
/// (only one task can be the run's real `active` claim at a time, and that
/// is not the rule under test here), the shared setup for
/// `bind_rejects_second_owner` and `provision_checks_ownership_first`.
/// Discovers GAIN-2 and GAIN-3, claims `owner` and binds it to `worker1`:
/// the half of `second_owner_state`'s setup that runs through the real
/// commands, split out so the fixture's own complexity (mostly `Store`
/// plumbing) is scored separately.
fn claim_and_bind_worker1(
    app: &mut App<'_>,
    owner: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Discover {
            tasks: vec![task()?, task_named("GAIN-3")?],
            planning_order: None,
        },
        false,
    )?;
    app.execute(
        Command::Claim {
            task: owner.clone(),
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: owner.clone(),
            slot: SlotId::from_str("worker1")?,
            branch: "agent/worker1/GAIN-2".into(),
            authority: None,
        },
        false,
    )?;
    Ok(())
}

fn second_owner_state() -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let owner = TaskId::from_str("GAIN-2")?;
    let other = TaskId::from_str("GAIN-3")?;
    claim_and_bind_worker1(&mut app, &owner)?;
    drop(app);
    Store::open(directory.path())?.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::Claimed {
            task: other,
            at: 10,
        }],
    )?;
    Ok((directory, fake))
}

#[test]
fn bind_rejects_second_owner() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = second_owner_state()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = app.execute(
        Command::BindSlot {
            task: TaskId::from_str("GAIN-3")?,
            slot: SlotId::from_str("worker1")?,
            branch: "agent/worker1/GAIN-3".into(),
            authority: None,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(domain::authority::AuthorityError::SlotUnfinishedOwner),
            ..
        })
    ));
    Ok(())
}

/// Ownership is checked before any `Vcs` call: the second-owner rejection
/// above happens without ever inspecting or binding the slot.
#[test]
fn provision_checks_ownership_first() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = second_owner_state()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let before = fake.vcs_calls.borrow().len();
    let _ = app.execute(
        Command::BindSlot {
            task: TaskId::from_str("GAIN-3")?,
            slot: SlotId::from_str("worker1")?,
            branch: "agent/worker1/GAIN-3".into(),
            authority: None,
        },
        false,
    );
    assert_eq!(fake.vcs_calls.borrow().len(), before);
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
