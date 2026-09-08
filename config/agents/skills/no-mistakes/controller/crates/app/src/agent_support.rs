use crate::{AgentError, App, Rejection};
use adapters::sqlite::Store;
use domain::{
    event::{Actor, Event, EventRecord},
    ports::{Clock, Harness, LaunchRequest, LaunchResult, PortError, ProcessIdentity},
};

/// Commits `events` (the launch's `LaunchStarted`, `BudgetSpent` and any
/// `PairSpent`) as one transaction inside the harness's `started` callback,
/// before the process is released past the gate. A launch that then times
/// out, is cancelled, or fails is already charged; only `LaunchEnded` (and,
/// for a valid review, `ReviewSettled`) is written afterwards.
pub(super) fn invoke(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    request: &LaunchRequest,
    events: Vec<Event>,
) -> Result<(Vec<EventRecord>, LaunchResult), AgentError> {
    invoke_harness(
        &mut app.store,
        app.services.harness,
        app.services.clock,
        request,
        events,
    )
    .map_err(|error| app.error("run-agent", Some(task), Rejection::External(error.0)))
}

fn invoke_harness(
    store: &mut Store,
    harness: &dyn Harness,
    clock: &dyn Clock,
    request: &LaunchRequest,
    events: Vec<Event>,
) -> Result<(Vec<EventRecord>, LaunchResult), PortError> {
    let mut records = Vec::new();
    let mut pending = Some(events);
    let result = harness.run(request, &mut |identity| {
        let batch = owned_batch(&mut pending, identity)?;
        records = store
            .commit(Actor::Coordinator, clock.now(), &batch)
            .map_err(|error| PortError(error.to_string()))?;
        Ok(())
    })?;
    Ok((records, result))
}

fn owned_batch(
    pending: &mut Option<Vec<Event>>,
    identity: ProcessIdentity,
) -> Result<Vec<Event>, PortError> {
    let mut batch = pending
        .take()
        .ok_or_else(|| PortError("launch already started".into()))?;
    let launch = batch
        .iter_mut()
        .find_map(|event| match event {
            Event::LaunchStarted { launch } => Some(launch),
            _ => None,
        })
        .ok_or_else(|| PortError("launch start event missing".into()))?;
    launch.process = Some(identity);
    Ok(batch)
}
