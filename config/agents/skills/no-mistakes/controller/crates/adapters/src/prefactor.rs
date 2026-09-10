//! Mirrors the event ledger to Prefactor through the `prefactor` CLI.
//!
//! One controller run is one agent instance. Every committed event becomes
//! one span named after the event kind and carrying the event body with its
//! sequence and actor. Paired events (a launch, a GitHub operation, a Jira
//! status change) open a span and finish it. Records that cannot be sent wait
//! in `TraceState` and are replayed, in order, on the next commit. Nothing
//! here can fail or block a command: the CLI reads `PREFACTOR_API_TOKEN`
//! itself, and tracing is on only when that token, `PREFACTOR_AGENT_ID`, and
//! `PREFACTOR_AGENT_IDENTIFIER` are all set.

use crate::{Process, ProcessRequest, success};
use domain::{
    Instant,
    event::EventRecord,
    ports::{Clock, PortError, TraceState, Tracer},
};
use serde_json::{Value, json};
use spans::Pairing;
use std::{cell::Cell, collections::BTreeMap, path::PathBuf};

mod schema;
mod spans;
#[cfg(test)]
mod tests;

const INSTANCE: &str = "instance";
const PENDING: &str = "pending";
const AGENT_NAME: &str = "no-mistakes";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracing {
    pub agent_id: String,
    pub agent_identifier: String,
    /// `PREFACTOR_CAPTURE_INPUTS`: attach the launch prompt text.
    pub capture_inputs: bool,
    /// `PREFACTOR_CAPTURE_OUTPUTS`: keep the launch output text.
    pub capture_outputs: bool,
}

impl Tracing {
    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let present = |key: &str| lookup(key).filter(|value| !value.trim().is_empty());
        present("PREFACTOR_API_TOKEN")?;
        Some(Self {
            agent_id: present("PREFACTOR_AGENT_ID")?,
            agent_identifier: present("PREFACTOR_AGENT_IDENTIFIER")?,
            capture_inputs: enabled(lookup("PREFACTOR_CAPTURE_INPUTS")),
            capture_outputs: enabled(lookup("PREFACTOR_CAPTURE_OUTPUTS")),
        })
    }
}

fn enabled(value: Option<String>) -> bool {
    !value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        )
    })
}

pub struct Prefactor<P, C> {
    process: P,
    clock: C,
    tracing: Option<Tracing>,
    /// The event enum's JSON schema, from which each span type's schema is
    /// derived when the instance is registered.
    event_schema: Option<String>,
    files: Cell<u64>,
}

impl<P, C> Prefactor<P, C> {
    pub fn new(process: P, clock: C, tracing: Option<Tracing>, event_schema: Option<String>) -> Self {
        Self {
            process,
            clock,
            tracing,
            event_schema,
            files: Cell::new(0),
        }
    }
}

impl<P: Process, C: Clock> Tracer for Prefactor<P, C> {
    fn record(&self, state: &mut dyn TraceState, records: &[EventRecord]) {
        let Some(tracing) = &self.tracing else {
            return;
        };
        let mut pending = load_pending(state);
        pending.extend(records.iter().cloned());
        if let Some(instance) = self.instance(state, tracing, &pending) {
            while let Some(record) = pending.first() {
                if self.send(state, &instance, tracing, record).is_err() {
                    break;
                }
                pending.remove(0);
            }
        }
        save_pending(state, &pending);
    }

    fn finish(&self, state: &mut dyn TraceState) {
        if self.tracing.is_none() {
            return;
        }
        self.record(state, &[]);
        if !load_pending(state).is_empty() {
            return;
        }
        let Ok(Some(instance)) = state.get(INSTANCE) else {
            return;
        };
        let finished = self.cli(vec![
            "agent_instances".into(),
            "finish".into(),
            instance,
            "--status".into(),
            "complete".into(),
            "--timestamp".into(),
            rfc3339(self.clock.now()),
        ]);
        if finished.is_ok() {
            let _ = state.remove(INSTANCE);
        }
    }
}

impl<P: Process, C: Clock> Prefactor<P, C> {
    fn cli(&self, args: Vec<String>) -> Result<Value, PortError> {
        let request = ProcessRequest {
            program: "prefactor".into(),
            args,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
            timeout_seconds: 60,
        };
        let output = success(self.process.run(&request)?)?;
        serde_json::from_str(&output).map_err(|error| PortError(error.to_string()))
    }

    /// Runs `call` with an `@file` argument holding `value`, then removes
    /// the file. Payloads go through a file because a single argv entry
    /// has a hard size limit.
    fn with_file<T>(
        &self,
        value: &Value,
        call: impl FnOnce(String) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        let number = self.files.get();
        self.files.set(number.wrapping_add(1));
        let path = std::env::temp_dir().join(format!(
            "no-mistakes-trace-{}-{number}.json",
            std::process::id()
        ));
        std::fs::write(&path, value.to_string()).map_err(|error| PortError(error.to_string()))?;
        let result = call(format!("@{}", path.display()));
        let _ = std::fs::remove_file(&path);
        result
    }

    fn instance(
        &self,
        state: &mut dyn TraceState,
        tracing: &Tracing,
        pending: &[EventRecord],
    ) -> Option<String> {
        if let Ok(Some(id)) = state.get(INSTANCE) {
            return Some(id);
        }
        let at = pending
            .first()
            .map_or_else(|| self.clock.now(), |record| record.at);
        let id = self.register(tracing)?;
        self.cli(vec![
            "agent_instances".into(),
            "start".into(),
            id.clone(),
            "--timestamp".into(),
            rfc3339(at),
        ])
        .ok()?;
        state.set(INSTANCE, &id).ok()?;
        Some(id)
    }

    /// Declares one span type per event kind. The rich schemas carry each
    /// kind's field definitions; if Prefactor rejects them, permissive
    /// objects keep every kind declared.
    fn register(&self, tracing: &Tracing) -> Option<String> {
        let schema = self.event_schema.as_deref();
        let rich = schema::rich(schema).and_then(|schemas| self.register_with(tracing, &schemas));
        rich.or_else(|| {
            schema::permissive(schema).and_then(|schemas| self.register_with(tracing, &schemas))
        })
    }

    fn register_with(&self, tracing: &Tracing, schemas: &schema::Schemas) -> Option<String> {
        let registered = self
            .with_file(&schemas.params, |params| {
                self.with_file(&schemas.results, |results| {
                    self.cli(vec![
                        "agent_instances".into(),
                        "register".into(),
                        "--agent_id".into(),
                        tracing.agent_id.clone(),
                        "--agent_version_name".into(),
                        AGENT_NAME.into(),
                        "--agent_version_external_identifier".into(),
                        tracing.agent_identifier.clone(),
                        "--agent_schema_version_external_identifier".into(),
                        tracing.agent_identifier.clone(),
                        "--span_schemas".into(),
                        params,
                        "--span_result_schemas".into(),
                        results,
                    ])
                })
            })
            .ok()?;
        Some(registered.pointer("/details/id")?.as_str()?.to_owned())
    }

    fn send(
        &self,
        state: &mut dyn TraceState,
        instance: &str,
        tracing: &Tracing,
        record: &EventRecord,
    ) -> Result<(), PortError> {
        let span = spans::describe(record, tracing)?;
        match span.pairing {
            Pairing::Open(key) => {
                let id = self.create_span(instance, &span.kind, &span.payload, None, record.at)?;
                state.set(&key, &id)
            }
            Pairing::Close { key, status, result } => match state.get(&key)? {
                Some(id) => {
                    self.finish_span(&id, status, &result, record.at)?;
                    state.remove(&key)
                }
                None => self
                    .create_span(instance, &span.kind, &span.payload, Some(record.at), record.at)
                    .map(|_| ()),
            },
            Pairing::None => self
                .create_span(instance, &span.kind, &span.payload, Some(record.at), record.at)
                .map(|_| ()),
        }
    }

    /// One bulk item per call. The bulk endpoint is the one CLI path that
    /// accepts `sensitive_encoding`, so marked prompt and output values
    /// are stored apart and redacted by default.
    fn bulk(&self, item: Value) -> Result<Value, PortError> {
        let key = format!(
            "nm-{}-{}",
            self.files.get(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        );
        let mut item = item;
        item["idempotency_key"] = json!(key);
        let response = self.with_file(&json!([item]), |file| {
            self.cli(vec!["bulk".into(), "execute".into(), "--items".into(), file])
        })?;
        let output = response
            .pointer(&format!("/outputs/{key}"))
            .ok_or_else(|| PortError("bulk response omitted the item".into()))?;
        if output["status"] != "success" {
            return Err(PortError(format!("bulk item failed: {output}")));
        }
        Ok(output.clone())
    }

    fn create_span(
        &self,
        instance: &str,
        kind: &str,
        payload: &Value,
        finished: Option<Instant>,
        started: Instant,
    ) -> Result<String, PortError> {
        let mut details = json!({
            "agent_instance_id": instance,
            "schema_name": kind,
            "status": if finished.is_some() { "complete" } else { "active" },
            "sensitive_encoding": true,
            "started_at": rfc3339(started),
            "payload": payload,
        });
        if let Some(finished) = finished {
            details["finished_at"] = json!(rfc3339(finished));
        }
        let created = self.bulk(json!({
            "method": "POST",
            "path": "/api/v1/agent_spans",
            "_type": "agent_spans/create",
            "details": details,
        }))?;
        created
            .pointer("/details/id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| PortError("span create returned no id".into()))
    }

    fn finish_span(
        &self,
        span: &str,
        status: &str,
        result: &Value,
        at: Instant,
    ) -> Result<(), PortError> {
        self.bulk(json!({
            "method": "POST",
            "path": format!("/api/v1/agent_spans/{span}/finish"),
            "_type": "agent_spans/finish",
            "agent_span_id": span,
            "details": {
                "status": status,
                "sensitive_encoding": true,
                "timestamp": rfc3339(at),
                "result_payload": result,
            },
        }))
        .map(|_| ())
    }
}

fn load_pending(state: &dyn TraceState) -> Vec<EventRecord> {
    state
        .get(PENDING)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_pending(state: &mut dyn TraceState, pending: &[EventRecord]) {
    if pending.is_empty() {
        let _ = state.remove(PENDING);
    } else if let Ok(text) = serde_json::to_string(pending) {
        let _ = state.set(PENDING, &text);
    }
}

/// Formats Unix seconds as UTC RFC 3339 (civil-from-days, no dependency).
pub(crate) fn rfc3339(at: Instant) -> String {
    let days = at / 86_400;
    let seconds = at % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}
