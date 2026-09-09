mod common;

use adapters::sqlite::Store;
use app::{App, Rejection};
use common::golden::{plan_review_and_merge, verified_task};
use common::literals::{jira_status, transition_id, write_text};
use common::script::merged_commit;
use common::*;
use domain::{
    command::{Command, Transition},
    ids::{IssueKey, TaskId},
    task::Phase,
};

/// `final-verify` is rejected `Evidence` when the supplied commit is not
/// the recorded merge commit, and again when the commit is not observed
/// on main; only once both hold does it settle the task `Verified`.
#[test]
fn final_verify_checks_main_head() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = plan_review_and_merge(&mut app, &fake, directory.path())?;
    let commit = merged_commit(&mut app, &task)?;

    let wrong = sha('9')?;
    let result = app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit: wrong,
            evidence: write_text(directory.path(), "final-verify-wrong.txt", "evidence"),
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
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Merged { .. }
    ));

    fake.on_main.set(false);
    let result = app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit: commit.clone(),
            evidence: write_text(directory.path(), "final-verify-off-main.txt", "evidence"),
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
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Merged { .. }
    ));
    fake.on_main.set(true);

    app.execute(
        Command::FinalVerify {
            task: task.clone(),
            commit,
            evidence: write_text(directory.path(), "final-verify.txt", "evidence"),
        },
        false,
    )?;
    assert!(matches!(
        task_state(&mut app, &task)?.phase,
        Phase::Verified { .. }
    ));
    Ok(())
}

/// Reads back the artifact path `record_proof_for` wrote for `AC1` on a
/// `Verified` task's only delivery, so a test can corrupt its bytes
/// without knowing the fixture's internal layout.
fn ac1_artifact(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    Ok(task_state(app, task)?
        .deliveries
        .last()
        .ok_or("delivery missing")?
        .proof
        .entries
        .get(&"AC1".parse()?)
        .ok_or("AC1 proof entry missing")?
        .artifact
        .clone())
}

/// `set-status` to Jira Done on a `Verified` task re-checks proof
/// artifacts against their recorded digests: a proof artifact rewritten
/// after `final-verify` fails the Done intent with `Rejection::Evidence`
/// and leaves the task's sync untouched (no `StatusIntended` was ever
/// written).
#[test]
fn done_revalidates_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = verified_task(&mut app, &fake, directory.path())?;

    let artifact = ac1_artifact(&mut app, &task)?;
    std::fs::write(&artifact, "tampered result")?;

    let before = task_state(&mut app, &task)?.sync;
    let result = app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: jira_status("progress"),
            target: jira_status("done"),
            transitions: vec![Transition {
                id: transition_id("61"),
                to: jira_status("done"),
            }],
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
    let after = task_state(&mut app, &task)?.sync;
    assert_eq!(
        before, after,
        "a rejected Done intent must not write StatusIntended"
    );
    Ok(())
}
