mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::literals::{discovered_task, task_id};
use common::{initialized, services};
use domain::{
    command::{Command, NextAction},
    ids::TaskId,
    task::{HoldReason, Phase},
};

fn setup<'a>(
    directory: &std::path::Path,
    fake: &'a common::Fake,
) -> Result<App<'a>, Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::Discover {
            tasks: vec![discovered_task("GAIN-2"), discovered_task("GAIN-3")],
            planning_order: None,
        },
        false,
    )?;
    Ok(app)
}

fn remove(app: &mut App<'_>, task: TaskId) -> Result<(), app::AgentError> {
    app.execute(
        Command::Refresh {
            changed: Vec::new(),
            removed: vec![task],
        },
        false,
    )?;
    Ok(())
}

fn assert_next_task(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let output = app.execute(Command::Next, false)?;
    assert!(matches!(
        output.result,
        ResultData::Next { next: Some(next) }
            if matches!(next.action, NextAction::Claim { ref task } if *task == task_id("GAIN-3"))
    ));
    Ok(())
}

#[test]
fn removing_held_task_releases_queue() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = setup(directory.path(), &fake)?;
    let removed = task_id("GAIN-2");
    app.execute(
        Command::Claim {
            task: removed.clone(),
        },
        false,
    )?;
    app.execute(
        Command::Hold {
            task: removed.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "task removed from the frozen queue".into(),
                remaining: Vec::new(),
            },
        },
        false,
    )?;
    remove(&mut app, removed.clone())?;

    assert_next_task(&mut app)?;
    let state = app.execute(Command::Status, false)?;
    let ResultData::State { state, .. } = state.result else {
        return Err("status returned wrong result".into());
    };
    assert!(matches!(state.tasks[&removed].phase, Phase::Excluded(_)));
    assert!(state.tasks[&removed].hold.is_none());
    assert!(state.active.is_none());
    Ok(())
}

#[test]
fn removing_active_task_releases_queue() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = setup(directory.path(), &fake)?;
    let removed = task_id("GAIN-2");
    app.execute(
        Command::Claim {
            task: removed.clone(),
        },
        false,
    )?;
    remove(&mut app, removed)?;

    assert_next_task(&mut app)
}
