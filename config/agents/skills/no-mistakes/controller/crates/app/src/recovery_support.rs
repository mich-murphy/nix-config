use crate::{AgentError, App, ConflictReason, Output, Rejection};
use adapters::process::Recovery;
use domain::{
    event::{Event, Launch, LaunchOutcome},
    ids::LaunchId,
};

pub(super) fn recover_launch(
    app: &mut App<'_>,
    state: &domain::state::State,
    launch: LaunchId,
    terminate: bool,
    check: bool,
) -> Result<Output, AgentError> {
    let item = unsettled_launch(app, state, launch)?;
    let status = recover_process(app, &item, terminate)?;
    if status == Recovery::Running {
        return Err(app.error(
            "recover-operation",
            Some(&item.task),
            ConflictReason::LaunchStillRunning,
        ));
    }
    let result = match status {
        Recovery::Terminated => LaunchOutcome::Cancelled,
        Recovery::Stopped => LaunchOutcome::Failed {
            reason: "owned launch stopped before settlement".into(),
        },
        Recovery::Running => unreachable!("running launch returned above"),
    };
    app.commit(
        "recover-operation",
        Some(&item.task),
        vec![Event::LaunchEnded {
            launch,
            result,
            usage: None,
        }],
        check,
    )
}

fn unsettled_launch(
    app: &App<'_>,
    state: &domain::state::State,
    launch: LaunchId,
) -> Result<Launch, AgentError> {
    let item = state
        .launches
        .iter()
        .find(|item| item.id == launch)
        .ok_or_else(|| {
            app.error(
                "recover-operation",
                None,
                Rejection::Invalid("unknown launch".into()),
            )
        })?;
    if item.outcome.is_none() {
        Ok(item.clone())
    } else {
        Err(app.error(
            "recover-operation",
            Some(&item.task),
            ConflictReason::LaunchNotRecoverable,
        ))
    }
}

fn recover_process(
    app: &App<'_>,
    launch: &Launch,
    terminate: bool,
) -> Result<Recovery, AgentError> {
    let identity = launch.process.ok_or_else(|| {
        app.error(
            "recover-operation",
            Some(&launch.task),
            ConflictReason::LaunchHasNoProcess,
        )
    })?;
    app.services
        .process
        .recover(identity, terminate)
        .map_err(|error| {
            app.error(
                "recover-operation",
                Some(&launch.task),
                Rejection::External(error.0),
            )
        })
}
