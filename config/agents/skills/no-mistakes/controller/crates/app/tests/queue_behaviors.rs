mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::*;
use domain::{
    command::{Command, NextAction},
    ids::TaskId,
    task::{Dependency, Phase},
};
use std::str::FromStr;

fn discover(
    app: &mut App<'_>,
    tasks: Vec<domain::command::DiscoveredTask>,
) -> Result<(), app::AgentError> {
    app.execute(
        Command::Discover {
            tasks,
            planning_order: None,
        },
        false,
    )?;
    Ok(())
}

fn next_action(app: &mut App<'_>) -> Result<NextAction, Box<dyn std::error::Error>> {
    match app.execute(Command::Next, false)?.result {
        ResultData::Next { next: Some(next) } => Ok(next.action),
        _ => Err("next returned no action".into()),
    }
}

#[test]
fn claim_respects_priority() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let mut low = task_named("GAIN-2")?;
    low.spec.priority = 2;
    let high = task_named("GAIN-3")?;
    discover(&mut app, vec![low, high])?;
    assert!(matches!(
        next_action(&mut app)?,
        NextAction::Claim { task } if task == TaskId::from_str("GAIN-3")?
    ));
    Ok(())
}

#[test]
fn dependency_requires_main_commit() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let mut blocked = task_named("GAIN-2")?;
    blocked.spec.dependencies.push(Dependency {
        task: TaskId::from_str("GAIN-9")?,
        code: true,
        verified: true,
        main_commit: None,
    });
    discover(&mut app, vec![blocked])?;
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state } => {
            assert!(matches!(
                state.tasks[&TaskId::from_str("GAIN-2")?].phase,
                Phase::Blocked(_)
            ));
        }
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn claim_excludes_other_runs() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with(Fake {
        reserve: false,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(None),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
    })?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    discover(&mut app, vec![task()?])?;
    let error = app.execute(
        Command::Claim {
            task: TaskId::from_str("GAIN-2")?,
        },
        false,
    );
    assert!(matches!(
        error,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state } => assert!(state.active.is_none()),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn discover_partitions_blocked_and_needs_input() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let mut blocked = task_named("GAIN-2")?;
    blocked.spec.dependencies.push(Dependency {
        task: TaskId::from_str("GAIN-9")?,
        code: true,
        verified: false,
        main_commit: None,
    });
    let mut question = task_named("GAIN-3")?;
    question.spec.ownership_clear = false;
    question.spec.ownership_evidence = "ambiguous branch owner".into();
    discover(&mut app, vec![blocked, question])?;
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state } => {
            assert!(matches!(
                state.tasks[&TaskId::from_str("GAIN-2")?].phase,
                Phase::Blocked(_)
            ));
            assert!(matches!(
                state.tasks[&TaskId::from_str("GAIN-3")?].phase,
                Phase::NeedsInput(_)
            ));
            assert_eq!(state.questions.len(), 1);
        }
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}
