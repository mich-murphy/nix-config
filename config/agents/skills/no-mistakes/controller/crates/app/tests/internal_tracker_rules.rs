mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{arrange_passing_review, record_proof_for};
use common::literals::{criterion_id, digest, write_text};
use common::script::merged_commit;
use common::*;
use domain::{
    command::{AgentRole, Command, NextAction, PublishStep},
    ids::{IssueKey, JiraStatus, TaskId},
    ports::MergeMethod,
    task::{Criterion, Phase},
};
use std::str::FromStr;

/// The full delivery loop for a run with no external tracker: every step
/// the autonomous-mode golden path runs, minus the tracker sync the
/// external path interleaves. Proves the ledger alone can carry a task
/// from claim to completion.
#[test]
fn internal_run_completes_without_tracker_sync() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_internal()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(&mut app, &task)?;
    expect_next(&mut app, |action| {
        matches!(action, NextAction::Brief { .. })
    })?;
    brief_bind_plan(&mut app, &task)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: common::golden::prompt_file(directory.path(), "implement.md")?,
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced: true,
            observation: "implementation complete".into(),
            next: "record proof".into(),
            outside_paths: Vec::new(),
            scope_reason: None,
        },
        false,
    )?;
    let (delivery, snapshot) = current_delivery(&mut app, &task)?;
    record_proof_for(
        &mut app,
        &task,
        delivery,
        "AC1",
        digest('b'),
        snapshot,
        directory.path().join("proof-ac1.txt"),
    )?;
    arrange_passing_review(&fake)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: write_text(directory.path(), "review.md", "bounded task"),
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory.path(), "pr-body.md", "delivers AC1")),
            method: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Ready,
            title: None,
            body: None,
            method: None,
        },
        false,
    )?;
    // No review-status sync gate: a ready PR with green checks goes
    // straight to merge.
    expect_next(&mut app, |action| {
        matches!(
            action,
            NextAction::Publish {
                step: PublishStep::Merge,
                ..
            }
        )
    })?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Merge,
            title: None,
            body: None,
            method: Some(MergeMethod::Squash),
        },
        false,
    )?;
    let commit = merged_commit(&mut app, &task)?;
    app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit,
            evidence: write_text(directory.path(), "final-verify.txt", "final evidence"),
        },
        false,
    )?;
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Verified { .. }
    ));
    // No Done sync stands between Verified and Complete either.
    expect_next(&mut app, |action| {
        matches!(action, NextAction::Complete { .. })
    })?;
    app.execute(Command::Complete { task: task.clone() }, false)?;
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Completed { .. }
    ));
    Ok(())
}

/// `set-status`, `observe-status`, and `subtask-record` mutate a tracker
/// this run does not have; each must reject rather than record a phantom
/// transition.
#[test]
fn tracker_commands_reject_without_a_tracker() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_internal()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(&mut app, &task)?;
    let set = app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: JiraStatus::from_str("todo")?,
            target: JiraStatus::from_str("progress")?,
            transitions: Vec::new(),
        },
        false,
    );
    let observe = app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            status: JiraStatus::from_str("progress")?,
            evidence: "fresh read".into(),
        },
        false,
    );
    let subtask = app.execute(
        Command::SubtaskRecord {
            task: task.clone(),
            issue: IssueKey::from_str("GAIN-9")?,
            criteria: vec![criterion_id("AC1")],
            owned: true,
            was_terminal: false,
            status: JiraStatus::from_str("todo")?,
        },
        false,
    );
    for result in [set, observe, subtask] {
        assert!(matches!(
            result,
            Err(app::AgentError {
                why: Rejection::Conflict(app::ConflictReason::NoExternalTracker),
                ..
            })
        ));
    }
    assert!(matches!(task_state(&mut app, &task)?.phase, Phase::Claimed));
    Ok(())
}

/// A tracked run may not skip the fresh read: `open-delivery` with no
/// tracker read rejects, because omitting it would bypass membership,
/// ownership, and criteria-freshness validation.
#[test]
fn tracked_run_requires_fresh_read_for_verification() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(&mut app, &task)?;
    sync_progress(&mut app, &task)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![Criterion {
                id: criterion_id("AC1"),
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    let result = app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: domain::delivery::DeliveryKind::Verification { of: sha('a')? },
            receipt: None,
            jira: None,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(app::ConflictReason::TrackerReadRequired),
            ..
        })
    ));
    Ok(())
}
