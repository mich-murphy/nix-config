mod common;

use adapters::sqlite::Store;
use app::{App, ConflictReason, Rejection};
use common::golden::verified_task;
use common::literals::{jira_status, transition_id};
use common::*;
use domain::{
    command::{Command, Transition},
    ids::{IssueKey, TaskId},
};
use std::str::FromStr;

fn attempt_complete(app: &mut App<'_>, task: &TaskId) -> Result<app::Output, app::AgentError> {
    app.execute(Command::Complete { task: task.clone() }, false)
}

fn assert_rejected_not_done(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = attempt_complete(app, task);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::CompletionRequiresJiraDone),
            ..
        })
    ));
    Ok(())
}

/// One Jira round trip (`set-status` then `observe-status`) confirming
/// `issue` reached `done`, for an issue key the fixed `sync_progress` and
/// `sync_status` helpers do not cover: a subtask's own key, distinct from
/// the parent task's.
fn confirm_done(
    app: &mut App<'_>,
    task: &TaskId,
    issue: IssueKey,
    transition: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let done = jira_status("done");
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: issue.clone(),
            current: jira_status("progress"),
            target: done.clone(),
            transitions: vec![Transition {
                id: transition_id(transition),
                to: done.clone(),
            }],
        },
        false,
    )?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue,
            status: done,
            evidence: "fresh connector read".into(),
        },
        false,
    )?;
    Ok(())
}

/// `complete` is rejected on a `Verified` task whose parent Jira sync has
/// not confirmed `done`, and accepted once it has.
#[test]
fn complete_requires_parent_done() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = verified_task(&mut app, &fake, directory.path())?;

    assert_rejected_not_done(&mut app, &task)?;

    confirm_done(&mut app, &task, IssueKey::from(task.clone()), "61")?;
    attempt_complete(&mut app, &task)?;
    Ok(())
}

/// `complete` is rejected on a `Verified` task whose parent is confirmed
/// `done` but a recorded subtask (`subtask-record`) is not, and accepted
/// once the subtask's own Jira sync confirms `done` too.
#[test]
fn complete_requires_subtasks_done() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = verified_task(&mut app, &fake, directory.path())?;
    confirm_done(&mut app, &task, IssueKey::from(task.clone()), "61")?;

    let subtask = IssueKey::from_str("GAIN-20")?;
    app.execute(
        Command::SubtaskRecord {
            task: task.clone(),
            issue: subtask.clone(),
            criteria: Vec::new(),
            owned: true,
            was_terminal: false,
            status: jira_status("progress"),
        },
        false,
    )?;

    assert_rejected_not_done(&mut app, &task)?;

    confirm_done(&mut app, &task, subtask, "41")?;
    attempt_complete(&mut app, &task)?;
    Ok(())
}
