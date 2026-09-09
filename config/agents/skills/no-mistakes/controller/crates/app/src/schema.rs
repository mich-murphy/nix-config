//! The real `Command` schema, and everything derived from it: help,
//! argv dispatch, and `next`'s own schema field. `schemars::schema_for!`
//! is called here (and in `review-schema`), never in `domain`, because
//! turning its output into a per-command schema and a scalar/non-scalar
//! field table means walking `serde_json::Value`, which `domain` must
//! stay free of.
use domain::command::{ActionEnvelope, Command, schema::Template};
use schemars::Schema;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Every `Command` variant's tag name, in the enum's declaration order.
#[must_use]
pub fn command_names() -> Vec<String> {
    variants().map(|(name, _)| name).collect()
}

/// The real JSON schema for the `Command` variant tagged `name`: the
/// matching member of `schema_for!(Command)`'s `oneOf`, given its own
/// copy of the shared `$defs` so it resolves as a standalone document.
#[must_use]
pub fn command_schema(name: &str) -> Option<Schema> {
    variants()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, schema)| schema)
}

fn variants() -> impl Iterator<Item = (String, Schema)> {
    let root = schemars::schema_for!(Command);
    let defs = root.get("$defs").cloned();
    let items: Vec<Value> = root
        .get("oneOf")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    items.into_iter().filter_map(move |variant| {
        let name = variant
            .pointer("/properties/command/const")
            .and_then(Value::as_str)?
            .to_owned();
        let mut object = variant.as_object()?.clone();
        if let Some(defs) = defs.as_ref().and_then(Value::as_object) {
            object.insert("$defs".to_owned(), referenced_defs(&variant, defs));
        }
        Some((name, Schema::from(object)))
    })
}

/// Only the `$defs` `variant` reaches, directly or through other
/// definitions. The root schema's `$defs` covers every command, and
/// attaching all of it to each `next` result and each rejection would
/// cost the coordinator the same tokens on every call (design Section
/// 11) for definitions the command never uses.
fn referenced_defs(variant: &Value, defs: &serde_json::Map<String, Value>) -> Value {
    let mut pending: Vec<String> = Vec::new();
    collect_refs(variant, &mut pending);
    let mut kept = serde_json::Map::new();
    while let Some(name) = pending.pop() {
        if kept.contains_key(&name) {
            continue;
        }
        if let Some(definition) = defs.get(&name) {
            collect_refs(definition, &mut pending);
            kept.insert(name, definition.clone());
        }
    }
    Value::Object(kept)
}

fn collect_refs(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|reference| reference.strip_prefix("#/$defs/"))
            {
                found.push(name.to_owned());
            }
            object.values().for_each(|child| collect_refs(child, found));
        }
        Value::Array(items) => items.iter().for_each(|child| collect_refs(child, found)),
        _ => {}
    }
}

/// A command field's shape: whether it can be given as `--field value`
/// on argv (a string, integer, boolean, or a newtype/enum whose own
/// schema resolves to one of those) or must come through `--input`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    String,
    Integer,
    Boolean,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub kind: FieldKind,
    pub required: bool,
}

impl Field {
    #[must_use]
    pub const fn scalar(self) -> bool {
        !matches!(self.kind, FieldKind::Other)
    }
}

/// `schema`'s fields, excluding the `command` tag itself: each field's
/// scalar kind (resolving a `$ref`, an `Option<T>`'s `anyOf`-with-null,
/// or a `type` array with `null`) and whether the schema requires it.
#[must_use]
pub fn fields(schema: &Schema) -> BTreeMap<String, Field> {
    let Some(object) = schema.as_object() else {
        return BTreeMap::new();
    };
    let required: Vec<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let empty = serde_json::Map::new();
    let defs = object
        .get("$defs")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    object
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(name, _)| name.as_str() != "command")
        .map(|(name, value)| {
            let field = Field {
                kind: resolve(value, defs),
                required: required.contains(&name.as_str()),
            };
            (name.clone(), field)
        })
        .collect()
}

fn resolve(value: &Value, defs: &serde_json::Map<String, Value>) -> FieldKind {
    if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
        return reference
            .strip_prefix("#/$defs/")
            .and_then(|name| defs.get(name))
            .map_or(FieldKind::Other, |target| resolve(target, defs));
    }
    if let Some(options) = value.get("anyOf").and_then(Value::as_array) {
        return options
            .iter()
            .find(|option| option.get("type").and_then(Value::as_str) != Some("null"))
            .map_or(FieldKind::Other, |option| resolve(option, defs));
    }
    match value.get("type") {
        Some(Value::String(kind)) => scalar_kind(kind),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|kind| *kind != "null")
            .map_or(FieldKind::Other, scalar_kind),
        _ => FieldKind::Other,
    }
}

fn scalar_kind(kind: &str) -> FieldKind {
    match kind {
        "string" => FieldKind::String,
        "integer" => FieldKind::Integer,
        "boolean" => FieldKind::Boolean,
        _ => FieldKind::Other,
    }
}

/// The command name, filled template and real schema `next` (and a
/// rejection's suggested next action) report: `domain::state::next`
/// computes everything but the schema and the filled invocation line,
/// which need `schemars::schema_for!` and JSON introspection this crate
/// has and `domain` does not.
#[derive(Debug, Clone, Serialize)]
pub struct AnnotatedAction {
    pub action: domain::command::NextAction,
    pub command: String,
    pub template: Template,
    pub schema: Schema,
}

/// Builds the full annotated action from `domain`'s pure result: looks
/// up `envelope.name`'s real schema and fills the invocable command
/// line from it and `envelope.template`.
#[must_use]
pub fn annotate(envelope: ActionEnvelope) -> AnnotatedAction {
    let schema = command_schema(&envelope.name).unwrap_or_default();
    let command = command_line(&envelope.name, &envelope.template, &schema);
    AnnotatedAction {
        action: envelope.action,
        command,
        template: envelope.template,
        schema,
    }
}

/// `controller <name> [<task>] --run <run-directory> [--input
/// <request.json>]`: the task is embedded positionally when the schema
/// carries a scalar `task` field and the template has its value;
/// `--input` is appended when the schema requires any field argv
/// cannot carry.
fn command_line(name: &str, template: &Template, schema: &Schema) -> String {
    let info = fields(schema);
    let mut line = format!("controller {name}");
    let task_scalar = info.get("task").is_some_and(|field| field.scalar());
    if task_scalar && let Some(task) = template.values.get("task") {
        line.push(' ');
        line.push_str(task);
    }
    line.push_str(" --run <run-directory>");
    let needs_input = info.values().any(|field| field.required && !field.scalar());
    if needs_input {
        line.push_str(" --input <request.json>");
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_schema_carries_only_referenced_defs() {
        let schema = command_schema("claim").unwrap_or_default();
        let defs = schema
            .get("$defs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(defs.keys().collect::<Vec<_>>(), vec!["TaskId"]);
    }

    #[test]
    fn referenced_defs_follow_nested_references() {
        let schema = command_schema("brief").unwrap_or_default();
        let defs = schema
            .get("$defs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert!(defs.contains_key("Criterion"));
        assert!(defs.contains_key("CriterionId"));
        assert!(!defs.contains_key("Profile"));
    }
}
