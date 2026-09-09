use super::*;

fn launch(id: u64) -> Event {
    Event::LaunchStarted {
        launch: domain::event::Launch {
            id: domain::ids::LaunchId(id),
            task: "GAIN-1"
                .parse()
                .unwrap_or_else(|error| panic!("fixture: {error}")),
            delivery: domain::ids::DeliveryId(1),
            role: domain::command::AgentRole::Implementer,
            prompt: "/prompt".into(),
            outcome: None,
            usage: None,
            checkpointed: false,
            process: None,
        },
    }
}

#[test]
fn simultaneous_launch_has_one_winner() -> Result<(), StoreError> {
    let directory = tempfile::tempdir().map_err(error)?;
    let mut first = Store::create(directory.path())?;
    let mut second = Store::open(directory.path())?;
    first.commit(Actor::Coordinator, 1, &[launch(1)])?;
    assert!(second.commit(Actor::Coordinator, 1, &[launch(2)]).is_err());
    Ok(())
}

#[test]
fn status_matches_fold() -> Result<(), StoreError> {
    let directory = tempfile::tempdir().map_err(error)?;
    let mut store = Store::create(directory.path())?;
    store.commit(Actor::Coordinator, 1, &[launch(1)])?;
    // Corrupt the projection row directly, bypassing `commit`, so the
    // stored row and the folded event log disagree.
    store
        .connection
        .execute(
            "UPDATE projection SET state = ?1 WHERE singleton = 1",
            params![serde_json::to_string(&State::empty()).map_err(error)?],
        )
        .map_err(error)?;
    assert!(store.verify().is_err());
    // `state` is the read path: it trusts the row and still returns.
    assert!(store.state()?.tasks.is_empty());
    Ok(())
}
