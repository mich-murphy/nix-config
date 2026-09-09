//! The CLI frame (`clap` derive: `--run`, `--input`, `--check`,
//! `--pretty`) plus schema-driven dispatch: every `Command` variant is a
//! subcommand, its scalar fields (string, integer, boolean, or a newtype
//! over one of those) are `clap`-validated `--field value` flags built
//! from `app::schema`, and anything else must come through `--input`.

use app::{AgentError, Rejection};
use domain::command::Command;
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

mod build;
#[cfg(test)]
mod tests;

pub(super) struct Args {
    pub(super) command: String,
    pub(super) run: PathBuf,
    pub(super) input: Option<String>,
    pub(super) check: bool,
    pub(super) pretty: bool,
    pub(super) values: BTreeMap<String, String>,
}

pub(super) fn parse_args() -> Result<Args, AgentError> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let command = raw
        .first()
        .cloned()
        .ok_or_else(|| invalid("controller", "command is required"))?;
    if !app::schema::command_names()
        .iter()
        .any(|name| name == &command)
    {
        return Err(invalid(&command, "unknown command"));
    }
    build::parse(command, &raw[1..])
}

/// Builds the domain `Command` from `args`: an `--input` payload (a
/// file, or `-` for stdin) if given, overlaid with argv's scalar
/// fields (argv wins), then `deny_unknown_fields` on `Command` itself
/// rejects anything neither side declared.
pub(super) fn command(args: &Args) -> Result<Command, AgentError> {
    let mut object = match &args.input {
        Some(input) => read_json(input, &args.command)?,
        None => Map::new(),
    };
    let schema = app::schema::command_schema(&args.command)
        .ok_or_else(|| invalid(&args.command, "unknown command"))?;
    let fields = app::schema::fields(&schema);
    for (field, value) in &args.values {
        let kind = fields
            .get(field)
            .map_or(app::schema::FieldKind::String, |info| info.kind);
        object.insert(field.clone(), scalar_value(kind, value, &args.command)?);
    }
    if object
        .insert("command".into(), Value::String(args.command.clone()))
        .is_some()
    {
        return Err(invalid(&args.command, "command belongs on argv"));
    }
    serde_json::from_value(Value::Object(object))
        .map_err(|error| invalid(&args.command, &error.to_string()))
}

fn scalar_value(
    kind: app::schema::FieldKind,
    value: &str,
    command: &str,
) -> Result<Value, AgentError> {
    match kind {
        app::schema::FieldKind::Integer => value
            .parse::<u64>()
            .map(Value::from)
            .map_err(|error| invalid(command, &error.to_string())),
        app::schema::FieldKind::Boolean => value
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|error| invalid(command, &error.to_string())),
        app::schema::FieldKind::String | app::schema::FieldKind::Other => {
            Ok(Value::String(value.to_owned()))
        }
    }
}

fn read_json(source: &str, command: &str) -> Result<Map<String, Value>, AgentError> {
    let mut text = String::new();
    if source == "-" {
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| invalid(command, &error.to_string()))?;
    } else {
        text = fs::read_to_string(source).map_err(|error| invalid(command, &error.to_string()))?;
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|error| invalid(command, &error.to_string()))?;
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(invalid(command, "--input must be a JSON object")),
    }
}

pub(super) fn parse_toml<T: serde::de::DeserializeOwned>(
    path: &Path,
    name: &str,
) -> Result<T, AgentError> {
    let text = fs::read_to_string(path)
        .map_err(|error| invalid("init", &format!("read {name}: {error}")))?;
    toml::from_str(&text).map_err(|error| invalid("init", &format!("parse {name}: {error}")))
}

pub(super) fn validate_init(args: &Args) -> Result<(), AgentError> {
    if args.input.is_some() || args.check {
        return Err(invalid(
            "init",
            "init accepts only --config, --profile, --run and --pretty",
        ));
    }
    Ok(())
}

pub(super) fn required_value<'a>(args: &'a Args, name: &str) -> Result<&'a str, AgentError> {
    args.values
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| invalid(&args.command, &format!("--{name} is required")))
}

pub(super) fn print_help(command: Option<&str>) {
    println!("{}", help_text(command));
}

pub(super) fn help_text(command: Option<&str>) -> String {
    match command.filter(|name| app::schema::command_schema(name).is_some()) {
        Some(name) => command_help_text(name),
        None => catalog_help_text(),
    }
}

fn catalog_help_text() -> String {
    let mut text = String::from("controller <command> --run <directory> [options]\n\nCommands:\n");
    for name in app::schema::command_names() {
        text.push_str("  ");
        text.push_str(&name);
        text.push('\n');
    }
    text
}

fn command_help_text(name: &str) -> String {
    let mut text =
        format!("controller {name} --run <directory> [--input <file|-|>] [--check] [--pretty]");
    let Some(schema) = app::schema::command_schema(name) else {
        return text;
    };
    let fields = app::schema::fields(&schema);
    if fields.is_empty() {
        return text;
    }
    text.push_str("\n\nFields (name <type> required/optional):\n");
    for (field, info) in fields {
        text.push_str(&field_help_line(&field, info));
    }
    text
}

fn field_help_line(field: &str, info: app::schema::Field) -> String {
    let required = if info.required {
        "required"
    } else {
        "optional"
    };
    let kind = match info.kind {
        app::schema::FieldKind::String => "string",
        app::schema::FieldKind::Integer => "integer",
        app::schema::FieldKind::Boolean => "boolean",
        app::schema::FieldKind::Other => "object/array, use --input",
    };
    if field == "task" {
        format!("  {field} (first positional) <{kind}> {required}\n")
    } else {
        format!("  --{field} <{kind}> {required}\n")
    }
}

pub(super) fn invalid(command: &str, message: &str) -> AgentError {
    AgentError::new(command, None, Rejection::Invalid(message.into()), None)
}
