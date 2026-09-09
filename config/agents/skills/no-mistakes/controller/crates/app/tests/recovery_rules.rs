mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::*;
use domain::{
    command::{AgentRole, Command, RecoveryTarget, Transition},
    delivery::Outcome,
    event::{GitHubAction, Launch, Operation, OperationStatus},
    ids::{
        DeliveryId, IssueKey, JiraStatus, LaunchId, OperationId, PrNumber, TaskId, TransitionId,
    },
    ports::ProcessIdentity,
    task::HoldReason,
};
use std::str::FromStr;

#[test]
fn ci_hold_requires_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let result = app.execute(
        Command::Hold {
            task: TaskId::from_str("GAIN-2")?,
            reason: HoldReason::CiPending {
                pr: PrNumber(1),
                head: sha('a')?,
                deadline: 11,
            },
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
fn ci_deadline_survives_hold() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let head = sha('a')?;
    app.execute(
        Command::Hold {
            task: task.clone(),
            reason: HoldReason::CiPending {
                pr: PrNumber(1),
                head: head.clone(),
                deadline: 10,
            },
        },
        false,
    )?;
    app.execute(Command::Resume { task: task.clone() }, false)?;
    let app::ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    let delivery = state.tasks[&task]
        .deliveries
        .last()
        .ok_or("delivery missing")?;
    assert_eq!(delivery.check_deadlines.get(&head), Some(&10));
    Ok(())
}

fn pending_launch(
    directory: &std::path::Path,
    process: Option<ProcessIdentity>,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::LaunchStarted {
            launch: Launch {
                id: LaunchId(1),
                task: TaskId::from_str("GAIN-2")?,
                delivery: DeliveryId(1),
                role: AgentRole::Implementer,
                prompt: directory.join("prompt.md"),
                outcome: None,
                usage: None,
                checkpointed: false,
                process,
            },
        }],
    )?;
    Ok(())
}

#[test]
fn recovery_refuses_unowned() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    pending_launch(directory.path(), None)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch {
                launch: LaunchId(1),
            },
            terminate: true,
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
fn recovery_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    pending_launch(
        directory.path(),
        Some(ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        }),
    )?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let command = || Command::RecoverOperation {
        target: RecoveryTarget::Launch {
            launch: LaunchId(1),
        },
        terminate: true,
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

fn implementation_spend(app: &mut App<'_>) -> Result<u32, Box<dyn std::error::Error>> {
    let app::ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    Ok(state.tasks[&TaskId::from_str("GAIN-2")?]
        .budgets
        .implementation_turns)
}

#[test]
fn recovery_preserves_spend() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    drop(app);
    pending_launch(
        directory.path(),
        Some(ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        }),
    )?;
    Store::open(directory.path())?.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::BudgetSpent {
            task: TaskId::from_str("GAIN-2")?,
            budget: domain::budget::BudgetKind::Implementation,
            counted: true,
        }],
    )?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Launch {
                launch: LaunchId(1),
            },
            terminate: true,
        },
        false,
    )?;
    assert_eq!(implementation_spend(&mut app)?, 1);
    Ok(())
}

#[test]
fn closed_operations_not_dispatchable() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    drop(app);
    let task = TaskId::from_str("GAIN-2")?;
    let operation = Operation {
        id: OperationId(1),
        task: task.clone(),
        delivery: DeliveryId(1),
        action: GitHubAction::Check {
            argv: vec!["true".into()],
            cwd: directory.path().to_owned(),
            timeout_seconds: 5,
        },
        status: OperationStatus::Running,
        process: Some(ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        }),
        timeout_seconds: 5,
    };
    let mut store = Store::open(directory.path())?;
    store.commit(
        domain::event::Actor::Coordinator,
        10,
        &[
            domain::event::Event::OperationStarted { operation },
            domain::event::Event::DeliveryClosed {
                task: task.clone(),
                delivery: DeliveryId(1),
                outcome: Outcome::Merged {
                    commit: sha('a')?,
                    at: 10,
                },
            },
        ],
    )?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = app.execute(
        Command::RecoverOperation {
            target: RecoveryTarget::Operation {
                operation: OperationId(1),
            },
            terminate: true,
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

fn mark_verified(
    directory: &std::path::Path,
    fake: &Fake,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    drop(app);
    Store::open(directory)?.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::Verified {
            task: TaskId::from_str("GAIN-2")?,
            receipt: domain::task::Receipt::Delivery { commit: sha('a')? },
        }],
    )?;
    Ok(())
}

#[test]
fn subtask_done_requires_record() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    mark_verified(directory.path(), &fake)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = app.execute(
        Command::SetStatus {
            task: TaskId::from_str("GAIN-2")?,
            issue: IssueKey::from_str("GAIN-20")?,
            current: JiraStatus::from_str("progress")?,
            target: JiraStatus::from_str("done")?,
            transitions: vec![Transition {
                id: TransitionId::from_str("41")?,
                to: JiraStatus::from_str("done")?,
            }],
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
