mod common;

use adapters::sqlite::Store;
use app::{App, ResultData};
use common::literals::{criterion_id, digest, discovered_task, slot_id, task_id, write_text};
use common::script::{
    Step, execute, final_verify, record_proof, review, run_steps, step, sync, unchecked,
};
use common::*;
use domain::{
    command::{AgentRole, Command, NextAction, PublishStep},
    ports::MergeMethod,
    task::{Criterion, Phase, Plan},
};
use std::collections::BTreeMap;

fn claims(task: domain::ids::TaskId) -> impl Fn(&NextAction) -> bool {
    move |action| matches!(action, NextAction::Claim { task: id } if *id == task)
}

fn publishes(want: PublishStep) -> impl Fn(&NextAction) -> bool {
    move |action| matches!(action, NextAction::Publish { step, .. } if *step == want)
}

/// Builds the whole `code_delivery_reaches_cleanup` script: one `Step` per
/// `next` action the coordinator sees, in order, from discovery through
/// `cleanup`. Fixture values (`TaskId`, `Digest`, paths) come from
/// `common::literals`'s infallible constructors, so the table itself needs
/// no `?`; the three Jira round trips are `script::sync`, which asserts
/// `next` on both of its own halves, so they need no outer `expect`.
fn code_delivery_script(directory: &std::path::Path) -> Vec<Step> {
    let task = task_id("GAIN-2");
    let ac1_baseline = digest('b');
    vec![
        unchecked(execute(Command::Discover {
            tasks: vec![discovered_task("GAIN-2")],
            planning_order: None,
        })),
        step(
            claims(task.clone()),
            execute(Command::Claim { task: task.clone() }),
        ),
        unchecked(sync("todo", "progress", "31")),
        step(
            |action| matches!(action, NextAction::Brief { .. }),
            execute(Command::Brief {
                task: task.clone(),
                criteria: vec![Criterion {
                    id: criterion_id("AC1"),
                    text: "observable result".into(),
                    human_only: false,
                }],
            }),
        ),
        step(
            |action| matches!(action, NextAction::BindSlot { .. }),
            execute(Command::BindSlot {
                task: task.clone(),
                slot: slot_id("worker1"),
                branch: "agent/worker1/GAIN-2".into(),
                authority: None,
            }),
        ),
        step(
            |action| matches!(action, NextAction::Plan { .. }),
            execute(Command::Plan {
                task: task.clone(),
                plan: Plan {
                    deliverable: "observable result".into(),
                    components: vec!["src".into()],
                    examples: Vec::new(),
                    baselines: BTreeMap::from([(criterion_id("AC1"), ac1_baseline.clone())]),
                    lesson_families: vec!["controller".into()],
                },
            }),
        ),
        step(
            |action| matches!(action, NextAction::Implement { .. }),
            execute(Command::RunAgent {
                task: task.clone(),
                role: AgentRole::Implementer,
                prompt: write_text(directory, "implement.md", "bounded task"),
                fallback: None,
            }),
        ),
        step(
            |action| matches!(action, NextAction::Checkpoint { .. }),
            execute(Command::Checkpoint {
                task: task.clone(),
                advanced: true,
                observation: "implementation complete".into(),
                next: "record proof".into(),
                outside_paths: Vec::new(),
                scope_reason: None,
            }),
        ),
        step(
            |action| matches!(action, NextAction::RecordProof { .. }),
            record_proof("AC1", ac1_baseline, directory.join("proof.txt")),
        ),
        step(
            |action| matches!(action, NextAction::Review { .. }),
            review(write_text(directory, "review.md", "bounded task")),
        ),
        // The reviewer's PASS opens no PR yet, so `next` asks for `Create`
        // before it asks Jira to reflect Review (design Section 11: the
        // review sync gate only applies once a PR exists).
        step(
            publishes(PublishStep::Create),
            execute(Command::Publish {
                task: task.clone(),
                step: PublishStep::Create,
                title: Some("Deliver AC1".into()),
                body: Some(write_text(directory, "pr-body.md", "delivers AC1")),
                method: None,
            }),
        ),
        unchecked(sync("progress", "review", "41")),
        step(
            publishes(PublishStep::Ready),
            execute(Command::Publish {
                task: task.clone(),
                step: PublishStep::Ready,
                title: None,
                body: None,
                method: None,
            }),
        ),
        step(
            publishes(PublishStep::Merge),
            execute(Command::Publish {
                task: task.clone(),
                step: PublishStep::Merge,
                title: None,
                body: None,
                method: Some(MergeMethod::Squash),
            }),
        ),
        step(
            |action| matches!(action, NextAction::FinalVerify { .. }),
            final_verify(directory.join("final-verify.txt")),
        ),
        unchecked(sync("review", "done", "51")),
        step(
            |action| matches!(action, NextAction::Complete { .. }),
            execute(Command::Complete { task: task.clone() }),
        ),
        step(
            |action| matches!(action, NextAction::Cleanup { .. }),
            execute(Command::Cleanup {
                task: task.clone(),
                delete: true,
            }),
        ),
    ]
}

/// Drives one code delivery from discovery through cleanup, asserting
/// before every step that `Command::Next` names the action about to be
/// performed. This is the single rule under test: the coordinator's
/// contract with `next` holds across a whole successful run, not only in
/// isolated commands. `code_delivery_script` builds the table; `run_steps`
/// is the loop that asserts and performs each step in turn.
#[test]
fn code_delivery_reaches_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = task_id("GAIN-2");

    run_steps(
        &mut app,
        &fake,
        &task,
        code_delivery_script(directory.path()),
    )?;

    let state = task_state(&mut app, &task)?;
    assert!(matches!(state.phase, Phase::Completed { .. }));
    let ResultData::State { state: whole, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    assert!(whole.active.is_none());

    expect_next(
        &mut app,
        |action| matches!(action, NextAction::Report { remaining } if remaining.is_empty()),
    )?;
    Ok(())
}
