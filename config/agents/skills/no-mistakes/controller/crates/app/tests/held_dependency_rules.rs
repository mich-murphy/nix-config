mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::*;
use domain::{
    command::{Command, NextAction},
    ids::TaskId,
    task::{Dependency, HoldReason},
};

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

fn active_task(app: &mut App<'_>) -> Result<Option<TaskId>, Box<dyn std::error::Error>> {
    match app.execute(Command::Status, false)?.result {
        ResultData::State { state, .. } => Ok(state.active),
        _ => Err("status returned wrong result".into()),
    }
}

fn hold_root_with_dependency_chain(
    app: &mut App<'_>,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let root = task_named("GAIN-2")?;
    let middle = task_named("GAIN-3")?;
    let leaf = task_named("GAIN-4")?;
    discover(app, vec![root.clone(), middle.clone(), leaf.clone()])?;
    app.execute(
        Command::Claim {
            task: root.id.clone(),
        },
        false,
    )?;
    app.execute(
        Command::Hold {
            task: root.id.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "a repository prerequisite was discovered".into(),
                remaining: vec![common::literals::criterion_id("AC1")],
            },
        },
        false,
    )?;

    let mut changed_root = root;
    changed_root.spec.dependencies.push(Dependency {
        task: middle.id.clone(),
        code: true,
        main_commit: None,
    });
    let mut changed_middle = middle;
    changed_middle.spec.dependencies.push(Dependency {
        task: leaf.id.clone(),
        code: true,
        main_commit: None,
    });
    app.execute(
        Command::Refresh {
            changed: vec![changed_root, changed_middle],
            removed: Vec::new(),
        },
        false,
    )?;
    Ok(leaf.id)
}

#[test]
fn held_task_schedules_its_new_dependency_chain() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let leaf = hold_root_with_dependency_chain(&mut app)?;
    assert!(matches!(
        next_action(&mut app)?,
        NextAction::Claim { task } if task == leaf
    ));
    app.execute(Command::Claim { task: leaf.clone() }, false)?;
    assert_eq!(active_task(&mut app)?, Some(leaf));
    Ok(())
}
