//! A small table-driven runner for the golden-path scripts
//! (`../golden_path.rs`, `../golden_path_verification.rs`,
//! `../golden_path_recovery.rs`). Each scenario is a linear sequence of
//! `Step`s built once, up front, from infallible literals
//! (`super::literals`); `run_steps` is the one loop that asserts `next`
//! (design Section 11: the model never performs a step `next` did not just
//! name) and performs the step, so a test body needs only the handful of
//! `?`s the table itself cannot absorb.
//!
//! Every step's action is a boxed closure (`ActionFn`) rather than a
//! `match`-dispatched enum, so `run_steps` stays a plain loop and each
//! action's own logic is scored on its own, separately from both the loop
//! and the test that built the table.

use super::golden::{arrange_passing_review, record_proof_for};
use super::literals::{jira_status, transition_id};
use super::{Fake, current_delivery, expect_next, task_state};
use app::App;
use domain::{
    acceptance::Snapshot,
    command::{AgentRole, Command, NextAction, Transition},
    ids::{Digest, IssueKey, TaskId},
    task::Phase,
};
use std::path::PathBuf;

pub type Expect = Box<dyn Fn(&NextAction) -> bool>;
type StepError = Box<dyn std::error::Error>;
pub type ActionFn = Box<dyn FnOnce(&mut App<'_>, &Fake, &TaskId) -> Result<(), StepError>>;

/// One scripted step: assert `next` matches `expect` when present, then run
/// `action`.
pub struct Step {
    pub expect: Option<Expect>,
    pub action: ActionFn,
}

/// Builds a `Step` that asserts `next` first. Most table entries use this.
pub fn step(expect: impl Fn(&NextAction) -> bool + 'static, action: ActionFn) -> Step {
    Step {
        expect: Some(Box::new(expect)),
        action,
    }
}

/// Builds a `Step` with no `next` assertion, for an action whose own
/// internal round trip (for example `sync`) already covers it, or a
/// fake-harness adjustment that is not itself a modelled action.
pub fn unchecked(action: ActionFn) -> Step {
    Step {
        expect: None,
        action,
    }
}

pub fn run_steps(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    steps: Vec<Step>,
) -> Result<(), StepError> {
    for entry in steps {
        if let Some(expect) = entry.expect {
            expect_next(app, expect)?;
        }
        (entry.action)(app, fake, task)?;
    }
    Ok(())
}

/// Does nothing, for a step that only asserts `next` with no command of
/// its own to run (the fake clock or harness has already been adjusted by
/// an earlier, `unchecked` step).
pub fn nothing() -> ActionFn {
    Box::new(|_app, _fake, _task| Ok(()))
}

/// Runs `command` as-is, for a step whose fields are all known when the
/// table is built.
pub fn execute(command: Command) -> ActionFn {
    Box::new(move |app, _fake, _task| {
        app.execute(command, false)?;
        Ok(())
    })
}

/// Records one passing `Command`-kind proof entry for `criterion`, reading
/// the current delivery and snapshot back from state since neither is
/// known until a prior step has taken one.
pub fn record_proof(criterion: &'static str, baseline: Digest, artifact: PathBuf) -> ActionFn {
    Box::new(move |app, _fake, task| {
        let (delivery, snapshot) = current_delivery(app, task)?;
        record_proof_for(app, task, delivery, criterion, baseline, snapshot, artifact)
    })
}

/// Records one passing `Command`-kind proof entry for `criterion` against
/// an explicit `snapshot`, for a delivery with no `PlannedWork` of its own
/// (a `Verification` delivery, reopened against a merge commit) where
/// `current_delivery` cannot read one back.
pub fn record_proof_with(
    criterion: &'static str,
    baseline: Digest,
    snapshot: Snapshot,
    artifact: PathBuf,
) -> ActionFn {
    Box::new(move |app, _fake, task| {
        let delivery = last_delivery_id(app, task)?;
        record_proof_for(app, task, delivery, criterion, baseline, snapshot, artifact)
    })
}

/// Predicts the reviewer's launch id with `--check`, installs a PASS
/// review for the current snapshot, then runs the real reviewer launch.
pub fn review(prompt: PathBuf) -> ActionFn {
    Box::new(move |app, fake, task| {
        let (_, snapshot) = current_delivery(app, task)?;
        review_step(app, fake, task, prompt, snapshot)
    })
}

/// `review`, against an explicit `snapshot` rather than one read back from
/// `current_delivery`, for a delivery with no `PlannedWork` of its own (a
/// `Verification` delivery).
pub fn review_with(prompt: PathBuf, snapshot: Snapshot) -> ActionFn {
    Box::new(move |app, fake, task| review_step(app, fake, task, prompt, snapshot))
}

fn review_step(
    app: &mut App<'_>,
    fake: &Fake,
    task: &TaskId,
    prompt: PathBuf,
    snapshot: Snapshot,
) -> Result<(), StepError> {
    arrange_passing_review(app, fake, task, &prompt, snapshot)?;
    app.execute(
        Command::RunAgent {
            task: task.clone(),
            role: AgentRole::Reviewer,
            prompt,
            fallback: None,
        },
        false,
    )?;
    Ok(())
}

/// Writes final-verification evidence and records it against the commit
/// the task's `Merged` phase reports, since that commit is not known until
/// `publish merge` has run.
pub fn final_verify(artifact: PathBuf) -> ActionFn {
    Box::new(move |app, _fake, task| {
        let commit = merged_commit(app, task)?;
        super::write_artifact(&artifact, "final verification evidence")?;
        app.execute(
            Command::FinalVerify {
                task: task.clone(),
                commit,
                evidence: artifact,
            },
            false,
        )?;
        Ok(())
    })
}

/// Moves the fake `Vcs::head` to simulate a new commit between snapshots.
pub fn move_head(head: char) -> ActionFn {
    Box::new(move |_app, fake, _task| {
        fake.head.set(head);
        Ok(())
    })
}

/// Toggles whether the fake bakes a pending required check into every PR
/// it records, simulating CI still running.
pub fn pending_checks(pending: bool) -> ActionFn {
    Box::new(move |_app, fake, _task| {
        fake.pending_checks.set(pending);
        Ok(())
    })
}

/// Nudges the fake clock by `delta` seconds (negative to rewind), the way
/// a recovery scenario steps up to and past a per-head deadline.
pub fn adjust_clock(delta: i64) -> ActionFn {
    Box::new(move |_app, fake, _task| {
        let current = fake.clock.get();
        let adjusted = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta.unsigned_abs())
        };
        fake.clock.set(adjusted);
        Ok(())
    })
}

/// One Jira round trip (`set-status` then `observe-status`) from `current`
/// to `target`, asserting `next` for each half before performing it
/// (design Section 11: `next` never skips a step the coordinator is about
/// to take). Always built with `unchecked`: the two internal assertions
/// are the ones that matter.
pub fn sync(current: &'static str, target: &'static str, transition: &'static str) -> ActionFn {
    Box::new(move |app, _fake, task| sync_step(app, task, current, target, transition))
}

fn sync_step(
    app: &mut App<'_>,
    task: &TaskId,
    current: &str,
    target: &str,
    transition: &str,
) -> Result<(), StepError> {
    let target = jira_status(target);
    let issue = IssueKey::from(task.clone());
    let awaited = target.clone();
    expect_next(
        app,
        move |action| matches!(action, NextAction::SyncStatus { target: value, .. } if *value == awaited),
    )?;
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: issue.clone(),
            current: jira_status(current),
            target: target.clone(),
            transitions: vec![Transition {
                id: transition_id(transition),
                to: target.clone(),
            }],
        },
        false,
    )?;
    expect_next(app, |action| {
        matches!(action, NextAction::ObserveStatus { .. })
    })?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue,
            status: target,
            evidence: "fresh connector read".into(),
        },
        false,
    )?;
    Ok(())
}

/// The commit a task's phase reports once `Merged`, the value a
/// `final_verify` step or a delivery reopened against that merge needs.
pub fn merged_commit(app: &mut App<'_>, task: &TaskId) -> Result<domain::ids::Sha, StepError> {
    let state = task_state(app, task)?;
    let Phase::Merged { commit, .. } = state.phase else {
        return Err(format!("task did not reach Merged: {:?}", state.phase).into());
    };
    Ok(commit)
}

/// The most recently opened delivery's id, read back from state for a
/// delivery (such as a `Verification` one) with no `PlannedWork` of its
/// own to carry it.
fn last_delivery_id(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<domain::ids::DeliveryId, StepError> {
    let state = task_state(app, task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    Ok(delivery.id)
}
