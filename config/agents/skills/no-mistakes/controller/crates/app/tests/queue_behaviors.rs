mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::*;
use domain::{
    command::{Command, NextAction},
    delivery::{PrState, PullRequest},
    event::{Actor, Event},
    ids::{DeliveryId, PrNumber, SlotId, TaskId},
    task::{Dependency, HoldReason, Phase},
};
use std::str::FromStr;

/// An open (unmerged) PR for `delivery` on `task`, recorded directly
/// through `Store` rather than a real `publish` round trip.
fn record_open_pr(
    directory: &std::path::Path,
    task: &TaskId,
    number: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: task.clone(),
            delivery: DeliveryId(1),
            pr: PullRequest {
                number: PrNumber(number),
                state: PrState::Open,
                draft: false,
                head: sha('a')?,
                merge: None,
                checks: Vec::new(),
            },
        }],
    )?;
    Ok(())
}

/// A `CiPending` hold with a deadline still in the future, recorded
/// directly through `Store`: `Command::Hold` refuses this shape (a
/// coordinator cannot pre-empt a wait that has not expired), but this is
/// exactly the hold `poll-checks --wait` itself would leave behind. It
/// releases `task` as the run's active claim without making its open PR
/// disappear from the cap.
fn record_ci_wait(
    directory: &std::path::Path,
    task: &TaskId,
    number: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::Held {
            task: task.clone(),
            reason: HoldReason::CiPending {
                pr: PrNumber(number),
                head: sha('a')?,
                deadline: 1_000,
            },
            at: 10,
        }],
    )?;
    Ok(())
}

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
fn discovery_accepts_shared_dependency_paths() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let mut root = task_named("GAIN-2")?;
    root.spec.dependencies = vec![
        Dependency {
            task: TaskId::from_str("GAIN-3")?,
            code: true,
            main_commit: None,
        },
        Dependency {
            task: TaskId::from_str("GAIN-4")?,
            code: true,
            main_commit: None,
        },
    ];
    let mut left = task_named("GAIN-3")?;
    left.spec.dependencies.push(Dependency {
        task: TaskId::from_str("GAIN-5")?,
        code: true,
        main_commit: None,
    });
    let mut right = task_named("GAIN-4")?;
    right.spec.dependencies = left.spec.dependencies.clone();

    discover(&mut app, vec![root, left, right, task_named("GAIN-5")?])?;
    Ok(())
}

#[test]
fn discovery_rejects_dependency_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let mut first = task_named("GAIN-2")?;
    first.spec.dependencies.push(Dependency {
        task: TaskId::from_str("GAIN-3")?,
        code: true,
        main_commit: None,
    });
    let mut second = task_named("GAIN-3")?;
    second.spec.dependencies.push(Dependency {
        task: TaskId::from_str("GAIN-2")?,
        code: true,
        main_commit: None,
    });

    let error = discover(&mut app, vec![first, second]);
    assert!(matches!(
        error,
        Err(app::AgentError {
            why: Rejection::Invalid(message),
            ..
        }) if message.contains("dependency cycle")
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
        main_commit: None,
    });
    discover(&mut app, vec![blocked])?;
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state, .. } => {
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
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
        last_prompt: std::cell::RefCell::new(None),
        create_fails_once: std::cell::Cell::new(false),
        spawned: std::cell::RefCell::new(Vec::new()),
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
        ResultData::State { state, .. } => assert!(state.active.is_none()),
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
        main_commit: None,
    });
    let mut question = task_named("GAIN-3")?;
    question.spec.ownership_clear = false;
    question.spec.ownership_evidence = "ambiguous branch owner".into();
    discover(&mut app, vec![blocked, question])?;
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state, .. } => {
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

/// Claims `task`, binds it to `slot`, then records an open PR and a
/// not-yet-expired `CiPending` wait for it: one task's share of the
/// `max_open_prs` cap `pr_cap_blocks_claim_not_resume` fills twice, split
/// out so that repetition is not scored against the scenario itself.
fn claim_bind_and_wait(
    directory: &std::path::Path,
    fake: &Fake,
    task: &TaskId,
    slot: &str,
    number: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(Command::Claim { task: task.clone() }, false)?;
    app.execute(
        Command::BindSlot {
            task: task.clone(),
            slot: SlotId::from_str(slot)?,
            branch: format!("agent/{slot}/{task}"),
            authority: None,
        },
        false,
    )?;
    drop(app);
    record_open_pr(directory, task, number)?;
    record_ci_wait(directory, task, number)?;
    Ok(())
}

/// `state.active`, read back through `Command::Status`.
fn active_task(app: &mut App<'_>) -> Result<Option<TaskId>, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    Ok(state.active)
}

/// Asserts that claiming `task` is rejected on conflict and leaves it
/// `Queued`, the cap's effect on a third, otherwise eligible task.
fn assert_claim_capped(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    let claim = app.execute(Command::Claim { task: task.clone() }, false);
    assert!(matches!(
        claim,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    assert!(matches!(task_state(app, task)?.phase, Phase::Queued));
    Ok(())
}

/// Discovers GAIN-2, GAIN-3 and GAIN-4 together, the three-task queue
/// `pr_cap_blocks_claim_not_resume` fills to the open-PR cap.
fn discover_three(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    discover(
        app,
        vec![task()?, task_named("GAIN-3")?, task_named("GAIN-4")?],
    )?;
    Ok(())
}

/// Asserts `next` reports `RecoverUnfinished` (the cap is full), then that
/// claiming `task` is rejected and leaves it untouched.
fn assert_capped_at_recovery(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(matches!(
        next_action(app)?,
        NextAction::RecoverUnfinished { .. }
    ));
    assert_claim_capped(app, task)
}

/// Resumes `task` and confirms it becomes the run's active claim again,
/// unaffected by the open-PR cap that blocks a fresh `Claim`.
fn resume_and_confirm_active(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(Command::Resume { task: task.clone() }, false)?;
    assert_eq!(active_task(app)?, Some(task.clone()));
    Ok(())
}

/// Two tasks each hold an open PR (a `CiPending` wait, not yet expired) at
/// `max_open_prs` (2 by config). `next` then reports `RecoverUnfinished`
/// rather than a fresh `Claim`, so `Command::Claim` on a third, otherwise
/// eligible task is rejected and leaves it untouched; `Command::Resume` on
/// an already-held task is unaffected by the cap.
#[test]
fn pr_cap_blocks_claim_not_resume() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let gain2 = common::literals::task_id("GAIN-2");
    let gain3 = common::literals::task_id("GAIN-3");
    let gain4 = common::literals::task_id("GAIN-4");
    discover_three(&mut app)?;
    drop(app);
    claim_bind_and_wait(directory.path(), &fake, &gain2, "worker1", 1)?;
    claim_bind_and_wait(directory.path(), &fake, &gain3, "worker2", 2)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    assert_capped_at_recovery(&mut app, &gain4)?;
    resume_and_confirm_active(&mut app, &gain2)?;
    Ok(())
}
