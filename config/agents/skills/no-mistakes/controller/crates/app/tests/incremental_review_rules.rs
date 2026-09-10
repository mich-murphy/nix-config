mod common;

use adapters::sqlite::Store;
use app::App;
use common::golden::{arrange_passing_review, plan_and_implement, record_proof_for};
use common::literals::{digest, write_text};
use common::*;
use domain::{
    command::{AgentRole, Command},
    ids::TaskId,
};

/// Records AC1 proof against the current snapshot and runs a PASS review
/// of it, returning the prompt the reviewer was given.
fn prove_and_review(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    directory: &std::path::Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (delivery, snapshot) = current_delivery(app, task)?;
    record_proof_for(
        app,
        task,
        delivery,
        "AC1",
        digest('b'),
        snapshot,
        directory.join(format!("proof-{name}.txt")),
    )?;
    arrange_passing_review(fake)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt: write_text(directory, &format!("review-{name}.md"), "bounded task"),
            fallback: None,
        },
        false,
    )?;
    Ok(fake.last_prompt.borrow().clone().unwrap_or_default())
}

/// One more implementer turn on a new commit, checkpointed (and so
/// snapshotted), the shape of a repair after review.
fn repair_turn(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fake.head.set('b');
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt: write_text(directory, "repair.md", "bounded repair"),
            fallback: None,
        },
        false,
    )?;
    app.execute(
        Command::Checkpoint {
            task: task.clone(),
            advanced: true,
            observation: "repair applied".into(),
            next: "re-prove and re-review".into(),
            outside_paths: Vec::new(),
            scope_reason: None,
        },
        false,
    )?;
    Ok(())
}

/// A new commit after a settled review no longer discards that review:
/// it becomes the delivery's prior review, and the next reviewer's own
/// prompt names the head it covered so the reviewer can concentrate on
/// the diff since it. The first review of a delivery carries no such
/// packet.
#[test]
fn repair_review_is_told_about_the_prior_review() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;

    let first = prove_and_review(&mut app, &fake, &task, directory.path(), "first")?;
    assert!(!first.contains("Prior review covered head"));

    repair_turn(&mut app, &fake, &task, directory.path())?;
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(delivery.review.is_none());
    let prior = delivery
        .prior_review
        .as_ref()
        .ok_or("prior review missing")?;
    assert_eq!(prior.snapshot.head, sha('a')?);

    let second = prove_and_review(&mut app, &fake, &task, directory.path(), "second")?;
    assert!(second.contains(&format!("Prior review covered head {}", sha('a')?)));
    Ok(())
}
