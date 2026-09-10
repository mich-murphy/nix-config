//! The trace tap: every committed event reaches the tracer, a `--check`
//! reaches nothing, and `usage-report` closes the trace.

mod common;

use app::{App, initialize};
use domain::{
    command::{Command, ReviewMode},
    event::EventRecord,
    ports::{TraceState, Tracer},
};
use std::cell::{Cell, RefCell};

#[derive(Default)]
struct Recording {
    kinds: RefCell<Vec<String>>,
    finished: Cell<bool>,
}

impl Tracer for Recording {
    fn record(&self, _: &mut dyn TraceState, records: &[EventRecord]) {
        let mut kinds = self.kinds.borrow_mut();
        for record in records {
            let value = serde_json::to_value(&record.event).unwrap_or_default();
            kinds.push(value["kind"].as_str().unwrap_or_default().to_owned());
        }
    }
    fn finish(&self, _: &mut dyn TraceState) {
        self.finished.set(true);
    }
}

#[test]
fn commits_reach_the_tracer_and_checks_do_not() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = common::initialized()?;
    let recording = Recording::default();
    let mut services = common::services(&fake);
    services.tracer = &recording;
    let mut app = App::new(adapters::sqlite::Store::open(directory.path())?, services);
    let discover = Command::Discover {
        tasks: vec![common::task()?],
        planning_order: None,
    };
    app.execute(discover.clone(), true)?;
    assert!(recording.kinds.borrow().is_empty());
    app.execute(discover, false)?;
    assert_eq!(recording.kinds.borrow().as_slice(), ["queue-discovered"]);
    assert!(!recording.finished.get());
    app.execute(Command::UsageReport, false)?;
    assert!(recording.finished.get());
    Ok(())
}

#[test]
fn initialization_is_traced() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let (_, fake) = common::initialized()?;
    let recording = Recording::default();
    let mut services = common::services(&fake);
    services.tracer = &recording;
    initialize(
        directory.path(),
        common::config(directory.path(), ReviewMode::Autonomous)?,
        common::profile()?,
        services,
    )?;
    assert_eq!(recording.kinds.borrow().as_slice(), ["run-initialized"]);
    Ok(())
}
