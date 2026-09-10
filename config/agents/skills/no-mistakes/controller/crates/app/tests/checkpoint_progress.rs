mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::literals::{criterion_id, digest, task_id};
use common::*;
use domain::{
    command::{AgentRole, Command},
    ids::{Digest, TaskId},
    task::Plan,
};
use std::{collections::BTreeMap, fs, str::FromStr};

fn prompt(directory: &std::path::Path) -> Result<std::path::PathBuf, std::io::Error> {
    let path = directory.join("prompt.md");
    fs::write(&path, "bounded task")?;
    Ok(path)
}

fn run_implementer(
    app: &mut App<'_>,
    task: &TaskId,
    prompt: &std::path::Path,
) -> Result<(), app::AgentError> {
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt.to_owned(),
            fallback: None,
        },
        false,
    )?;
    Ok(())
}

fn checkpoint(app: &mut App<'_>, task: &TaskId, advanced: bool) -> Result<(), app::AgentError> {
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced,
            observation: "new measured observation".into(),
            next: "bounded correction".into(),
            outside_paths: Vec::new(),
            scope_reason: Some("same deliverable".into()),
        },
        false,
    )?;
    Ok(())
}

fn turn(
    app: &mut App<'_>,
    task: &TaskId,
    prompt: &std::path::Path,
    advanced: bool,
) -> Result<(), app::AgentError> {
    run_implementer(app, task, prompt)?;
    checkpoint(app, task, advanced)
}

fn stalled(app: &mut App<'_>, task: &TaskId) -> Result<(u32, bool), Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    let value = &state.tasks[task];
    Ok((value.budgets.stalled_checkpoints, value.hold.is_some()))
}

fn replan(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "observable result".into(),
                components: vec!["src".into(), "tests".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([("AC1".parse()?, Digest::from_str(&"b".repeat(64))?)]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    Ok(())
}

#[test]
fn turn_requires_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let file = prompt(directory.path())?;
    run_implementer(&mut app, &task, &file)?;
    let second = app.execute(
        Command::RunAgent {
            task,
            role: AgentRole::Implementer,
            prompt: file,
            fallback: None,
        },
        true,
    );
    assert!(matches!(
        second,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    Ok(())
}

#[test]
fn stall_resets_on_new_artifact() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let file = prompt(directory.path())?;
    turn(&mut app, &task, &file, false)?;
    turn(&mut app, &task, &file, true)?;
    assert_eq!(stalled(&mut app, &task)?.0, 0);
    Ok(())
}

#[test]
fn replan_keeps_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let result = app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "changed plan".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([("AC1".parse()?, Digest::from_str(&"c".repeat(64))?)]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Evidence(_),
            ..
        })
    ));
    Ok(())
}

/// A re-plan that adds a baseline for a criterion the task never had one
/// for (as `narrow-acceptance` would introduce) succeeds and the task keeps
/// both the original and the new baseline; only rewriting an existing one
/// is a rejection (`replan_keeps_baseline`, above).
#[test]
fn replan_adds_new_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    let ac1 = criterion_id("AC1");
    let stage1 = criterion_id("STAGE1");
    let ac1_baseline = digest('b');
    let stage1_baseline = digest('d');
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "observable result".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([
                    (ac1.clone(), ac1_baseline.clone()),
                    (stage1.clone(), stage1_baseline.clone()),
                ]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.baselines.get(&ac1), Some(&ac1_baseline));
    assert_eq!(state.baselines.get(&stage1), Some(&stage1_baseline));
    Ok(())
}

#[test]
fn stall_limit_survives_replan() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let file = prompt(directory.path())?;
    turn(&mut app, &task, &file, false)?;
    replan(&mut app, &task)?;
    turn(&mut app, &task, &file, false)?;
    assert_eq!(stalled(&mut app, &task)?, (2, false));
    turn(&mut app, &task, &file, false)?;
    assert_eq!(stalled(&mut app, &task)?, (3, true));
    Ok(())
}

#[test]
fn brief_change_keeps_counters() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    run_implementer(&mut app, &task, &prompt(directory.path())?)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![domain::task::Criterion {
                id: "AC2".parse()?,
                text: "revised observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    assert_eq!(state.tasks[&task].budgets.implementation_turns, 1);
    let value = &state.tasks[&task];
    assert!(matches!(value.phase, domain::task::Phase::Planned));
    let work = value
        .current_work()
        .ok_or("planned task has no current work")?;
    assert!(
        work.plan.is_none() && work.snapshot.is_none(),
        "delivery retained stale plan: {work:?}"
    );
    Ok(())
}

#[test]
fn checkpoint_requires_scope_reason() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    run_implementer(&mut app, &task, &prompt(directory.path())?)?;
    let result = app.execute(
        Command::Checkpoint {
            task,
            advanced: true,
            observation: "new observation".into(),
            next: "continue".into(),
            outside_paths: vec!["docs/guide.md".into()],
            scope_reason: None,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Invalid(_),
            ..
        })
    ));
    Ok(())
}

#[test]
fn checkpoint_allows_integration() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    app.execute(Command::Snapshot { task: task.clone() }, false)?;
    checkpoint(&mut app, &task, true)?;
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    // An integration checkpoint marks no launch: there is none to mark.
    assert!(state.launches.is_empty());
    Ok(())
}
