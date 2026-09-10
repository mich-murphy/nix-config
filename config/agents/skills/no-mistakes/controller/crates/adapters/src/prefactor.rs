//! Mirrors the event ledger to Prefactor through the `prefactor` CLI.
//!
//! One controller run is one agent instance. Every committed event becomes
//! one span named after the event kind and carrying the event body with its
//! sequence and actor. Paired events (a launch, a GitHub operation, a Jira
//! status change) open a span and finish it. Records that cannot be sent wait
//! in `TraceState` and are replayed, in order, on the next commit. Nothing
//! here can fail or block a command: the CLI reads `PREFACTOR_API_TOKEN`
//! itself, and tracing is on only when that token, `PREFACTOR_AGENT_ID`, and
//! `PREFACTOR_AGENT_IDENTIFIER` are all set. Registration and the quality
//! payload use the HTTP API directly (`http`), because the CLI has no way to
//! declare or record quality schemas.

use crate::{Process, ProcessRequest, success};
use domain::{
    Instant,
    event::EventRecord,
    judge::QualityPayload,
    ports::{Clock, PortError, TraceState, Tracer},
};
use http::Http;
use serde_json::{Value, json};
use spans::Pairing;
use std::{collections::BTreeMap, path::PathBuf};
pub use support::rfc3339;
use support::{PayloadFile, load_pending, next_number, save_pending};

mod http;
mod quality;
mod schema;
mod spans;
mod support;
#[cfg(test)]
mod tests;

const INSTANCE: &str = "instance";
/// The most recently closed instance: a judge that lands after
/// `usage-report` still records its quality payload there.
const LAST_INSTANCE: &str = "last-instance";
const AGENT_NAME: &str = "no-mistakes";
const DEFAULT_API_URL: &str = "https://app.prefactorai.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracing {
    pub agent_id: String,
    pub agent_identifier: String,
    /// `PREFACTOR_API_URL`, for the calls the CLI cannot make.
    pub api_url: String,
    /// `PREFACTOR_API_TOKEN`; the CLI reads it from the environment itself.
    pub token: String,
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
        Some(Self {
            token: present("PREFACTOR_API_TOKEN")?,
            agent_id: present("PREFACTOR_AGENT_ID")?,
            agent_identifier: present("PREFACTOR_AGENT_IDENTIFIER")?,
            api_url: present("PREFACTOR_API_URL").unwrap_or_else(|| DEFAULT_API_URL.into()),
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
}

impl<P, C> Prefactor<P, C> {
    pub fn new(
        process: P,
        clock: C,
        tracing: Option<Tracing>,
        event_schema: Option<String>,
    ) -> Self {
        Self {
            process,
            clock,
            tracing,
            event_schema,
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
            // A failure leaves the rest pending for the next commit.
            let _ = self.drain(state, &instance, tracing, &mut pending);
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
            instance.clone(),
            "--status".into(),
            "complete".into(),
            "--timestamp".into(),
            rfc3339(self.clock.now()),
        ]);
        if finished.is_ok() {
            let _ = state.set(LAST_INSTANCE, &instance);
            let _ = state.remove(INSTANCE);
        }
    }

    fn quality(&self, state: &mut dyn TraceState, payload: &QualityPayload) {
        let Some(tracing) = &self.tracing else {
            return;
        };
        let instance = match state.get(INSTANCE) {
            Ok(Some(id)) => id,
            _ => match state.get(LAST_INSTANCE) {
                Ok(Some(id)) => id,
                _ => return,
            },
        };
        let Ok(body) = serde_json::to_value(payload) else {
            return;
        };
        let _ = Http::new(&self.process, &tracing.api_url, &tracing.token).post(
            &format!("/api/v1/agent_instance/{instance}/record_quality"),
            &json!({ "name": quality::NAME, "payload": body }),
        );
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

    /// Sends pending records in order; the first failure stops the drain
    /// with that record and everything after it still pending.
    fn drain(
        &self,
        state: &mut dyn TraceState,
        instance: &str,
        tracing: &Tracing,
        pending: &mut Vec<EventRecord>,
    ) -> Result<(), PortError> {
        while let Some(record) = pending.first() {
            self.send(state, instance, tracing, record)?;
            pending.remove(0);
        }
        Ok(())
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

    /// Registration goes over HTTP rather than the CLI because only the
    /// endpoint accepts `quality_schemas`, which the quality payload needs.
    fn register_with(&self, tracing: &Tracing, schemas: &schema::Schemas) -> Option<String> {
        let registered = Http::new(&self.process, &tracing.api_url, &tracing.token)
            .post(
                "/api/v1/agent_instance/register",
                &json!({
                    "agent_id": tracing.agent_id,
                    "agent_version": {
                        "name": AGENT_NAME,
                        "external_identifier": tracing.agent_identifier,
                    },
                    "agent_schema_version": {
                        "external_identifier": tracing.agent_identifier,
                        "span_schemas": schemas.params,
                        "span_result_schemas": schemas.results,
                        "quality_schemas": quality::schemas(),
                    },
                }),
            )
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
            Pairing::Close {
                key,
                status,
                result,
            } => match state.get(&key)? {
                Some(id) => {
                    self.finish_span(&id, status, &result, record.at)?;
                    state.remove(&key)
                }
                None => self
                    .create_span(
                        instance,
                        &span.kind,
                        &span.payload,
                        Some(record.at),
                        record.at,
                    )
                    .map(|_| ()),
            },
            Pairing::None => self
                .create_span(
                    instance,
                    &span.kind,
                    &span.payload,
                    Some(record.at),
                    record.at,
                )
                .map(|_| ()),
        }
    }

    /// One bulk item per call. The bulk endpoint is the one CLI path that
    /// accepts `sensitive_encoding`, so marked prompt and output values
    /// are stored apart and redacted by default.
    fn bulk(&self, item: Value) -> Result<Value, PortError> {
        let key = format!(
            "nm-{}-{}",
            next_number(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        );
        let mut item = item;
        item["idempotency_key"] = json!(key);
        let items = PayloadFile::new(&json!([item]))?;
        let response = self.cli(vec![
            "bulk".into(),
            "execute".into(),
            "--items".into(),
            items.arg(),
        ])?;
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

    /// Unlike create, finish takes its fields beside the span id: a
    /// `details` wrapper is accepted but stores no result.
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
            "status": status,
            "sensitive_encoding": true,
            "timestamp": rfc3339(at),
            "result_payload": result,
        }))
        .map(|_| ())
    }
}
