mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::literals::{slot_id, task_id};
use common::*;
use domain::{
    acceptance::Proof,
    authority::{AuthorityError, AuthorityReceipt, Grant},
    command::Command,
    delivery::{Delivery, DeliveryKind, Outcome},
    event::{Actor, Event},
    ids::{DeliveryId, TaskId},
    task::{PlannedWork, Receipt, SlotBinding, SlotOrigin},
};

/// Discovers `tasks`, claims `owner`, binds it to `slot`, and directly
/// records `owner` as `Completed` (bypassing the rest of the pipeline, the
/// way `recovery_rules.rs`'s `mark_verified` does): the shared shape of a
/// closed delivery a slot's later, different owner must reckon with.
fn complete_on_slot(
    directory: &std::path::Path,
    fake: &Fake,
    tasks: Vec<domain::command::DiscoveredTask>,
    owner: &TaskId,
    slot: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::Discover {
            tasks,
            planning_order: None,
        },
        false,
    )?;
    app.execute(
        Command::Claim {
            task: owner.clone(),
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: owner.clone(),
            slot: slot_id(slot),
            branch: format!("agent/{slot}/{owner}"),
            authority: None,
        },
        false,
    )?;
    drop(app);
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[
            Event::Verified {
                task: owner.clone(),
                receipt: Receipt::Delivery { commit: sha('a')? },
            },
            Event::Completed {
                task: owner.clone(),
            },
            Event::SlotReleased {
                task: owner.clone(),
                slot: slot_id(slot),
            },
        ],
    )?;
    Ok(())
}

/// A slot any closed delivery used cannot be bound by another task without
/// a slot-reuse grant.
#[test]
fn bind_rejects_historical_slot() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let owner = task_id("GAIN-2");
    let other = task_id("GAIN-3");
    complete_on_slot(
        directory.path(),
        &fake,
        vec![task()?, task_named("GAIN-3")?],
        &owner,
        "worker1",
    )?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Claim {
            task: other.clone(),
        },
        false,
    )?;
    let before = fake.vcs_calls.borrow().len();
    let result = app.execute(
        Command::BindSlot {
            task: other,
            slot: slot_id("worker1"),
            branch: "agent/worker1/GAIN-3".into(),
            authority: None,
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::SlotReuseRequiresGrant),
            ..
        })
    ));
    assert_eq!(fake.vcs_calls.borrow().len(), before);
    Ok(())
}

/// Registers a `SlotReuse` grant on `task` naming `historical`/`slot`,
/// returning the assigned authority id.
fn grant_slot_reuse(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
    historical: &TaskId,
    slot: &str,
) -> Result<domain::ids::AuthorityId, Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let artifact = directory.join(format!("slot-reuse-{task}.txt"));
    let digest = write_artifact(&artifact, "reuse the historical slot")?;
    app.execute(
        Command::Grant {
            task: task.clone(),
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact,
                digest: digest.clone(),
                requirements,
                grant: Grant::SlotReuse {
                    historical: historical.clone(),
                    slot: slot_id(slot),
                },
            },
        },
        false,
    )?;
    let state = task_state(app, task)?;
    state
        .authorities
        .iter()
        .find(|entry| entry.digest == digest)
        .map(|entry| entry.id)
        .ok_or_else(|| "slot-reuse authority missing".into())
}

/// The grant's `historical` task must actually be `Completed`; naming one
/// that is not is rejected even though the slot itself matches.
#[test]
fn slot_reuse_requires_completed_owner() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let owner = task_id("GAIN-2");
    let claimant = task_id("GAIN-3");
    let unfinished = task_id("GAIN-4");
    complete_on_slot(
        directory.path(),
        &fake,
        vec![task()?, task_named("GAIN-3")?, task_named("GAIN-4")?],
        &owner,
        "worker1",
    )?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let authority = grant_slot_reuse(
        &mut app,
        directory.path(),
        &claimant,
        &unfinished,
        "worker1",
    )?;
    app.execute(
        Command::Claim {
            task: claimant.clone(),
        },
        false,
    )?;
    let before = fake.vcs_calls.borrow().len();
    let result = app.execute(
        Command::BindSlot {
            task: claimant,
            slot: slot_id("worker1"),
            branch: "agent/worker1/GAIN-3".into(),
            authority: Some(authority),
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::SlotReuseOwnerIncomplete),
            ..
        })
    ));
    assert_eq!(fake.vcs_calls.borrow().len(), before);
    Ok(())
}

/// Directly records `claimant`'s first delivery already carrying non-empty
/// `work` (bound fresh to `worker2`, as if a bind-slot had already run):
/// the "started" shape a `SlotReuse` grant must not be usable against.
fn commit_started_delivery(
    directory: &std::path::Path,
    claimant: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::DeliveryOpened {
            task: claimant.clone(),
            delivery: Box::new(Delivery {
                id: DeliveryId(1),
                kind: DeliveryKind::Code { pr: None },
                authority: None,
                criteria: Vec::new(),
                paths: None,
                base: sha('a')?,
                work: Some(PlannedWork {
                    slot: SlotBinding {
                        slot: slot_id("worker2"),
                        origin: SlotOrigin::Fresh,
                    },
                    branch: format!("agent/worker2/{claimant}"),
                    base: sha('a')?,
                    snapshot: None,
                    plan: None,
                    feedback: Vec::new(),
                    snapshot_launch: None,
                }),
                proof: Proof::default(),
                review: None,
                human_review: None,
                check_deadlines: std::collections::BTreeMap::new(),
                prior_review: None,
                outcome: Outcome::Open,
                launches: Vec::new(),
                operations: Vec::new(),
            }),
        }],
    )?;
    Ok(())
}

/// A `SlotReuse` grant only binds a slot for a delivery that has not yet
/// started: one directly recorded with non-empty `work` is rejected.
#[test]
fn slot_reuse_requires_unstarted_delivery() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let owner = task_id("GAIN-2");
    let claimant = task_id("GAIN-3");
    complete_on_slot(
        directory.path(),
        &fake,
        vec![task()?, task_named("GAIN-3")?],
        &owner,
        "worker1",
    )?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let authority = grant_slot_reuse(&mut app, directory.path(), &claimant, &owner, "worker1")?;
    app.execute(
        Command::Claim {
            task: claimant.clone(),
        },
        false,
    )?;
    drop(app);
    commit_started_delivery(directory.path(), &claimant)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let before = fake.vcs_calls.borrow().len();
    let result = app.execute(
        Command::BindSlot {
            task: claimant,
            slot: slot_id("worker1"),
            branch: "agent/worker1/GAIN-3".into(),
            authority: Some(authority),
        },
        false,
    );
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Authority(AuthorityError::SlotReuseRequiresUnstartedDelivery),
            ..
        })
    ));
    assert_eq!(fake.vcs_calls.borrow().len(), before);
    Ok(())
}

/// A valid slot-reuse bind refreshes the new delivery's base to the
/// current head (not the historical owner's) and calls `reuse_slot`, never
/// `bind_slot`.
#[test]
fn slot_reuse_refreshes_base() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, mut fake) = initialized()?;
    let owner = task_id("GAIN-2");
    let claimant = task_id("GAIN-3");
    complete_on_slot(
        directory.path(),
        &fake,
        vec![task()?, task_named("GAIN-3")?],
        &owner,
        "worker1",
    )?;
    // The historical owner's checkout is still on disk (a clean worktree
    // at worker1), the shape a real `reuse_slot` reuses rather than
    // rebinding fresh.
    fake.slot = domain::ports::SlotState::Checkout {
        clean: true,
        head: sha('a')?,
    };

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let authority = grant_slot_reuse(&mut app, directory.path(), &claimant, &owner, "worker1")?;
    fake.head.set('b');
    app.execute(
        Command::Claim {
            task: claimant.clone(),
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: claimant.clone(),
            slot: slot_id("worker1"),
            branch: "agent/worker1/GAIN-3".into(),
            authority: Some(authority),
        },
        false,
    )?;
    let state = task_state(&mut app, &claimant)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert_eq!(delivery.base, sha('b')?);
    assert_eq!(
        fake.vcs_calls.borrow().last(),
        Some(&"reuse_slot"),
        "expected reuse_slot, not bind_slot"
    );
    Ok(())
}
