mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for};
use common::literals::{digest, write_text};
use common::*;
use domain::{
    authority::{AuthorityError, AuthorityReceipt, Grant},
    command::{AgentRole, Command, NextAction, PublishStep, Transition},
    delivery::Outcome,
    ids::{IssueKey, JiraStatus, TaskId, TransitionId},
    ports::MergeMethod,
};
use std::str::FromStr;

/// The golden path one step before merge: planned, proven, PASS-reviewed,
/// published through `create` and `ready`. Every test here varies what a
/// receipt-required run does at that boundary.
fn ready_for_merge(
    app: &mut App<'_>,
    fake: &Fake,
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
        directory.join("proof-ac1.txt"),
    )?;
    arrange_passing_review(fake)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: write_text(directory, "review.md", "bounded task"),
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Create,
            title: Some("Deliver AC1".into()),
            body: Some(write_text(directory, "pr-body.md", "delivers AC1")),
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
    Ok(task)
}

fn attempt_merge(app: &mut App<'_>, task: &TaskId) -> Result<app::Output, app::AgentError> {
    app.execute(
        Command::Publish {
            task: task.clone(),
            step: PublishStep::Merge,
            title: None,
            body: None,
            method: Some(MergeMethod::Squash),
        },
        false,
    )
}

fn grant_merge(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let artifact = directory.join("merge-receipt.md");
    let digest = write_artifact(&artifact, "merge this delivery")?;
    app.execute(
        Command::Grant {
            task: task.clone(),
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact: artifact.clone(),
                digest,
                requirements,
                grant: Grant::Merge,
            },
        },
        false,
    )?;
    Ok(artifact)
}

#[test]
fn merge_without_receipt_is_rejected_and_leaves_pr_open() -> Result<(), Box<dyn std::error::Error>>
{
    let (directory, fake) = initialized_with_merge_policy(false)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path())?;
    let result = attempt_merge(&mut app, &task);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::MergeRequiresReceipt),
            ..
        })
    ));
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(matches!(delivery.outcome, Outcome::Open));
    Ok(())
}

#[test]
fn next_asks_for_merge_authority_instead_of_merging() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_merge_policy(false)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path())?;
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: JiraStatus::from_str("progress")?,
            target: JiraStatus::from_str("review")?,
            transitions: vec![Transition {
                id: TransitionId::from_str("41")?,
                to: JiraStatus::from_str("review")?,
            }],
        },
        false,
    )?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            status: JiraStatus::from_str("review")?,
            evidence: "fresh read".into(),
        },
        false,
    )?;
    expect_next(&mut app, |action| {
        matches!(action, NextAction::RequestMergeAuthority { .. })
    })?;
    Ok(())
}

#[test]
fn granted_receipt_allows_exactly_one_merge() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_merge_policy(false)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path())?;
    grant_merge(&mut app, directory.path(), &task)?;
    attempt_merge(&mut app, &task).map_err(|error| format!("merge rejected: {error}"))?;
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(matches!(delivery.outcome, Outcome::Merged { .. }));
    let receipt = state
        .authorities
        .iter()
        .find(|entry| matches!(entry.grant, Grant::Merge))
        .ok_or("merge authority missing")?;
    assert!(receipt.used.whole.is_some());
    Ok(())
}

#[test]
fn changed_receipt_artifact_blocks_merge() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_merge_policy(false)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path())?;
    let artifact = grant_merge(&mut app, directory.path(), &task)?;
    std::fs::write(&artifact, "a different answer after the fact")?;
    let result = attempt_merge(&mut app, &task);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::ReceiptChanged),
            ..
        })
    ));
    Ok(())
}

#[test]
fn autonomous_mode_merges_without_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized_with_merge_policy(true)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path())?;
    attempt_merge(&mut app, &task).map_err(|error| format!("merge rejected: {error}"))?;
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(matches!(delivery.outcome, Outcome::Merged { .. }));
    Ok(())
}
