//! Span type schemas derived from the event enum's JSON schema: one entry
//! per `kind`, keyed by that kind, so Prefactor knows every span type the
//! ledger can produce.

use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

pub(super) struct Schemas {
    /// Span type name to params schema.
    pub params: Value,
    /// Span type name to result schema.
    pub results: Value,
}

fn open_object() -> Value {
    json!({ "type": "object", "additionalProperties": true })
}

/// Each kind's own object schema, with the tag removed, extra fields
/// allowed (sequence, actor, prompt text), and the definitions it refers
/// to carried along so the schema stands alone.
pub(super) fn rich(schema: Option<&str>) -> Option<Schemas> {
    let root: Value = serde_json::from_str(schema?).ok()?;
    let defs = root.get("$defs").and_then(Value::as_object);
    let mut params = Map::new();
    let mut results = Map::new();
    for variant in root.get("oneOf")?.as_array()? {
        let kind = kind_of(variant)?;
        params.insert(kind.clone(), variant_schema(variant, defs));
        results.insert(kind, open_object());
    }
    Some(Schemas {
        params: Value::Object(params),
        results: Value::Object(results),
    })
}

/// Every kind declared as an open object.
pub(super) fn permissive(schema: Option<&str>) -> Option<Schemas> {
    let root: Value = serde_json::from_str(schema?).ok()?;
    let mut params = Map::new();
    let mut results = Map::new();
    for variant in root.get("oneOf")?.as_array()? {
        let kind = kind_of(variant)?;
        params.insert(kind.clone(), open_object());
        results.insert(kind, open_object());
    }
    Some(Schemas {
        params: Value::Object(params),
        results: Value::Object(results),
    })
}

fn kind_of(variant: &Value) -> Option<String> {
    variant
        .pointer("/properties/kind/const")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn variant_schema(variant: &Value, defs: Option<&Map<String, Value>>) -> Value {
    let mut schema = variant.clone();
    if let Some(properties) = schema
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        properties.remove("kind");
    }
    if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|name| name != "kind");
    }
    schema["additionalProperties"] = json!(true);
    if let Some(defs) = defs {
        let used = closure(&schema, defs);
        if !used.is_empty() {
            let kept: Map<String, Value> = defs
                .iter()
                .filter(|(name, _)| used.contains(*name))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            schema["$defs"] = Value::Object(kept);
        }
    }
    schema
}

/// Names of every definition reachable from `value` through `$ref`.
fn closure(value: &Value, defs: &Map<String, Value>) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    let mut queue: Vec<String> = refs(value);
    while let Some(name) = queue.pop() {
        if !used.insert(name.clone()) {
            continue;
        }
        if let Some(definition) = defs.get(&name) {
            queue.extend(refs(definition));
        }
    }
    used
}

fn refs(value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_refs(value, &mut found);
    found
}

fn collect_refs(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(name) = fields
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|reference| reference.strip_prefix("#/$defs/"))
            {
                found.push(name.to_owned());
            }
            fields.values().for_each(|field| collect_refs(field, found));
        }
        Value::Array(items) => items.iter().for_each(|item| collect_refs(item, found)),
        _ => {}
    }
}
