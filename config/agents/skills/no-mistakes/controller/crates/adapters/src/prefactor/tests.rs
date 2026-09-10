use super::*;
use crate::ProcessOutput;
use domain::{
    command::AgentRole,
    event::{Actor, Event, Launch, LaunchOutcome},
    ids::{DeliveryId, InvalidId, LaunchId, TaskId},
    ports::Tokens,
};
use serde_json::json;
use std::{cell::RefCell, collections::BTreeMap, str::FromStr};

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

    fn groups(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .map(|(args, _)| format!("{} {}", args[0], args[1]))
            .collect()
    }

    fn item(&self, index: usize) -> Value {
        self.calls.borrow()[index].1[0][0].clone()
    }
}

impl Process for Fake {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        let files = request
            .args
            .iter()
            .filter_map(|arg| arg.strip_prefix('@'))
            .map(|path| serde_json::from_str(&std::fs::read_to_string(path)?))
            .collect::<Result<Vec<Value>, _>>()
            .map_err(|error| PortError(error.to_string()))?;
        let key = files
            .first()
            .and_then(|items| items[0]["idempotency_key"].as_str())
            .unwrap_or("none")
            .to_owned();
        self.calls
            .borrow_mut()
            .push((request.args.clone(), files));
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

const SCHEMA: &str = r#"{"oneOf":[
  {"type":"object","properties":{"kind":{"const":"launch-started","type":"string"},"launch":{"$ref":"#/$defs/Launch"}},"required":["kind","launch"]},
  {"type":"object","properties":{"kind":{"const":"completed","type":"string"},"task":{"$ref":"#/$defs/TaskId"}},"required":["kind","task"]}
],"$defs":{"Launch":{"type":"object","properties":{"task":{"$ref":"#/$defs/TaskId"}}},"TaskId":{"type":"string"},"Unused":{"type":"integer"}}}"#;

fn tracing() -> Tracing {
    Tracing {
        agent_id: "a1".into(),
        agent_identifier: "1.0.0".into(),
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
        ["agent_instances register", "agent_instances start", "bulk execute"]
    );
    let calls = fake.calls.borrow();
    let params = &calls[0].1[0];
    assert!(params["launch-started"]["properties"]["launch"].is_object());
    assert!(params["launch-started"]["properties"].get("kind").is_none());
    assert!(params["launch-started"]["$defs"].get("TaskId").is_some());
    assert!(params["launch-started"]["$defs"].get("Unused").is_none());
    assert_eq!(calls[1].0[2..], ["i1", "--timestamp", "1970-01-01T00:00:51Z"]);
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
    assert_eq!(open["details"]["payload"]["prompt_text"]["$sensitive"], "string");
    assert_eq!(open["details"]["payload"]["prompt_text"]["value"], "do the thing");
    assert_eq!(state.get("span/launch/7")?, Some("s1".into()));
    tracer.record(&mut state, &[record(2, ended("done"))]);
    let close = fake.item(3);
    assert_eq!(close["_type"], "agent_spans/finish");
    assert_eq!(close["agent_span_id"], "s1");
    assert_eq!(close["details"]["status"], "complete");
    assert_eq!(close["details"]["timestamp"], "1970-01-01T00:00:52Z");
    let result = &close["details"]["result_payload"];
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
    tracer.record(&mut state, &[record(2, completed()?), record(3, completed()?)]);
    let pending: Vec<EventRecord> = serde_json::from_str(&state.get(PENDING)?.unwrap_or_default())?;
    assert_eq!(pending.len(), 2);
    fake.failing.set(false);
    tracer.record(&mut state, &[record(4, completed()?)]);
    let sequences: Vec<u64> = (3..6)
        .map(|index| fake.item(index)["details"]["payload"]["sequence"].as_u64())
        .map(Option::unwrap_or_default)
        .collect();
    assert_eq!(sequences, [2, 3, 4]);
    assert!(state.get(PENDING)?.is_none());
    Ok(())
}

#[test]
fn finish_closes_the_instance_only_when_nothing_is_pending() -> Result<(), Box<dyn std::error::Error>> {
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

#[test]
fn capture_flags_drop_prompt_and_output() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let prompt = directory.path().join("prompt.md");
    std::fs::write(&prompt, "secret")?;
    let fake = Fake::new();
    let mut state = Memory::default();
    let quiet = Tracing {
        capture_inputs: false,
        capture_outputs: false,
        ..tracing()
    };
    let tracer = Prefactor::new(&fake, Fixed, Some(quiet), Some(SCHEMA.into()));
    tracer.record(&mut state, &[record(1, started(&prompt)?), record(2, ended("out"))]);
    assert!(fake.item(2)["details"]["payload"].get("prompt_text").is_none());
    let result = &fake.item(3)["details"]["result_payload"];
    assert!(result["result"]["completed"].get("output").is_none());
    assert_eq!(result["result"]["completed"]["session"], "s");
    Ok(())
}

#[test]
fn long_text_is_capped() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let long = "x".repeat(spans::MAX_TEXT + 10);
    tracer(&fake).record(&mut state, &[record(1, ended(&long))]);
    let output = fake.item(2)["details"]["payload"]["result"]["completed"]["output"]["value"].clone();
    let text = output.as_str().unwrap_or_default();
    assert!(text.ends_with("[truncated]"));
    assert!(text.chars().count() < spans::MAX_TEXT + 20);
    Ok(())
}

#[test]
fn tracing_reads_identity_and_capture_flags() {
    let complete = |key: &str| match key {
        "PREFACTOR_API_TOKEN" => Some("t".to_owned()),
        "PREFACTOR_AGENT_ID" => Some("a1".to_owned()),
        "PREFACTOR_AGENT_IDENTIFIER" => Some("1.0.0".to_owned()),
        "PREFACTOR_CAPTURE_OUTPUTS" => Some("false".to_owned()),
        _ => None,
    };
    let tracing = Tracing::from_lookup(complete);
    assert_eq!(tracing.as_ref().map(|value| value.agent_id.as_str()), Some("a1"));
    assert_eq!(tracing.as_ref().map(|value| value.capture_inputs), Some(true));
    assert_eq!(tracing.as_ref().map(|value| value.capture_outputs), Some(false));
    let no_token = |key: &str| (key != "PREFACTOR_API_TOKEN").then(|| complete(key)).flatten();
    assert_eq!(Tracing::from_lookup(no_token), None);
}

#[test]
fn rfc3339_formats_utc() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339(1_757_415_393), "2025-09-09T10:56:33Z");
}
