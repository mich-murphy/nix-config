mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::*;
use domain::command::Command;

#[test]
fn check_flag_writes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let preview = app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        true,
    )?;
    assert!(!preview.events.is_empty());
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state } => assert!(!state.frozen),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn command_returns_events() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let output = app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    assert_eq!(output.events.len(), 1);
    Ok(())
}

#[test]
fn next_fills_command_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let output = app.execute(Command::Next, false)?;
    match output.result {
        ResultData::Next { next: Some(next) } => {
            assert!(next.command.contains("controller claim GAIN-2"));
            assert!(next.schema.required.contains(&"run".to_owned()));
            assert!(!next.schema.additional_properties);
            assert_eq!(next.template.values.get("task"), Some(&"GAIN-2".to_owned()));
        }
        _ => return Err("next returned no action".into()),
    }
    Ok(())
}

#[test]
fn refresh_accepts_delta() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let mut changed = task()?;
    changed.spec.priority = 2;
    let output = app.execute(
        Command::Refresh {
            changed: vec![changed],
            removed: Vec::new(),
        },
        false,
    )?;
    assert_eq!(output.events.len(), 1);
    Ok(())
}
