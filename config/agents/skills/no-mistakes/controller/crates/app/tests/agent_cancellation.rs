mod common;

use adapters::sqlite::Store;
use app::App;
use common::*;
use domain::{
    command::{AgentRole, Command, RecoveryTarget},
    ids::TaskId,
    ports::{
        Capabilities, Harness, Isolation, LaunchRequest, LaunchResult, PortError, ProcessIdentity,
        StructuredOutput,
    },
    risk::HarnessKind,
};
use std::str::FromStr;

fn prompt(directory: &std::path::Path) -> Result<std::path::PathBuf, std::io::Error> {
    let path = directory.join("prompt.md");
    std::fs::write(&path, "bounded task")?;
    Ok(path)
}

/// A harness that reaches the gate (so the launch is charged) and then fails,
/// simulating a timeout or cancellation after the process was released.
struct Canceling;

impl Harness for Canceling {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Pi
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::SelfValidated,
            isolation: Isolation::ToolRestriction,
        }
    }
    fn run(
        &self,
        _request: &LaunchRequest,
        started: &mut dyn FnMut(ProcessIdentity) -> Result<(), PortError>,
    ) -> Result<LaunchResult, PortError> {
        started(ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        })?;
        Err(PortError("process timed out after 5s".into()))
    }
}

fn prepare_in(directory: &std::path::Path, fake: &Fake) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    prepare(&mut app)
}

fn canceling_app<'a>(
    directory: &std::path::Path,
    fake: &'a Fake,
    canceling: &'a Canceling,
) -> Result<App<'a>, Box<dyn std::error::Error>> {
    Ok(App::new(
        Store::open(directory)?,
        app::Services {
            vcs: fake,
            github: fake,
            harness: canceling,
            process: fake,
            clock: fake,
        },
    ))
}

#[test]
fn cancelled_launch_is_charged() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    prepare_in(directory.path(), &fake)?;
    let task = TaskId::from_str("GAIN-2")?;
    let canceling = Canceling;
    let mut app = canceling_app(directory.path(), &fake, &canceling)?;
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        false,
    );
    assert!(result.is_err());
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.budgets.implementation_turns, 1);
    let launch = launch_for(&mut app, &task)?;
    assert!(launch.session.is_none());
    app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch { launch: launch.id },
            terminate: true,
        },
        false,
    )?;
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.budgets.implementation_turns, 1);
    Ok(())
}

fn launch_for(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<domain::event::Launch, Box<dyn std::error::Error>> {
    let app::ResultData::State { state } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .launches
        .iter()
        .find(|launch| launch.task == *task && launch.role == AgentRole::Implementer)
        .cloned()
        .ok_or_else(|| "launch missing".into())
}
