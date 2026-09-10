mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for};
use common::literals::digest;
use common::*;
use domain::{
    budget::BudgetKind,
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
            tracer: fake,
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
    assert!(launch.outcome.is_none());
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

/// A launch that reaches the gate (so it is charged) and then fails is
/// unsettled: `next` surfaces it as `MonitorLaunch` rather than skipping
/// past it, and `recover-operation` settles it `Failed` — a stopped, not
/// terminated, process, since the harness itself already returned before
/// any termination was requested. The budget charged when the launch
/// started is unchanged by settling it: recovery is neither a second
/// charge nor a refund.
/// Runs one implementer turn against `canceling`, asserting it fails at
/// the gate, and returns the unsettled launch it leaves behind.
fn cancel_at_gate(
    app: &mut App<'_>,
    task: &TaskId,
    directory: &std::path::Path,
) -> Result<domain::event::Launch, Box<dyn std::error::Error>> {
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt(directory)?,
            fallback: None,
        },
        false,
    );
    assert!(result.is_err());
    unsettled_launch(app, task)
}

/// Asserts `launch` settled `Failed` (a stopped, not terminated, process:
/// the harness itself already returned before any termination was
/// requested) and that the budget `launch` charged at start is
/// unchanged: settling it is neither a second charge nor a refund.
fn assert_settled_failed_without_recharge(
    app: &mut App<'_>,
    task: &TaskId,
    charged: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let settled = launch_for(app, task)?;
    assert!(matches!(
        settled.outcome,
        Some(domain::event::LaunchOutcome::Failed { .. })
    ));
    assert_eq!(task_state(app, task)?.budgets.implementation_turns, charged);
    Ok(())
}

#[test]
fn unsettled_launch_is_recoverable() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    prepare_in(directory.path(), &fake)?;
    let task = TaskId::from_str("GAIN-2")?;
    let canceling = Canceling;
    let mut app = canceling_app(directory.path(), &fake, &canceling)?;
    let launch = cancel_at_gate(&mut app, &task, directory.path())?;
    let charged = task_state(&mut app, &task)?.budgets.implementation_turns;

    expect_next(
        &mut app,
        |action| matches!(action, domain::command::NextAction::MonitorLaunch { launch: value } if *value == launch.id),
    )?;

    app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch { launch: launch.id },
            terminate: false,
        },
        false,
    )?;
    assert_settled_failed_without_recharge(&mut app, &task, charged)?;
    Ok(())
}

fn launch_for(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<domain::event::Launch, Box<dyn std::error::Error>> {
    let app::ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .launches
        .iter()
        .find(|launch| launch.task == *task && launch.role == AgentRole::Implementer)
        .cloned()
        .ok_or_else(|| "launch missing".into())
}

/// The most recent unsettled implementer launch for `task`, the one a
/// just-failed `run-agent` left behind for `recover-operation`.
fn unsettled_launch(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<domain::event::Launch, Box<dyn std::error::Error>> {
    let app::ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .launches
        .iter()
        .rev()
        .find(|launch| {
            launch.task == *task
                && launch.role == AgentRole::Implementer
                && launch.outcome.is_none()
        })
        .cloned()
        .ok_or_else(|| "unsettled launch missing".into())
}

/// One implementer turn that reaches the gate, is charged, then fails:
/// recovered (terminated) and checkpointed (`advanced: true`, so it never
/// contributes to the stalled-checkpoint count) so the next launch is
/// free to start. Shared by `turn_limit_counts_cancelled` and
/// `exhausted_turns_allow_review` to drive the implementation budget to
/// its Trivial-tier cap of four without ever completing real work.
fn cancel_one_turn(
    app: &mut App<'_>,
    task: &TaskId,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt(directory)?,
            fallback: None,
        },
        false,
    );
    assert!(result.is_err());
    let launch = unsettled_launch(app, task)?;
    app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch { launch: launch.id },
            terminate: true,
        },
        false,
    )?;
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced: true,
            observation: "turn cancelled".into(),
            next: "retry".into(),
            outside_paths: Vec::new(),
            scope_reason: None,
        },
        false,
    )?;
    Ok(())
}

/// Driving a Trivial-tier task (limit 4) to implementation exhaustion
/// through four cancelled turns still counts every one of them: the fifth
/// launch is rejected on budget, not merely "still running".
#[test]
fn turn_limit_counts_cancelled() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    prepare_in(directory.path(), &fake)?;
    let task = TaskId::from_str("GAIN-2")?;
    let canceling = Canceling;
    let mut app = canceling_app(directory.path(), &fake, &canceling)?;
    cancel_turns(&mut app, &task, directory.path(), 4)?;
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.budgets.implementation_turns, 4);

    let fifth = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        false,
    );
    assert!(matches!(
        fifth,
        Err(app::AgentError {
            why: Rejection::Budget(BudgetKind::Implementation),
            ..
        })
    ));
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.budgets.implementation_turns, 4);
    Ok(())
}

/// `plan_and_implement` plus one recorded AC1 proof entry, the shared
/// starting point `exhausted_turns_allow_review` needs before it cancels
/// out the implementation budget. Split out so its own three fallible
/// steps are scored apart from the scenario that drives the turns.
fn plan_implement_and_record_proof(
    app: &mut App<'_>,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = plan_and_implement(app, directory)?;
    let (delivery, snapshot) = current_delivery(app, &task)?;
    record_proof_for(
        app,
        &task,
        delivery,
        "AC1",
        digest('b'),
        snapshot,
        directory.join("proof.txt"),
    )?;
    Ok(task)
}

/// Cancels `count` implementer turns in a row, the loop
/// `exhausted_turns_allow_review` and `turn_limit_counts_cancelled` both
/// drive to exhaust the implementation budget, split out so the loop's own
/// branch is not scored against the scenario around it.
fn cancel_turns(
    app: &mut App<'_>,
    task: &TaskId,
    directory: &std::path::Path,
    count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..count {
        cancel_one_turn(app, task, directory)?;
    }
    Ok(())
}

/// Implementation exhausted by four cancelled turns still permits a
/// reviewer launch once real proof satisfies readiness: the two budgets
/// are independent.
#[test]
fn exhausted_turns_allow_review() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_record_proof(&mut app, directory.path())?;
    drop(app);

    let canceling = Canceling;
    let mut app = canceling_app(directory.path(), &fake, &canceling)?;
    cancel_turns(&mut app, &task, directory.path(), 3)?;
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.budgets.implementation_turns, 4);
    drop(app);

    arrange_passing_review(&fake)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let review = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        false,
    );
    assert!(review.is_ok());
    Ok(())
}
