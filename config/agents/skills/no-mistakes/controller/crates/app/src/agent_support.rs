use crate::{AgentError, App, Rejection};
use adapters::sqlite::Store;
use domain::{
    event::{Actor, Event, EventRecord},
    ports::{Clock, Harness, LaunchRequest, LaunchResult, PortError, ProcessIdentity},
};

pub(super) fn invoke(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    request: &LaunchRequest,
    events: &mut Vec<Event>,
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
    events: &mut Vec<Event>,
) -> Result<(Vec<EventRecord>, LaunchResult), PortError> {
    let mut records = Vec::new();
    let result = harness.run(request, &mut |identity| {
        let start = owned_launch(events, identity)?;
        records = store
            .commit(Actor::Coordinator, clock.now(), &[start])
            .map_err(|error| PortError(error.to_string()))?;
        Ok(())
    })?;
    Ok((records, result))
}

fn owned_launch(events: &mut Vec<Event>, identity: ProcessIdentity) -> Result<Event, PortError> {
    let position = events
        .iter()
        .position(|event| matches!(event, Event::LaunchStarted { .. }))
        .ok_or_else(|| PortError("launch start event missing".into()))?;
    let mut start = events.remove(position);
    if let Event::LaunchStarted { launch } = &mut start {
        launch.process = Some(identity);
    }
    Ok(start)
}
