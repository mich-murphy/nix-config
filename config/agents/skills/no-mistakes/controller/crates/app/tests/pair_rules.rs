mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::literals::{criterion_id, slot_id, task_id};
use common::*;
use domain::{
    authority::{AuthorityError, AuthorityReceipt, Grant},
    budget::BudgetKind,
    command::{AgentRole, Command, NextAction},
    event::Event,
    ids::TaskId,
    task::HoldReason,
};

/// A `Pair` grant with no scope, for `task`, against its current
/// requirements digest.
fn pair_receipt(
    directory: &std::path::Path,
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<AuthorityReceipt, Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let artifact = directory.join(format!("pair-receipt-{task}.txt"));
    let digest = write_artifact(&artifact, "one additional implementer/reviewer cycle")?;
    Ok(AuthorityReceipt {
        source: "current user".into(),
        artifact,
        digest,
        requirements,
        grant: Grant::Pair { paths: None },
    })
}

/// A `Pair` grant against a task with no hold is rejected, and no
/// `AuthorityRegistered` event is written.
#[test]
fn pair_requires_hold() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = task_id("GAIN-2");
    discover_claim(&mut app, &task)?;
    let receipt = pair_receipt(directory.path(), &mut app, &task)?;
    let result = app.execute(
        Command::Grant {
            task: task.clone(),
            receipt,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::PairRequiresHold),
            ..
        })
    ));
    assert!(task_state(&mut app, &task)?.authorities.is_empty());
    Ok(())
}

/// Claims, briefs, binds (to `slot`) and plans `task`, reaching `Planned`
/// with `Implement` as its next action. Generalizes `common::prepare` (which
/// is fixed to GAIN-2 on `worker1`) to any task and slot, so a second task
/// can be brought up alongside the first.
fn claim_bind_and_plan(
    app: &mut App<'_>,
    task: &TaskId,
    slot: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(Command::Claim { task: task.clone() }, false)?;
    sync_progress(app, task)?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![domain::task::Criterion {
                id: criterion_id("AC1"),
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: task.clone(),
            slot: slot_id(slot),
            branch: format!("agent/{slot}/{task}"),
            authority: None,
        },
        false,
    )?;
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: domain::task::Plan {
                deliverable: "observable result".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: std::collections::BTreeMap::from([(
                    criterion_id("AC1"),
                    common::literals::digest('b'),
                )]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    Ok(())
}

fn implement_turns(app: &mut App<'_>) -> Result<u32, Box<dyn std::error::Error>> {
    match app.execute(Command::Next, false)?.result {
        app::ResultData::Next {
            next: Some(envelope),
        } => match envelope.action {
            NextAction::Implement {
                remaining_turns, ..
            } => Ok(remaining_turns),
            other => Err(format!("expected Implement, found {other:?}").into()),
        },
        _ => Err("next returned no action".into()),
    }
}

fn discover_two(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Discover {
            tasks: vec![task()?, task_named("GAIN-3")?],
            planning_order: None,
        },
        false,
    )?;
    Ok(())
}

/// A `CiPending` hold with a deadline still in the future releases `task`
/// as the run's active claim without being "actionable" (design Section
/// 19: `next` reports only an actionable hold ahead of a fresh claim), the
/// same shape `queue_behaviors.rs::record_ci_wait` uses so a held task
/// does not block the next one from being claimed.
fn hold_with_future_ci_wait(
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        domain::event::Actor::Coordinator,
        10,
        &[domain::event::Event::Held {
            task: task.clone(),
            reason: HoldReason::CiPending {
                pr: domain::ids::PrNumber(1),
                head: sha('a')?,
                deadline: 1_000,
            },
            at: 10,
        }],
    )?;
    Ok(())
}

/// Discovers both GAIN-2 and GAIN-3, brings GAIN-2 (task A) up through
/// `Planned` on `worker1`, then holds it with a not-yet-actionable
/// `CiPending` wait, releasing it as the run's active claim: the shared
/// starting state `pair_is_task_scoped` needs before task B can be
/// claimed alongside it.
fn setup_two_tasks(
    directory: &std::path::Path,
    fake: &Fake,
) -> Result<(TaskId, TaskId), Box<dyn std::error::Error>> {
    let a = task_id("GAIN-2");
    let b = task_id("GAIN-3");
    let mut app = App::new(Store::open(directory)?, services(fake));
    discover_two(&mut app)?;
    claim_bind_and_plan(&mut app, &a, "worker1")?;
    drop(app);
    hold_with_future_ci_wait(directory, &a)?;
    Ok((a, b))
}

/// Grants `task` an unscoped `Pair`.
fn grant_pair(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let receipt = pair_receipt(directory, app, task)?;
    app.execute(
        Command::Grant {
            task: task.clone(),
            receipt,
        },
        false,
    )?;
    Ok(())
}

/// A `Pair` grant on one held task never changes another task's own
/// budget: task B's `Implement.remaining_turns` is unaffected by a grant
/// made to task A.
#[test]
fn pair_is_task_scoped() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let (a, b) = setup_two_tasks(directory.path(), &fake)?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    claim_bind_and_plan(&mut app, &b, "worker2")?;

    let before = implement_turns(&mut app)?;
    grant_pair(&mut app, directory.path(), &a)?;
    let after = implement_turns(&mut app)?;
    assert_eq!(before, after);
    Ok(())
}

/// Writes `docs/guide.md` into the repo, holds `task`, then grants it a
/// `Pair` scoped to that one file: the path-scoped authority
/// `scoped_pair_needs_acceptance` runs an implementer turn against.
fn grant_scoped_docs_pair(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory.join("docs"))?;
    write_artifact(&directory.join("docs/guide.md"), "guide")?;
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
    let requirements = task_state(app, task)?.spec.requirements;
    let artifact = directory.join("scoped-pair-receipt.txt");
    let digest = write_artifact(&artifact, "one documentation-only cycle")?;
    app.execute(
        Command::Grant {
            task: task.clone(),
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact,
                digest,
                requirements,
                grant: Grant::Pair {
                    paths: Some(vec!["docs/guide.md".into()]),
                },
            },
        },
        false,
    )?;
    Ok(())
}

/// Runs one implementer turn for `task` and reports whether its
/// `BudgetSpent` event was uncounted (a path-scoped pair spends no
/// budget).
fn implementer_turn_uncounted(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<bool, Box<dyn std::error::Error>> {
    let prompt = directory.join("implement.md");
    std::fs::write(&prompt, "bounded task")?;
    let output = app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Implementer,
            prompt,
            fallback: None,
        },
        false,
    )?;
    Ok(output.events.iter().any(|record| {
        matches!(
            &record.event,
            Event::BudgetSpent {
                task: id,
                budget: BudgetKind::Implementation,
                counted: false,
            } if *id == *task
        )
    }))
}

/// A path-scoped `Pair` grant (docs only) lets an implementer launch run
/// uncounted (`BudgetSpent.counted == false`), but `final-verify` is still
/// rejected without full proof and a PASS review: an uncounted turn is not
/// acceptance.
#[test]
fn scoped_pair_needs_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    grant_scoped_docs_pair(&mut app, directory.path(), &task)?;
    let uncounted = implementer_turn_uncounted(&mut app, directory.path(), &task)?;
    assert!(uncounted, "expected an uncounted BudgetSpent event");

    let evidence = directory.path().join("final-verify.txt");
    write_artifact(&evidence, "final verification evidence")?;
    let result = app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit: sha('a')?,
            evidence,
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
    Ok(())
}
