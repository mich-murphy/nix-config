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
    if let Some(path) = &request.output_schema {
        write_review_schema(path)
            .map_err(|message| app.error("run-agent", Some(task), Rejection::Internal(message)))?;
    }
    invoke_harness(
        &mut app.store,
        app.services.harness,
        app.services.clock,
        app.services.tracer,
        request,
        events,
    )
    .map_err(|error| app.error("run-agent", Some(task), Rejection::External(error.0)))
}

/// Writes the reviewer's output schema for a `Native` harness
/// (`Capabilities::structured_output`), so `--output-schema` names a real
/// file: `schemars::schema_for!` is called here, in `app`, never in
/// `domain`.
fn write_review_schema(path: &std::path::Path) -> Result<(), String> {
    let schema = schemars::schema_for!(domain::review::ReviewReport);
    let encoded = serde_json::to_string_pretty(&schema).map_err(|error| error.to_string())?;
    std::fs::write(path, encoded).map_err(|error| error.to_string())
}

fn invoke_harness(
    store: &mut Store,
    harness: &dyn Harness,
    clock: &dyn Clock,
    tracer: &dyn domain::ports::Tracer,
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
        tracer.record(store, &records);
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
