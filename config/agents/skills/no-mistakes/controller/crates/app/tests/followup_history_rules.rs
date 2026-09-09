mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::plan_implement_and_merge_externally;
use common::literals::{criterion_id, task_id};
use common::*;
use domain::{
    authority::{AuthorityReceipt, Grant},
    command::{Command, JiraRead},
    delivery::{Delivery, DeliveryKind, Outcome, PrState, PullRequest, Replacement},
    event::{Actor, Event},
    ids::{DeliveryId, PrNumber, TaskId},
    task::HoldReason,
};

/// A minimal, but fully valid, `Grant::Delivery` receipt for reopening
/// AC1 as a fresh code delivery.
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

/// `task`'s current requirements digest and a fresh `AuthorityReceipt`
/// built against it, together: every `open-delivery` attempt here needs
/// both, and a receipt built against a stale digest would fail for the
/// wrong reason.
fn open_receipt_and_requirements(
    app: &mut App<'_>,
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(AuthorityReceipt, domain::ids::Digest), Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let receipt = dummy_receipt(directory, requirements.clone())?;
    Ok((receipt, requirements))
}

fn discover_one(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    Ok(())
}

/// A prior `Merged` commit that has since left `origin/main` blocks
/// `open-delivery`, even with an otherwise fully valid receipt and Jira
/// read.
#[test]
fn history_merges_stay_on_main() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_implement_and_merge_externally(&mut app, &fake, directory.path())?;
    fake.on_main.set(false);

    let requirements = task_state(&mut app, &task)?.spec.requirements;
    let receipt = dummy_receipt(directory.path(), requirements.clone())?;
    let deliveries_before = task_state(&mut app, &task)?.deliveries.len();
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
    assert_eq!(state.deliveries.len(), deliveries_before);
    assert!(state.authorities.is_empty());
    Ok(())
}

/// Directly records `task`'s first delivery already closed `Replaced` by
/// PR 99 at a fixed head and merge commit, held `NeedsHuman`: the shape
/// `replacement_identity_is_fixed` needs before it can attempt a follow-up
/// delivery against it.
fn commit_replaced_delivery(
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[
            Event::DeliveryOpened {
                task: task.clone(),
                delivery: Box::new(Delivery {
                    id: DeliveryId(1),
                    kind: DeliveryKind::Code { pr: None },
                    authority: None,
                    criteria: Vec::new(),
                    paths: None,
                    base: sha('a')?,
                    work: None,
                    proof: domain::acceptance::Proof::default(),
                    review: None,
                    human_review: None,
                    check_deadlines: std::collections::BTreeMap::new(),
                    outcome: Outcome::Replaced {
                        by: Replacement {
                            pr: PrNumber(99),
                            head: sha('c')?,
                            merge: sha('d')?,
                        },
                        at: 10,
                    },
                    launches: Vec::new(),
                    operations: Vec::new(),
                }),
            },
            Event::Held {
                task: task.clone(),
                reason: HoldReason::NeedsHuman {
                    diagnosis: "prior delivery replaced externally".into(),
                    remaining: Vec::new(),
                },
                at: 10,
            },
        ],
    )?;
    Ok(())
}

/// A prior `Replaced` PR must still be merged at the exact recorded head
/// and merge commit; a live observation that drifts from that record
/// blocks `open-delivery`.
#[test]
fn replacement_identity_is_fixed() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    discover_one(&mut app)?;
    drop(app);
    let task = task_id("GAIN-2");
    commit_replaced_delivery(directory.path(), &task)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    // The replacement PR is observed again, but its head has drifted from
    // the recorded identity.
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: PrNumber(99),
        state: PrState::Merged,
        draft: false,
        head: sha('f')?,
        merge: Some(sha('d')?),
        checks: Vec::new(),
    });
    let (receipt, requirements) = open_receipt_and_requirements(&mut app, directory.path(), &task)?;
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

fn commit_closed_own_pr(
    directory: &std::path::Path,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    Store::open(directory)?.commit(
        Actor::Coordinator,
        10,
        &[
            Event::PrObserved {
                task: task.clone(),
                delivery: DeliveryId(1),
                pr: PullRequest {
                    number: PrNumber(5),
                    state: PrState::Closed,
                    draft: false,
                    head: sha('a')?,
                    merge: None,
                    checks: Vec::new(),
                },
            },
            Event::Held {
                task: task.clone(),
                reason: HoldReason::SupersededPr { pr: PrNumber(5) },
                at: 10,
            },
        ],
    )?;
    Ok(())
}

/// Sets the fake's live observation to a merged PR reusing the same
/// number as `task`'s own closed PR (as if it were a foreign replacement),
/// then observes it: `observe_own_pr_as_merged` is the "as if" half of
/// `replacement_must_be_foreign`.
fn observe_own_pr_as_merged(
    directory: &std::path::Path,
    fake: &Fake,
    task: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: PrNumber(5),
        state: PrState::Merged,
        draft: false,
        head: sha('f')?,
        merge: Some(sha('e')?),
        checks: Vec::new(),
    });
    let mut app = App::new(Store::open(directory)?, services(fake));
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(5),
        },
        false,
    )?;
    Ok(())
}

/// `observe-pr` naming this delivery's own PR number takes the "known"
/// path, not the replacement one, even when the live observation looks
/// like an external merge: `observed_delivery` treats a known PR as this
/// delivery's own, so no `Replaced` outcome is ever written for it.
#[test]
fn replacement_must_be_foreign() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    let task = task_id("GAIN-2");
    drop(app);
    commit_closed_own_pr(directory.path(), &task)?;
    observe_own_pr_as_merged(directory.path(), &fake, &task)?;

    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(matches!(delivery.outcome, Outcome::Merged { .. }));
    Ok(())
}
