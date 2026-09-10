mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::*;
use domain::{
    command::{Command, NextAction},
    ids::TaskId,
    task::Phase,
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

/// One task with unclear ownership no longer stops the rest of the
/// queue: `next` claims the clear task first and puts the question to the
/// coordinator only once nothing else can proceed. Before this rule a
/// single unassigned issue at discovery returned `AnswerQuestions` for
/// the whole run.
#[test]
fn question_holds_only_its_task() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let clear = TaskId::from_str("GAIN-3")?;
    discover(
        &mut app,
        vec![questioned_task("GAIN-2")?, task_named("GAIN-3")?],
    )?;
    assert!(matches!(
        next_action(&mut app)?,
        NextAction::Claim { task } if task == clear
    ));
    app.execute(
        Command::Claim {
            task: clear.clone(),
        },
        false,
    )?;
    assert!(matches!(
        task_state(&mut app, &clear)?.phase,
        Phase::Claimed
    ));
    Ok(())
}

/// A discovered task whose Jira ownership the coordinator could not
/// settle (here: no active assignee).
fn questioned_task(id: &str) -> Result<domain::command::DiscoveredTask, domain::ids::InvalidId> {
    let mut task = task_named(id)?;
    task.spec.ownership_clear = false;
    task.spec.ownership_evidence = "no active assignee".into();
    Ok(task)
}

/// With no claimable task left, the open question is what `next`
/// reports, ahead of an idle `Report`.
#[test]
fn question_surfaces_when_queue_is_idle() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    discover(&mut app, vec![questioned_task("GAIN-2")?])?;
    assert!(matches!(
        next_action(&mut app)?,
        NextAction::AnswerQuestions { questions } if questions.len() == 1
    ));
    Ok(())
}
