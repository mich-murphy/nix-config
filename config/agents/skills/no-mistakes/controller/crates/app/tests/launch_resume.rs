mod common;

use adapters::sqlite::Store;
use app::App;
use common::golden::{plan_and_implement, prompt_file};
use common::*;
use domain::{
    command::{AgentRole, Command, RecoveryTarget},
    event::Launch,
    ids::{DeliveryId, LaunchId, TaskId},
    ports::ProcessIdentity,
};

/// The controller requires every implementer launch to be checkpointed
/// before the next one, a killed launch included. These tests care about
/// the session a launch carries, not the turn's narrative.
fn checkpoint(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced: true,
            observation: "turn observed".into(),
            next: "continue".into(),
            outside_paths: Vec::new(),
            scope_reason: None,
        },
        false,
    )?;
    Ok(())
}

/// One implementer turn: the launch plus its checkpoint.
fn turn(
    app: &mut App<'_>,
    task: &TaskId,
    prompt: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt,
            fallback: None,
        },
        false,
    )?;
    checkpoint(app, task)
}

/// A launch record the coordinator owns a live process for, appended after
/// whatever launches a test has already made, so `RecoverOperation` accepts
/// it and records the `Failed` outcome a host kill leaves behind.
fn kill_a_launch(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::open(directory)?;
    let next = LaunchId(store.state()?.launches.len() as u64 + 1);
    store.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::LaunchStarted {
            launch: Launch {
                id: next,
                task: task.clone(),
                delivery: DeliveryId(1),
                role: AgentRole::Implementer,
                prompt: directory.join("killed.md"),
                outcome: None,
                usage: None,
                checkpointed: false,
                process: Some(ProcessIdentity {
                    pid: 1,
                    start_ticks: 2,
                    group: 1,
                }),
            },
        }],
    )?;
    app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch { launch: next },
            terminate: true,
        },
        false,
    )?;
    checkpoint(app, task)
}

#[test]
fn completed_launch_is_resumed_by_session() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;
    assert_eq!(*fake.last_session.borrow(), Some(None));
    turn(
        &mut app,
        &task,
        prompt_file(directory.path(), "resumed.md")?,
    )?;
    assert_eq!(
        *fake.last_session.borrow(),
        Some(Some("implementer-session".to_owned()))
    );
    Ok(())
}

/// A launch the host killed records an empty session, and an empty
/// `--resume` is rejected by every harness. The next launch of that role
/// in the same delivery must start fresh rather than inherit the empty
/// string, or one kill would strand the delivery.
#[test]
fn killed_launch_does_not_poison_resume() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;
    kill_a_launch(&mut app, directory.path(), &task)?;
    turn(&mut app, &task, prompt_file(directory.path(), "after.md")?)?;
    assert_eq!(
        *fake.last_session.borrow(),
        Some(None),
        "a killed launch must not leave an empty session for the next one to resume"
    );
    Ok(())
}
