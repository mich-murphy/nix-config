mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::*;
use domain::{
    acceptance::{Measurement, ProofEntry, ProofKind, ProofStatus},
    command::{AgentRole, Command},
    event::{Actor, Event, Launch, LaunchOutcome},
    ids::{DeliveryId, Digest, LaunchId, TaskId},
};
use sha2::{Digest as _, Sha256};
use std::{fs, str::FromStr};

fn prompt(directory: &std::path::Path) -> Result<std::path::PathBuf, std::io::Error> {
    let path = directory.join("prompt.md");
    fs::write(&path, "bounded task")?;
    Ok(path)
}

fn proof_ready(
    app: &mut App<'_>,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let task = TaskId::from_str("GAIN-2")?;
    app.execute(Command::Snapshot { task: task.clone() }, false)?;
    let (delivery, snapshot) = current_delivery(app, &task)?;
    let artifact = directory.join("proof.txt");
    fs::write(&artifact, "measured result")?;
    let digest = Digest::from_str(&format!("{:x}", Sha256::digest(fs::read(&artifact)?)))?;
    app.execute(
        Command::RecordProof {
            task,
            delivery,
            entries: vec![proof_entry(artifact, digest, snapshot)?],
        },
        false,
    )?;
    Ok(())
}

fn current_delivery(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(domain::ids::DeliveryId, domain::acceptance::Snapshot), Box<dyn std::error::Error>> {
    let ResultData::State { state } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    let delivery = &state.tasks[task].deliveries[0];
    let work = delivery.work.as_ref().ok_or("delivery work missing")?;
    Ok((
        delivery.id,
        work.snapshot.clone().ok_or("snapshot missing")?,
    ))
}

fn proof_entry(
    artifact: std::path::PathBuf,
    digest: Digest,
    snapshot: domain::acceptance::Snapshot,
) -> Result<ProofEntry, domain::ids::InvalidId> {
    Ok(ProofEntry {
        criterion: "AC1".parse()?,
        status: ProofStatus::Passed,
        kind: ProofKind::Command,
        artifact,
        digest,
        snapshot,
        measurement: Some(Measurement {
            baseline: Digest::from_str(&"b".repeat(64))?,
            observed: "result now present".into(),
            satisfied: true,
        }),
        implementation: vec!["src/lib.rs:1".into()],
        description: "exercised public behavior".into(),
    })
}

fn prepare_escalation(
    app: &mut App<'_>,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    prepare(app)?;
    proof_ready(app, directory)?;
    let task = TaskId::from_str("GAIN-2")?;
    app.execute(
        Command::EscalateTier {
            task: task.clone(),
            to: domain::risk::Tier::Lite,
            reason: "cross-component risk".into(),
        },
        false,
    )?;
    Ok(task)
}

fn spent_events(output: &app::Output) -> usize {
    output
        .events
        .iter()
        .filter(|record| matches!(record.event, Event::BudgetSpent { .. }))
        .count()
}

fn record_prior_review(
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::open(directory)?;
    let launch = Launch {
        id: LaunchId(40),
        task: task.clone(),
        delivery: DeliveryId(1),
        role: AgentRole::Reviewer,
        prompt: directory.join("review.md"),
        session: None,
        counted: true,
        process: None,
    };
    store.commit(
        Actor::Harness,
        10,
        &[
            Event::LaunchStarted { launch },
            Event::LaunchEnded {
                launch: LaunchId(40),
                result: LaunchOutcome::Completed {
                    session: "prior-review".into(),
                    output: "review result".into(),
                },
                usage: None,
            },
        ],
    )?;
    Ok(())
}

#[test]
fn readiness_is_free() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: prompt(directory.path())?,
            fallback: None,
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
    match app.execute(Command::Status, false)?.result {
        ResultData::State { state } => assert_eq!(state.tasks[&task].budgets.reviews, 0),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn malformed_review_is_free() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    proof_ready(&mut app, directory.path())?;
    let task = TaskId::from_str("GAIN-2")?;
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: prompt(directory.path())?,
            fallback: None,
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
    match app.execute(Command::Status, false)?.result {
        ResultData::State { state } => assert_eq!(state.tasks[&task].budgets.reviews, 0),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn rejected_launch_spends_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = TaskId::from_str("GAIN-2")?;
    let result = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        true,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Evidence(_),
            ..
        })
    ));
    match app.execute(Command::Status, false)?.result {
        ResultData::State { state } => assert_eq!(state.tasks[&task].budgets.reviews, 0),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn escalation_requires_prior_review() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let result = app.execute(
        Command::RunAgent {
            task: TaskId::from_str("GAIN-2")?,
            role: AgentRole::Escalation,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        true,
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
fn escalation_spends_both_budgets() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = prepare_escalation(&mut app, directory.path())?;
    drop(app);
    record_prior_review(directory.path(), &task)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let output = app.execute(
        Command::RunAgent {
            task,
            role: AgentRole::Escalation,
            prompt: prompt(directory.path())?,
            fallback: None,
        },
        true,
    )?;
    assert_eq!(spent_events(&output), 2);
    Ok(())
}
