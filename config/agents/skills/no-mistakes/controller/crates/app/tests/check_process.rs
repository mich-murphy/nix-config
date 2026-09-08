mod common;

use adapters::{SystemProcess, sqlite::Store};
use app::{App, ResultData};
use common::*;
use domain::{
    command::Command,
    event::{Event, Observation, OperationStatus},
    ids::TaskId,
};
use std::str::FromStr;

fn run(
    app: &mut App<'_>,
    directory: &std::path::Path,
    argv: Vec<String>,
) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::RunCheck {
            task: TaskId::from_str("GAIN-2").map_err(|error| app::AgentError {
                failed: "fixture".into(),
                phase: None,
                why: app::Rejection::Invalid(error.to_string()),
                next: None,
            })?,
            criteria: vec!["AC1".parse().map_err(|error: domain::ids::InvalidId| {
                app::AgentError {
                    failed: "fixture".into(),
                    phase: None,
                    why: app::Rejection::Invalid(error.to_string()),
                    next: None,
                }
            })?],
            argv,
            cwd: directory.to_owned(),
            timeout_seconds: 5,
            implementation: vec!["src/lib.rs:1".into()],
        },
        false,
    )
}

#[test]
fn check_records_exit_status() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let process = SystemProcess;
    let mut app = App::new(
        Store::open(directory.path())?,
        services_with_process(&fake, &process),
    );
    prepare(&mut app)?;
    let output = run(
        &mut app,
        directory.path(),
        vec!["sh".into(), "-c".into(), "exit 7".into()],
    )?;
    let artifact = output.events.iter().find_map(|record| match &record.event {
        Event::OperationSettled {
            status: OperationStatus::Failed,
            observation: Observation::Check { exit: 7, artifact },
            ..
        } => Some(artifact),
        _ => None,
    });
    assert!(artifact.is_some_and(|path| path.is_file()));
    let ResultData::State { state } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    assert!(state.operations[0].process.is_some());
    assert_eq!(state.operations[0].timeout_seconds, 5);
    Ok(())
}

#[test]
fn missing_check_binary_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let process = SystemProcess;
    let mut app = App::new(
        Store::open(directory.path())?,
        services_with_process(&fake, &process),
    );
    prepare(&mut app)?;
    let output = run(
        &mut app,
        directory.path(),
        vec!["no-such-no-mistakes-check-binary".into()],
    )?;
    assert!(output.events.iter().any(|record| matches!(
        record.event,
        Event::OperationSettled {
            status: OperationStatus::Failed,
            observation: Observation::Check { exit: 127, .. },
            ..
        }
    )));
    Ok(())
}
