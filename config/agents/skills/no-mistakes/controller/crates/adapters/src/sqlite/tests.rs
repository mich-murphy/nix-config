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
fn launch_has_one_winner() -> Result<(), StoreError> {
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

#[test]
fn trace_state_survives_reopen_and_is_not_an_event() -> Result<(), Box<dyn std::error::Error>> {
    use domain::ports::TraceState;
    let directory = tempfile::tempdir()?;
    {
        let mut store = Store::create(directory.path())?;
        store.commit(Actor::Coordinator, 1, &[launch(1)])?;
        assert_eq!(store.get("instance")?, None);
        store.set("instance", "i1")?;
        store.set("instance", "i2")?;
    }
    let mut store = Store::open(directory.path())?;
    assert_eq!(store.get("instance")?, Some("i2".into()));
    assert_eq!(store.events()?.len(), 1);
    store.verify()?;
    store.remove("instance")?;
    assert_eq!(store.get("instance")?, None);
    Ok(())
}

#[test]
fn rejections_and_judges_live_beside_the_ledger() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let store = Store::create(directory.path())?;
    let task: domain::ids::TaskId = "GAIN-2".parse()?;
    store.record_rejection(&domain::judge::RejectionNote {
        at: 7,
        command: "bind-slot".into(),
        task: Some(task.clone()),
        check: false,
        class: "conflict".into(),
        message: "slot occupied".into(),
    })?;
    store.record_rejection(&domain::judge::RejectionNote {
        at: 8,
        command: "status".into(),
        task: None,
        check: true,
        class: "invalid".into(),
        message: "bad".into(),
    })?;
    let notes = store.rejections()?;
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].task.as_ref(), Some(&task));
    assert!(!notes[0].check && notes[1].check);
    assert_eq!(notes[1].task, None);
    assert!(store.events()?.is_empty());

    assert_eq!(store.judges_pending()?, 0);
    store.judge_started(&task)?;
    store.judge_started(&task)?;
    assert_eq!(store.judges_pending()?, 1);
    assert_eq!(store.judge_finished(&task)?, 0);
    Ok(())
}

#[test]
fn judge_launch_does_not_take_the_single_process_claim() -> Result<(), StoreError> {
    let directory = tempfile::tempdir().map_err(error)?;
    let mut store = Store::create(directory.path())?;
    store.commit(Actor::Coordinator, 1, &[launch(1)])?;
    let judge = match launch(2) {
        Event::LaunchStarted { mut launch } => {
            launch.role = domain::command::AgentRole::Judge;
            Event::LaunchStarted { launch }
        }
        other => other,
    };
    store.commit(Actor::Coordinator, 2, &[judge])?;
    assert!(store.commit(Actor::Coordinator, 3, &[launch(3)]).is_err());
    Ok(())
}
