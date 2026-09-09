mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{plan_implement_and_merge_externally, prompt_file};
use common::literals::{criterion_id, slot_id, task_id};
use common::*;
use domain::{
    authority::{AuthorityReceipt, Grant},
    command::{AgentRole, Command, JiraRead},
    delivery::DeliveryKind,
    event::{Actor, Event},
    ids::TaskId,
    task::HoldReason,
};

/// A minimal, but fully valid, `Grant::Delivery` receipt for reopening
/// AC1 as a fresh code delivery, so a test can isolate one `open-delivery`
/// precondition without the authority checks themselves ever failing.
fn dummy_receipt(
    directory: &std::path::Path,
    requirements: domain::ids::Digest,
) -> Result<AuthorityReceipt, Box<dyn std::error::Error>> {
    let artifact = directory.join("open-receipt.txt");
    let digest = write_artifact(&artifact, "resume AC1 as a fresh delivery")?;
    Ok(AuthorityReceipt {
        source: "current user".into(),
        artifact,
        digest,
        requirements,
        grant: Grant::Delivery {
            kind: DeliveryKind::Code { pr: None },
            criteria: vec![criterion_id("AC1")],
            paths: None,
        },
    })
}

fn open_delivery_command(task: &TaskId, receipt: AuthorityReceipt, jira: JiraRead) -> Command {
    Command::OpenDelivery {
        task: task.clone(),
        kind: DeliveryKind::Code { pr: None },
        receipt,
        jira,
    }
}

/// The `previous_session` bug: `task_support::previous_session` used to
/// pick the latest launch for the task and role regardless of delivery, so
/// the first implementer launch on a follow-up delivery resumed the closed
/// delivery's session. A new delivery must start with fresh sessions
/// (design Section 5); scoping the lookup to the current delivery's own
/// launches fixes it, while a same-delivery repair still finds its own
/// prior launch (`reviewer_session_is_separate`, `settlement_failure_is_retryable`
/// stay green).
#[test]
fn open_starts_from_main() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_merge_externally(&mut app, &fake, directory.path())?;

    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let receipt = dummy_receipt(directory.path(), requirements.clone())?;
    app.execute(
        open_delivery_command(
            &task,
            receipt,
            JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        ),
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: task.clone(),
            slot: slot_id("worker2"),
            branch: "agent/worker2/GAIN-2".into(),
            authority: None,
        },
        false,
    )?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: prompt_file(directory.path(), "implement2.md")?,
            fallback: None,
        },
        false,
    )?;
    assert_eq!(*fake.last_session.borrow(), Some(None));
    Ok(())
}

#[test]
fn open_requires_hold() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let receipt = dummy_receipt(directory.path(), requirements.clone())?;
    let result = app.execute(
        open_delivery_command(
            &task,
            receipt,
            JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        ),
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    let state = task_state(&mut app, &task)?;
    assert_eq!(state.deliveries.len(), 1);
    assert!(state.authorities.is_empty());
    Ok(())
}

#[test]
fn open_requires_jira_ownership() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    app.execute(
        Command::Hold {
            task: task.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "blocked pending input".into(),
                remaining: vec![criterion_id("AC1")],
            },
        },
        false,
    )?;
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let receipt = dummy_receipt(directory.path(), requirements.clone())?;
    let result = app.execute(
        open_delivery_command(
            &task,
            receipt,
            JiraRead {
                member: true,
                resolved: true,
                ownership_clear: true,
                requirements,
            },
        ),
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    assert_eq!(task_state(&mut app, &task)?.deliveries.len(), 1);
    Ok(())
}

fn hold_then_mark_completed(
    directory: &std::path::Path,
    fake: &Fake,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::Hold {
            task: task.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "blocked pending input".into(),
                remaining: vec![criterion_id("AC1")],
            },
        },
        false,
    )?;
    drop(app);
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::Completed { task: task.clone() }],
    )?;
    Ok(())
}

#[test]
fn open_rejects_terminal_task() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    drop(app);
    hold_then_mark_completed(directory.path(), &fake, &task)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let receipt = dummy_receipt(directory.path(), requirements.clone())?;
    let result = app.execute(
        open_delivery_command(
            &task,
            receipt,
            JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        ),
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    assert_eq!(task_state(&mut app, &task)?.deliveries.len(), 1);
    Ok(())
}
