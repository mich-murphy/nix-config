use super::support::PENDING;
use super::*;
use crate::ProcessOutput;
use domain::{
    command::AgentRole,
    event::{Actor, Event, Launch, LaunchOutcome},
    ids::{DeliveryId, InvalidId, LaunchId, TaskId},
    ports::Tokens,
};
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    str::FromStr,
};

/// Records every CLI call with the JSON any `@file` argument held at the
/// time, since the adapter removes the file afterwards.
struct Fake {
    calls: RefCell<Vec<(Vec<String>, Vec<Value>)>>,
    failing: Cell<bool>,
}

impl Fake {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            failing: Cell::new(false),
        }
    }

    /// One label per call: `curl <path>` for an HTTP call (the path after
    /// `/api/v1`), otherwise the CLI's first two words.
    fn groups(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .map(|(args, _)| label(args))
            .collect()
    }

    fn item(&self, index: usize) -> Value {
        self.calls.borrow()[index].1[0][0].clone()
    }
}

impl Process for Fake {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        // The HTTP header file is not JSON and is skipped; every other
        // `@file` is a JSON payload worth keeping.
        let files = request
            .args
            .iter()
            .filter_map(|arg| arg.strip_prefix('@'))
            .filter_map(|path| read_json(path).ok())
            .collect::<Vec<Value>>();
        let key = files
            .first()
            .and_then(|items| items[0]["idempotency_key"].as_str())
            .unwrap_or("none")
            .to_owned();
        self.calls.borrow_mut().push((request.args.clone(), files));
        let stdout = json!({
            "status": "success",
            "details": { "id": "i1" },
            "outputs": { key: { "status": "success", "details": { "id": "s1" } } },
        });
        Ok(ProcessOutput {
            code: Some(if self.failing.get() { 1 } else { 0 }),
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }
}

impl Process for &Fake {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        (**self).run(request)
    }
}

fn label(args: &[String]) -> String {
    if args.first().is_some_and(|arg| arg == "-sS") {
        let url = args.get(4).map(String::as_str).unwrap_or_default();
        let path = url.split_once("/api/v1").map_or(url, |(_, path)| path);
        return format!("curl {path}");
    }
    format!("{} {}", args[0], args[1])
}

fn read_json(path: &str) -> Result<Value, PortError> {
    let text = std::fs::read_to_string(path).map_err(|error| PortError(error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| PortError(error.to_string()))
}

#[derive(Default)]
struct Memory(BTreeMap<String, String>);

impl TraceState for Memory {
    fn get(&self, key: &str) -> Result<Option<String>, PortError> {
        Ok(self.0.get(key).cloned())
    }
    fn set(&mut self, key: &str, value: &str) -> Result<(), PortError> {
        self.0.insert(key.into(), value.into());
        Ok(())
    }
    fn remove(&mut self, key: &str) -> Result<(), PortError> {
        self.0.remove(key);
        Ok(())
    }
}

struct Fixed;

impl Clock for Fixed {
    fn now(&self) -> Instant {
        100
    }
    fn sleep(&self, _: u64) {}
}

const SCHEMA: &str = r##"{"oneOf":[
  {"type":"object","properties":{"kind":{"const":"launch-started","type":"string"},"launch":{"$ref":"#/$defs/Launch"}},"required":["kind","launch"]},
  {"type":"object","properties":{"kind":{"const":"completed","type":"string"},"task":{"$ref":"#/$defs/TaskId"}},"required":["kind","task"]}
],"$defs":{"Launch":{"type":"object","properties":{"task":{"$ref":"#/$defs/TaskId"}}},"TaskId":{"type":"string"},"Unused":{"type":"integer"}}}"##;

fn tracing() -> Tracing {
    Tracing {
        agent_id: "a1".into(),
        agent_identifier: "1.0.0".into(),
        api_url: "https://x.test".into(),
        token: "tok".into(),
        capture_inputs: true,
        capture_outputs: true,
    }
}

fn tracer(fake: &Fake) -> Prefactor<&Fake, Fixed> {
    Prefactor::new(fake, Fixed, Some(tracing()), Some(SCHEMA.into()))
}

fn record(sequence: u64, event: Event) -> EventRecord {
    EventRecord {
        sequence,
        at: 50 + sequence,
        actor: Actor::Coordinator,
        event,
    }
}

fn started(prompt: &std::path::Path) -> Result<Event, InvalidId> {
    Ok(Event::LaunchStarted {
        launch: Launch {
            id: LaunchId(7),
            task: TaskId::from_str("GAIN-1")?,
            delivery: DeliveryId(1),
            role: AgentRole::Reviewer,
            prompt: prompt.to_path_buf(),
            outcome: None,
            usage: None,
            checkpointed: false,
            process: None,
        },
    })
}

fn ended(output: &str) -> Event {
    Event::LaunchEnded {
        launch: LaunchId(7),
        result: LaunchOutcome::Completed {
            session: "s".into(),
            output: output.into(),
        },
        usage: Some(Tokens {
            input: 4,
            cached: None,
            output: 1,
        }),
    }
}

fn completed() -> Result<Event, InvalidId> {
    Ok(Event::Completed {
        task: TaskId::from_str("GAIN-1")?,
    })
}

#[test]
fn first_record_registers_and_starts_the_run_instance() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    tracer(&fake).record(&mut state, &[record(1, completed()?)]);
    assert_eq!(
        fake.groups(),
        [
            "curl /agent_instance/register",
            "agent_instances start",
            "bulk execute"
        ]
    );
    let calls = fake.calls.borrow();
    let register = &calls[0].1[0];
    assert_eq!(register["agent_id"], "a1");
    assert_eq!(register["agent_version"]["external_identifier"], "1.0.0");
    let schema_version = &register["agent_schema_version"];
    assert_eq!(
        schema_version["quality_schemas"][0]["name"],
        "delivery-quality"
    );
    assert!(schema_version["quality_schemas"][0]["schema"]["properties"]["rollup"].is_object());
    assert!(schema_version["span_result_schemas"]["completed"].is_object());
    let params = &schema_version["span_schemas"];
    assert!(params["launch-started"]["properties"]["launch"].is_object());
    assert!(params["launch-started"]["properties"].get("kind").is_none());
    assert!(params["launch-started"]["$defs"].get("TaskId").is_some());
    assert!(params["launch-started"]["$defs"].get("Unused").is_none());
    assert_eq!(
        calls[1].0[2..],
        ["i1", "--timestamp", "1970-01-01T00:00:51Z"]
    );
    assert_eq!(state.get(INSTANCE)?, Some("i1".into()));
    Ok(())
}

#[test]
fn event_becomes_a_span_named_by_kind() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    tracer(&fake).record(&mut state, &[record(3, completed()?)]);
    let item = fake.item(2);
    assert_eq!(item["_type"], "agent_spans/create");
    assert_eq!(item["details"]["schema_name"], "completed");
    assert_eq!(item["details"]["status"], "complete");
    assert_eq!(item["details"]["agent_instance_id"], "i1");
    assert_eq!(item["details"]["sensitive_encoding"], true);
    assert_eq!(item["details"]["finished_at"], "1970-01-01T00:00:53Z");
    let payload = &item["details"]["payload"];
    assert_eq!(payload["task"], "GAIN-1");
    assert_eq!(payload["sequence"], 3);
    assert_eq!(payload["actor"], "coordinator");
    assert!(payload.get("kind").is_none());
    assert!(state.get(PENDING)?.is_none());
    Ok(())
}

#[test]
fn launch_opens_a_span_and_its_end_finishes_it() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let prompt = directory.path().join("prompt.md");
    std::fs::write(&prompt, "do the thing")?;
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer = tracer(&fake);
    tracer.record(&mut state, &[record(1, started(&prompt)?)]);
    let open = fake.item(2);
    assert_eq!(open["details"]["status"], "active");
    assert!(open["details"].get("finished_at").is_none());
    assert_eq!(
        open["details"]["payload"]["prompt_text"]["$sensitive"],
        "string"
    );
    assert_eq!(
        open["details"]["payload"]["prompt_text"]["value"],
        "do the thing"
    );
    assert_eq!(state.get("span/launch/7")?, Some("s1".into()));
    tracer.record(&mut state, &[record(2, ended("done"))]);
    let close = fake.item(3);
    assert_eq!(close["_type"], "agent_spans/finish");
    assert_eq!(close["agent_span_id"], "s1");
    assert_eq!(close["status"], "complete");
    assert_eq!(close["sensitive_encoding"], true);
    assert_eq!(close["timestamp"], "1970-01-01T00:00:52Z");
    let result = &close["result_payload"];
    assert_eq!(result["result"]["completed"]["output"]["value"], "done");
    assert_eq!(result["usage"]["input"], 4);
    assert_eq!(state.get("span/launch/7")?, None);
    Ok(())
}

#[test]
fn unsent_records_wait_and_replay_in_order() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer = tracer(&fake);
    tracer.record(&mut state, &[record(1, completed()?)]);
    fake.failing.set(true);
    tracer.record(
        &mut state,
        &[record(2, completed()?), record(3, completed()?)],
    );
    let pending: Vec<EventRecord> = serde_json::from_str(&state.get(PENDING)?.unwrap_or_default())?;
    assert_eq!(pending.len(), 2);
    fake.failing.set(false);
    tracer.record(&mut state, &[record(4, completed()?)]);
    // Call 3 is the failed attempt at sequence 2; 4..7 are the replay.
    let sequences: Vec<u64> = (4..7)
        .map(|index| fake.item(index)["details"]["payload"]["sequence"].as_u64())
        .map(Option::unwrap_or_default)
        .collect();
    assert_eq!(sequences, [2, 3, 4]);
    assert!(state.get(PENDING)?.is_none());
    Ok(())
}

#[test]
fn finish_closes_the_instance_only_when_nothing_is_pending()
-> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer = tracer(&fake);
    tracer.record(&mut state, &[record(1, completed()?)]);
    fake.failing.set(true);
    tracer.record(&mut state, &[record(2, completed()?)]);
    tracer.finish(&mut state);
    assert_eq!(state.get(INSTANCE)?, Some("i1".into()));
    fake.failing.set(false);
    tracer.finish(&mut state);
    assert_eq!(state.get(INSTANCE)?, None);
    let last = fake.calls.borrow().last().map(|(args, _)| args.clone());
    assert_eq!(
        last.as_deref().map(|args| &args[..5]),
        Some(&["agent_instances", "finish", "i1", "--status", "complete"].map(String::from)[..])
    );
    Ok(())
}

#[test]
fn unconfigured_tracer_is_silent() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer: Prefactor<&Fake, Fixed> = Prefactor::new(&fake, Fixed, None, None);
    tracer.record(&mut state, &[record(1, completed()?)]);
    tracer.finish(&mut state);
    assert!(fake.calls.borrow().is_empty());
    assert!(state.0.is_empty());
    Ok(())
}

mod config;
mod quality;
