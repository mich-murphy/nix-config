//! One committed event, one span. The event kind names the span type; the
//! event body, sequence, and actor are the payload. Kinds that begin
//! something open a span under a state key; the kind that ends it finishes
//! that span with its own body as the result.

use super::Tracing;
use domain::{event::EventRecord, ports::PortError};
use serde_json::{Map, Value, json};

/// Longest string kept in any payload field. Roughly 25k tokens, so a
/// full launch prompt or review report survives while a runaway output
/// cannot swamp the trace.
pub(super) const MAX_TEXT: usize = 100_000;
const TRUNCATED: &str = "…[truncated]";

pub(super) struct Span {
    pub kind: String,
    pub payload: Value,
    pub pairing: Pairing,
}

pub(super) enum Pairing {
    None,
    Open(String),
    Close {
        key: String,
        status: &'static str,
        result: Value,
    },
}

pub(super) fn describe(record: &EventRecord, tracing: &Tracing) -> Result<Span, PortError> {
    let mut body =
        serde_json::to_value(&record.event).map_err(|error| PortError(error.to_string()))?;
    let fields = body
        .as_object_mut()
        .ok_or_else(|| PortError("event is not an object".into()))?;
    let kind = fields
        .remove("kind")
        .and_then(|kind| kind.as_str().map(ToOwned::to_owned))
        .ok_or_else(|| PortError("event has no kind".into()))?;
    fields.insert("sequence".into(), json!(record.sequence));
    fields.insert("actor".into(), json!(record.actor));
    enrich(&kind, fields, tracing);
    cap_strings(&mut body);
    let pairing = pairing(&kind, &body);
    Ok(Span {
        kind,
        payload: body,
        pairing,
    })
}

/// The prompt and the output are the two fields that carry repository and
/// Jira content, so both are sent as Prefactor sensitive values: stored
/// apart, redacted by default, discardable later.
fn enrich(kind: &str, fields: &mut Map<String, Value>, tracing: &Tracing) {
    if kind == "launch-started" && tracing.capture_inputs {
        let text = fields
            .get("launch")
            .and_then(|launch| launch.get("prompt"))
            .and_then(Value::as_str)
            .and_then(|path| std::fs::read_to_string(path).ok());
        if let Some(text) = text {
            fields.insert("prompt_text".into(), sensitive(text));
        }
    }
    if kind == "launch-ended" {
        mark_output(fields, tracing.capture_outputs);
    }
}

fn mark_output(fields: &mut Map<String, Value>, capture: bool) {
    let completed = fields
        .get_mut("result")
        .and_then(|result| result.get_mut("completed"))
        .and_then(Value::as_object_mut);
    let Some(completed) = completed else {
        return;
    };
    let Some(Value::String(output)) = completed.remove("output") else {
        return;
    };
    if capture {
        completed.insert("output".into(), sensitive(output));
    }
}

pub(super) fn sensitive(text: String) -> Value {
    json!({
        "$sensitive": "string",
        "labels": ["organisational_confidential"],
        "value": text,
    })
}

fn pairing(kind: &str, body: &Value) -> Pairing {
    match kind {
        "launch-started" => Pairing::Open(key("launch", &body["launch"]["id"])),
        "launch-ended" => Pairing::Close {
            key: key("launch", &body["launch"]),
            status: outcome_status(&body["result"]),
            result: body.clone(),
        },
        "operation-started" => Pairing::Open(key("operation", &body["operation"]["id"])),
        "operation-settled" => Pairing::Close {
            key: key("operation", &body["operation"]),
            status: if body["status"] == "failed" {
                "failed"
            } else {
                "complete"
            },
            result: body.clone(),
        },
        "status-intended" => Pairing::Open(key("status", &body["task"])),
        "status-observed" => Pairing::Close {
            key: key("status", &body["task"]),
            status: "complete",
            result: body.clone(),
        },
        _ => Pairing::None,
    }
}

fn key(group: &str, id: &Value) -> String {
    let id = id
        .as_str()
        .map_or_else(|| id.to_string(), ToOwned::to_owned);
    format!("span/{group}/{id}")
}

/// `LaunchOutcome` is externally tagged: an object under `completed` or
/// `failed`, or the bare string `cancelled`.
fn outcome_status(result: &Value) -> &'static str {
    if result == "cancelled" {
        "cancelled"
    } else if result.get("failed").is_some() {
        "failed"
    } else {
        "complete"
    }
}

fn cap_strings(value: &mut Value) {
    match value {
        Value::String(text) => {
            if text.chars().count() > MAX_TEXT {
                let mut kept: String = text.chars().take(MAX_TEXT).collect();
                kept.push_str(TRUNCATED);
                *text = kept;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(cap_strings),
        Value::Object(fields) => fields.values_mut().for_each(cap_strings),
        _ => {}
    }
}
