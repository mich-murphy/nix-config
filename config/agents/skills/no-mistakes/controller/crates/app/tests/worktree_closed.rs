mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::literals::task_id;
use common::*;
use domain::{
    command::Command,
    delivery::{Delivery, PrState, PullRequest},
    event::{Actor, Event},
    ids::{DeliveryId, PrNumber, TaskId},
};

/// The task's current delivery, read back and cloned through
/// `Command::Status`, so a test can compare it before and after an
/// attempted mutation without repeating the destructure at each call site.
fn last_delivery(app: &mut App<'_>, task: &TaskId) -> Result<Delivery, Box<dyn std::error::Error>> {
    task_state(app, task)?
        .deliveries
        .last()
        .cloned()
        .ok_or_else(|| "delivery missing".into())
}

/// Records an open PR for the task's first delivery, then observes it as
/// merged, closing that delivery; returns the closed delivery itself so a
/// caller can compare it, unchanged, after later worktree activity.
fn merge_first_delivery(
    directory: &std::path::Path,
    fake: &Fake,
    task: &TaskId,
) -> Result<Delivery, Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: task.clone(),
            delivery: DeliveryId(1),
            pr: PullRequest {
                number: PrNumber(1),
                state: PrState::Open,
                draft: false,
                head: sha('a')?,
                merge: None,
                checks: Vec::new(),
            },
        }],
    )?;
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: PrNumber(1),
        state: PrState::Merged,
        draft: false,
        head: sha('a')?,
        merge: Some(sha('a')?),
        checks: Vec::new(),
    });
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;
    last_delivery(&mut app, task)
}

/// Once a delivery is merged, changes to the fake worktree (head moved,
/// slot state changed) neither mutate it nor let it be re-snapshotted:
/// `snapshot` is rejected because the task is no longer `Planned`/`InFlight`,
/// and the closed delivery's own fields are untouched.
#[test]
fn closed_delivery_ignores_worktree_changes() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    drop(app);
    let before = merge_first_delivery(directory.path(), &fake, &task)?;

    let mut fake = fake;
    fake.head.set('c');
    fake.slot = domain::ports::SlotState::Checkout {
        clean: false,
        head: sha('c')?,
    };

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = app.execute(Command::Snapshot { task: task.clone() }, false);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(_),
            ..
        })
    ));
    let after = last_delivery(&mut app, &task)?;
    assert_eq!(before, after);
    Ok(())
}
