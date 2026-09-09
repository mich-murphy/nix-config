mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::*;
use domain::{
    command::{Command, Lesson},
    event::{Actor, Event},
    ids::TaskId,
    task::{Plan, Receipt},
};
use std::{collections::BTreeMap, str::FromStr};

fn lesson(text: &str, accepted: bool) -> Lesson {
    Lesson {
        text: text.into(),
        families: vec!["controller".into()],
        accepted,
    }
}

#[test]
fn lesson_acceptance_requires_verified() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let result = app.execute(
        Command::LessonRecord {
            task: TaskId::from_str("GAIN-2")?,
            lesson: lesson("use event outcomes", true),
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

#[test]
fn lesson_registers_once() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let command = || Command::LessonRecord {
        task: TaskId::from_str("GAIN-2").unwrap_or_else(|error| panic!("fixture: {error}")),
        lesson: lesson("use event outcomes", false),
    };
    app.execute(command(), false)?;
    assert!(matches!(
        app.execute(command(), false),
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    Ok(())
}

#[test]
fn plan_selects_accepted_lessons() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    record_verified_lesson(directory.path(), &fake)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare_second(&mut app)?;
    plan_second(&mut app)?;
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    let value = &state.tasks[&TaskId::from_str("GAIN-3")?];
    if !matches!(value.phase, domain::task::Phase::Planned) {
        return Err("task was not planned".into());
    }
    let work = value.current_work().ok_or("planned task has no work")?;
    assert_eq!(work.feedback, vec![lesson("use event outcomes", true)]);
    Ok(())
}

fn task_feedback(app: &mut App<'_>) -> Result<Vec<Lesson>, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    Ok(state.tasks[&TaskId::from_str("GAIN-3")?].deliveries[0]
        .work
        .as_ref()
        .ok_or("work missing")?
        .feedback
        .clone())
}

#[test]
fn feedback_pins_to_main() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    record_verified_lesson(directory.path(), &fake)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare_second(&mut app)?;
    plan_second(&mut app)?;
    app.execute(
        Command::LessonRecord {
            task: TaskId::from_str("GAIN-2")?,
            lesson: lesson("later guidance", true),
        },
        false,
    )?;
    assert_eq!(
        task_feedback(&mut app)?,
        vec![lesson("use event outcomes", true)]
    );
    Ok(())
}

fn base_fake() -> Fake {
    Fake {
        reserve: true,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(None),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
    }
}

#[test]
fn plan_loads_committed_lessons() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let feedback_path = directory.path().join("feedback.json");
    std::fs::write(
        &feedback_path,
        serde_json::to_string(&vec![lesson("pinned run guidance", true)])?,
    )?;
    let (directory, fake) = initialized_with_feedback(base_fake(), feedback_path)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let value = task_state(&mut app, &TaskId::from_str("GAIN-2")?)?;
    let work = value.current_work().ok_or("planned task has no work")?;
    assert_eq!(work.feedback, vec![lesson("pinned run guidance", true)]);
    Ok(())
}

#[test]
fn plan_rejects_malformed_feedback_file() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let feedback_path = directory.path().join("feedback.json");
    std::fs::write(&feedback_path, "not json")?;
    let (directory, fake) = initialized_with_feedback(base_fake(), feedback_path)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task_id = TaskId::from_str("GAIN-2")?;
    discover_claim(&mut app, &task_id)?;
    sync_progress(&mut app, &task_id)?;
    let plan = brief_and_bind(&mut app, &task_id)?;
    let result = app.execute(
        Command::Plan {
            task: task_id,
            plan,
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

fn record_verified_lesson(
    directory: &std::path::Path,
    fake: &Fake,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?, task_named("GAIN-3")?],
            planning_order: None,
        },
        false,
    )?;
    drop(app);
    let mut store = Store::open(directory)?;
    store.commit(
        Actor::Coordinator,
        10,
        &[Event::Verified {
            task: TaskId::from_str("GAIN-2")?,
            receipt: Receipt::Delivery { commit: sha('a')? },
        }],
    )?;
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::LessonRecord {
            task: TaskId::from_str("GAIN-2")?,
            lesson: lesson("use event outcomes", true),
        },
        false,
    )?;
    Ok(())
}

fn prepare_second(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let task = TaskId::from_str("GAIN-3")?;
    app.execute(Command::Claim { task: task.clone() }, false)?;
    sync_progress(app, &task)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![domain::task::Criterion {
                id: "AC1".parse()?,
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task,
            slot: "worker2".parse()?,
            branch: "agent/worker2/GAIN-3".into(),
            authority: None,
        },
        false,
    )?;
    Ok(())
}

fn plan_second(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Plan {
            task: TaskId::from_str("GAIN-3")?,
            plan: Plan {
                deliverable: "observable result".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([("AC1".parse()?, "b".repeat(64).parse()?)]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    Ok(())
}
