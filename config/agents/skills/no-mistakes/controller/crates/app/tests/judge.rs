//! The judge: `complete` starts one in the background, `judge` records a
//! verdict from a well-formed report and rejects a malformed one as a
//! failed launch, a task is judged once, `next` ignores a judge launch,
//! and `usage-report` leaves the trace open while judges are out.

mod common;

use adapters::sqlite::Store;
use app::{App, ConflictReason, Rejection, ResultData};
use common::golden::verified_task;
use common::literals::{jira_status, transition_id};
use common::*;
use domain::{
    command::{AgentRole, Command, NextAction, Transition},
    event::{Event, LaunchOutcome},
    ids::{IssueKey, TaskId},
    judge::{JudgeVerdict, RejectionNote},
    ports::{TraceState, Tracer},
};
use std::cell::{Cell, RefCell};

const REPORT: &str = r#"{
  "outcome": {"score": 90, "rationale": "merged with proof"},
  "efficiency": {"score": 80, "rationale": "one review"},
  "friction": {"score": 60, "rationale": "a slot had to be cleared"},
  "process": {"score": 100, "rationale": "rules held"},
  "overall": 84,
  "verdict": "degraded",
  "friction_events": [{
    "kind": "slot-administration",
    "description": "bind-slot rejected until the slot was cleared",
    "source": "rejections",
    "severity": "moderate",
    "avoidable": true
  }],
  "summary": "delivered, with one avoidable slot clearance",
  "improvements": ["let bind-slot reuse a slot whose owner completed"]
}"#;

#[derive(Default)]
struct Recording {
    finished: Cell<bool>,
    quality: RefCell<Vec<u32>>,
}

impl Tracer for Recording {
    fn record(&self, _: &mut dyn TraceState, _: &[domain::event::EventRecord]) {}
    fn finish(&self, _: &mut dyn TraceState) {
        self.finished.set(true);
    }
    fn quality(&self, _: &mut dyn TraceState, payload: &domain::judge::QualityPayload) {
        self.quality.borrow_mut().push(payload.rollup.judged);
    }
}

fn confirm_done(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    let done = jira_status("done");
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: jira_status("progress"),
            target: done.clone(),
            transitions: vec![Transition {
                id: transition_id("61"),
                to: done.clone(),
            }],
        },
        false,
    )?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            status: done,
            evidence: "fresh connector read".into(),
        },
        false,
    )?;
    Ok(())
}

fn completed_task(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<TaskId, Box<dyn std::error::Error>> {
    let task = verified_task(app, fake, directory)?;
    confirm_done(app, &task)?;
    app.execute(Command::Complete { task: task.clone() }, false)?;
    Ok(task)
}

#[test]
fn complete_starts_a_detached_judge() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = completed_task(&mut app, &fake, directory.path())?;
    let spawned = fake.spawned.borrow();
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].args[0], "judge");
    assert_eq!(spawned[0].args[1], "--run");
    assert_eq!(spawned[0].args[3], task.to_string());
    assert!(spawned[0].args[2].ends_with(&directory.path().to_string_lossy().to_string()));
    Ok(())
}

#[test]
fn judge_records_a_verdict_and_publishes_quality() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let recording = Recording::default();
    let mut services = services(&fake);
    services.tracer = &recording;
    let mut app = App::new(Store::open(directory.path())?, services);
    let task = completed_task(&mut app, &fake, directory.path())?;
    Store::open(directory.path())?.record_rejection(&RejectionNote {
        at: 11,
        command: "bind-slot".into(),
        task: Some(task.clone()),
        check: false,
        class: "conflict".into(),
        message: "slot occupied".into(),
    })?;
    *fake.reviewer_output.borrow_mut() = REPORT.into();

    let output = app.execute(Command::Judge { task: task.clone() }, false)?;
    let kinds: Vec<String> = output
        .events
        .iter()
        .filter_map(|record| serde_json::to_value(&record.event).ok())
        .filter_map(|value| value["kind"].as_str().map(ToOwned::to_owned))
        .collect();
    assert_eq!(kinds, ["launch-started", "launch-ended", "judged"]);
    assert!(matches!(output.result, ResultData::Launch { .. }));
    let judged = task_state(&mut app, &task)?;
    let judgment = judged.judgment.ok_or("no judgment")?;
    assert_eq!(judgment.report.verdict, JudgeVerdict::Degraded);
    assert_eq!(judgment.report.friction_events.len(), 1);
    assert_eq!(*recording.quality.borrow(), [1]);
    let prompt = std::fs::read_to_string(
        directory
            .path()
            .join("judge")
            .join(format!("{task}.prompt.md")),
    )?;
    assert!(prompt.contains("# Judge instructions"));
    assert!(prompt.contains("slot occupied"));
    assert!(prompt.contains("## Ledger timeline"));

    let again = app.execute(Command::Judge { task: task.clone() }, false);
    assert!(matches!(
        again,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::AlreadyJudged),
            ..
        })
    ));
    Ok(())
}

#[test]
fn malformed_report_settles_a_failed_launch() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = completed_task(&mut app, &fake, directory.path())?;
    *fake.reviewer_output.borrow_mut() = "not json".into();
    let result = app.execute(Command::Judge { task: task.clone() }, false);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Invalid(_),
            ..
        })
    ));
    let state = match app.execute(Command::Status, false)?.result {
        ResultData::State { state, .. } => state,
        _ => return Err("status".into()),
    };
    let launch = state
        .launches
        .iter()
        .rev()
        .find(|launch| launch.role == AgentRole::Judge)
        .ok_or("no judge launch")?;
    assert!(matches!(launch.outcome, Some(LaunchOutcome::Failed { .. })));
    assert!(state.tasks[&task].judgment.is_none());
    Ok(())
}

#[test]
fn judge_check_proposes_only_the_launch() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = completed_task(&mut app, &fake, directory.path())?;
    let output = app.execute(Command::Judge { task: task.clone() }, true)?;
    assert_eq!(output.events.len(), 1);
    assert!(
        matches!(&output.events[0].event, Event::LaunchStarted { launch } if launch.role == AgentRole::Judge)
    );
    assert!(task_state(&mut app, &task)?.judgment.is_none());
    Ok(())
}

#[test]
fn unclaimed_task_is_not_judged() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = task()?.id;
    app.execute(
        Command::Discover {
            tasks: vec![common::task()?],
            planning_order: None,
        },
        false,
    )?;
    let result = app.execute(Command::Judge { task }, false);
    assert!(matches!(
        result,
        Err(app::AgentError {
            why: Rejection::Conflict(ConflictReason::JudgeRequiresClaimedTask),
            ..
        })
    ));
    Ok(())
}

#[test]
fn usage_report_defers_the_trace_close_while_judges_run() -> Result<(), Box<dyn std::error::Error>>
{
    let (directory, fake) = initialized()?;
    let recording = Recording::default();
    let mut services = services(&fake);
    services.tracer = &recording;
    let mut app = App::new(Store::open(directory.path())?, services);
    let task = completed_task(&mut app, &fake, directory.path())?;
    // The judge `complete` started is (as far as this store knows) still
    // out, so the handoff must not close the instance yet.
    app.execute(Command::UsageReport, false)?;
    assert!(!recording.finished.get());
    // A `next` meanwhile never asks to monitor the judge's launch.
    *fake.reviewer_output.borrow_mut() = REPORT.into();
    app.execute(Command::Judge { task }, false)?;
    assert!(recording.finished.get());
    assert_eq!(Store::open(directory.path())?.judges_pending()?, 0);
    let next = app.execute(Command::Next, false)?;
    assert!(!matches!(
        next.result,
        ResultData::Next { next: Some(action) } if matches!(action.action, NextAction::MonitorLaunch { .. })
    ));
    Ok(())
}
