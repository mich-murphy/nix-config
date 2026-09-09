mod common;

use adapters::sqlite::Store;
use app::{App, Rejection, ResultData};
use common::golden::{narrow_record_review_and_merge, plan_and_implement, record_proof_for};
use common::literals::{digest, write_text};
use common::*;
use domain::{
    command::{AgentRole, Command, PublishStep},
    delivery::{DeliveryKind, Outcome, PrState, PullRequest},
    event::{Actor, Event},
    ids::{DeliveryId, FindingId, PrNumber, TaskId},
    ports::MergeMethod,
    review::{Category, Finding, ReviewReport, Severity, Verdict},
    task::{HoldReason, Phase},
};
use std::str::FromStr;

/// `plan_and_implement`, then a recorded AC1 proof and a `report`-shaped
/// review, published through `create` and `ready`: the starting point
/// every `merge_requires_*` test needs, one step before the merge attempt
/// each varies.
fn ready_for_merge(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
    report: ReviewReport,
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
        directory.join("proof.txt"),
    )?;
    *fake.reviewer_output.borrow_mut() = serde_json::to_string(&report)?;
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

fn pass_report() -> ReviewReport {
    ReviewReport {
        findings: Vec::new(),
        evidence_gaps: Vec::new(),
        reviewer_opinion: Verdict::Pass,
    }
}

fn critical_report() -> Result<ReviewReport, domain::ids::InvalidId> {
    Ok(ReviewReport {
        findings: vec![Finding {
            id: FindingId::from_str("F1")?,
            severity: Severity::Critical,
            category: Category::Correctness,
            location: "src/lib.rs:1".into(),
            trigger: "input".into(),
            consequence: "failure".into(),
            correction: "fix".into(),
        }],
        evidence_gaps: Vec::new(),
        reviewer_opinion: Verdict::Pass,
    })
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

fn assert_rejected_open(
    app: &mut App<'_>,
    task: &TaskId,
    result: Result<app::Output, app::AgentError>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Evidence(_),
            ..
        })
    ));
    assert!(!matches!(
        task_state(app, task)?.phase,
        Phase::Merged { .. }
    ));
    Ok(())
}

/// `publish --step merge` is rejected when the review verdict is not
/// PASS: a settled review with a Critical finding computes to
/// `ChangesRequired`, and merge is refused without ever closing the
/// delivery.
#[test]
fn merge_requires_pass() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path(), critical_report()?)?;
    let result = attempt_merge(&mut app, &task);
    assert_rejected_open(&mut app, &task, result)
}

/// `publish --step merge` is rejected while a required check is pending
/// (or failing), even with a PASS review recorded.
#[test]
fn merge_requires_green_checks() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path(), pass_report())?;
    fake.pending_checks.set(true);
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;
    let result = attempt_merge(&mut app, &task);
    assert_rejected_open(&mut app, &task, result)
}

/// `publish --step merge` is rejected when the PR's remote head has
/// drifted from the one the PASS review actually covered (a new commit
/// pushed to the PR after review, observed independently of `publish`).
#[test]
fn merge_requires_reviewed_head() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path(), pass_report())?;
    drop(app);
    Store::open(directory.path())?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: task.clone(),
            delivery: DeliveryId(1),
            pr: PullRequest {
                number: PrNumber(1),
                state: PrState::Open,
                draft: false,
                head: sha('c')?,
                merge: None,
                checks: Vec::new(),
            },
        }],
    )?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let result = attempt_merge(&mut app, &task);
    assert_rejected_open(&mut app, &task, result)
}

/// `DeliveryClosed { Merged }` (and the phase change it drives) is only
/// written from a GitHub observation that actually reports `MERGED` with
/// a merge commit: a `merge` (or `observe-pr`) that comes back `Open`
/// leaves the delivery `Open` and the task's phase untouched.
#[test]
fn merged_requires_observation() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = ready_for_merge(&mut app, &fake, directory.path(), pass_report())?;
    *fake.observation.borrow_mut() = Some(PullRequest {
        number: PrNumber(1),
        state: PrState::Open,
        draft: false,
        head: sha('a')?,
        merge: None,
        checks: Vec::new(),
    });
    app.execute(
        Command::ObservePr {
            task: task.clone(),
            pr: PrNumber(1),
        },
        false,
    )?;
    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    assert!(matches!(delivery.outcome, Outcome::Open));
    assert!(!matches!(state.phase, Phase::Merged { .. }));
    Ok(())
}

/// A narrowed merge passes the GitHub port the narrowed review's own
/// snapshot head, not some other one: moving the fake's head between the
/// re-plan and the narrowed snapshot means the merged PR's recorded head
/// must be the moved-to commit the narrowed review actually covered.
#[test]
fn merge_uses_reviewed_head() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_and_implement(&mut app, directory.path())?;
    narrow_record_review_and_merge(&mut app, &fake, &task, directory.path(), Some('c'))?;

    let state = task_state(&mut app, &task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    let reviewed_head = delivery
        .review
        .as_ref()
        .ok_or("review missing")?
        .snapshot
        .head
        .clone();
    assert_eq!(reviewed_head, sha('c')?);
    match &delivery.kind {
        DeliveryKind::Code { pr: Some(pr) } => assert_eq!(pr.head, reviewed_head),
        other => return Err(format!("expected a merged code delivery, found {other:?}").into()),
    }
    Ok(())
}

fn pull_request(state: PrState) -> Result<PullRequest, domain::ports::PortError> {
    Ok(PullRequest {
        number: PrNumber(1),
        state,
        draft: false,
        head: sha('a')?,
        merge: (state == PrState::Merged).then(|| sha('b')).transpose()?,
        checks: Vec::new(),
    })
}

fn held_for_human(app: &mut App<'_>) -> Result<bool, Box<dyn std::error::Error>> {
    let ResultData::State { state, .. } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    Ok(matches!(
        state.tasks[&TaskId::from_str("GAIN-2")?]
            .hold
            .as_ref()
            .map(|hold| &hold.reason),
        Some(HoldReason::NeedsHuman { .. })
    ))
}

fn prepared_external_merge() -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    let merged = pull_request(PrState::Merged)?;
    let (directory, fake) = initialized_with(Fake {
        reserve: true,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(Some(merged)),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
        on_main: std::cell::Cell::new(true),
        vcs_calls: std::cell::RefCell::new(Vec::new()),
        last_session: std::cell::RefCell::new(None),
        create_fails_once: std::cell::Cell::new(false),
    })?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    prepare(&mut app)?;
    drop(app);
    Store::open(directory.path())?.commit(
        Actor::Coordinator,
        10,
        &[Event::PrObserved {
            task: TaskId::from_str("GAIN-2")?,
            delivery: DeliveryId(1),
            pr: pull_request(PrState::Open)?,
        }],
    )?;
    Ok((directory, fake))
}

#[test]
fn external_merge_holds() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = prepared_external_merge()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::ObservePr {
            task: TaskId::from_str("GAIN-2")?,
            pr: PrNumber(1),
        },
        false,
    )?;
    assert!(held_for_human(&mut app)?);
    Ok(())
}
