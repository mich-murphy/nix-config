mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::*;
use domain::{
    command::Command,
    delivery::{PrState, PullRequest},
    event::{Actor, Event},
    ids::{DeliveryId, PrNumber, TaskId},
    task::HoldReason,
};
use std::str::FromStr;

fn pull_request(state: PrState) -> Result<PullRequest, domain::ports::PortError> {
    Ok(PullRequest {
        number: PrNumber(1),
        state,
        draft: false,
        head: sha('a')?,
        merge: (state == PrState::Merged).then(|| sha('b')).transpose()?,
        checks: Vec::new(),
    })
}

fn held_for_human(app: &mut App<'_>) -> Result<bool, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    Ok(matches!(
        state.tasks[&TaskId::from_str("GAIN-2")?]
            .hold
            .as_ref()
            .map(|hold| &hold.reason),
        Some(HoldReason::NeedsHuman { .. })
    ))
}

fn prepared_external_merge() -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    let merged = pull_request(PrState::Merged)?;
    let (directory, fake) = initialized_with(Fake {
        reserve: true,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(Some(merged)),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
    })?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    drop(app);
    Store::open(directory.path())?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: TaskId::from_str("GAIN-2")?,
            delivery: DeliveryId(1),
            pr: pull_request(PrState::Open)?,
        }],
    )?;
    Ok((directory, fake))
}

#[test]
fn external_merge_holds() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = prepared_external_merge()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::ObservePr {
            task: TaskId::from_str("GAIN-2")?,
            pr: PrNumber(1),
        },
        false,
    )?;
    assert!(held_for_human(&mut app)?);
    Ok(())
}
