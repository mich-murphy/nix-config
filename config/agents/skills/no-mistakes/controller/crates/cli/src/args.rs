use app::{AgentError, Rejection};
use domain::command::Command;
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

const COMMANDS: &[&str] = &[
    "init",
    "status",
    "next",
    "usage-report",
    "discover",
    "refresh",
    "claim",
    "hold",
    "resume",
    "brief",
    "plan",
    "checkpoint",
    "snapshot",
    "subtask-record",
    "escalate-tier",
    "bind-slot",
    "cleanup",
    "record-proof",
    "run-check",
    "run-agent",
    "review-schema",
    "validate-review",
    "disposition",
    "human-review",
    "lesson-record",
    "publish",
    "observe-pr",
    "poll-checks",
    "final-verify",
    "complete",
    "set-status",
    "observe-status",
    "grant",
    "open-delivery",
    "narrow-acceptance",
    "recover-operation",
];

pub(super) struct Args {
    pub(super) command: String,
    pub(super) run: PathBuf,
    pub(super) input: Option<String>,
    pub(super) check: bool,
    pub(super) pretty: bool,
    pub(super) values: BTreeMap<String, String>,
    pub(super) positionals: Vec<String>,
}

pub(super) fn command(args: &Args) -> Result<Command, AgentError> {
    let mut value = if let Some(input) = &args.input {
        if !args.positionals.is_empty() || !args.values.is_empty() {
            return Err(invalid(
                &args.command,
                "--input cannot be combined with command fields",
            ));
        }
        read_json(input, &args.command)?
    } else {
        scalar(args)?
    };
    let object = value
        .as_object_mut()
        .ok_or_else(|| invalid(&args.command, "request must be an object"))?;
    if object
        .insert("command".into(), Value::String(args.command.clone()))
        .is_some()
    {
        return Err(invalid(&args.command, "command belongs on argv"));
    }
    serde_json::from_value(value).map_err(|error| invalid(&args.command, &error.to_string()))
}

fn scalar(args: &Args) -> Result<Value, AgentError> {
    let mut value = Map::new();
    match args.command.as_str() {
        "status" | "next" | "usage-report" | "review-schema" => {
            no_positionals(args)?;
            exact_values(args, &[])?;
        }
        "claim" | "snapshot" | "complete" | "resume" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            exact_positionals(args, 1)?;
            exact_values(args, &[])?;
        }
        "cleanup" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            value.insert("delete".into(), Value::Bool(flag(args, "delete")));
            exact_positionals(args, 1)?;
            exact_values(args, &["delete"])?;
        }
        "observe-pr" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            value.insert(
                "pr".into(),
                number(required_value(args, "pr")?, &args.command)?,
            );
            exact_positionals(args, 1)?;
            exact_values(args, &["pr"])?;
        }
        "poll-checks" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            value.insert("wait".into(), Value::Bool(flag(args, "wait")));
            exact_positionals(args, 1)?;
            exact_values(args, &["wait"])?;
        }
        "recover-operation" => {
            let kind = required_value(args, "kind")?;
            let id = number(positional(args, 0, kind)?, &args.command)?;
            value.insert(
                "target".into(),
                Value::Object(Map::from_iter([
                    ("kind".into(), Value::String(kind.into())),
                    (kind.into(), id),
                ])),
            );
            value.insert("terminate".into(), Value::Bool(flag(args, "terminate")));
            exact_positionals(args, 1)?;
            exact_values(args, &["terminate", "kind"])?;
        }
        "bind-slot" => {
            task(args, &mut value)?;
            value.insert(
                "slot".into(),
                Value::String(required_value(args, "slot")?.into()),
            );
            value.insert(
                "branch".into(),
                Value::String(required_value(args, "branch")?.into()),
            );
            if let Some(authority) = args.values.get("authority") {
                value.insert("authority".into(), number(authority, &args.command)?);
            }
            exact_values(args, &["slot", "branch", "authority"])?;
        }
        "escalate-tier" => {
            task(args, &mut value)?;
            value.insert(
                "to".into(),
                Value::String(required_value(args, "to")?.into()),
            );
            value.insert(
                "reason".into(),
                Value::String(required_value(args, "reason")?.into()),
            );
            exact_values(args, &["to", "reason"])?;
        }
        "final-verify" => {
            task(args, &mut value)?;
            value.insert(
                "commit".into(),
                Value::String(required_value(args, "commit")?.into()),
            );
            value.insert(
                "evidence".into(),
                Value::String(required_value(args, "evidence")?.into()),
            );
            exact_values(args, &["commit", "evidence"])?;
        }
        _ => {
            return Err(invalid(
                &args.command,
                "this command needs --input <file|->",
            ));
        }
    }
    Ok(Value::Object(value))
}

mod parse;
use parse::parse_raw;

pub(super) fn parse_args() -> Result<Args, AgentError> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let command = command_name(&raw)?;
    parse_raw(&raw, command)
}

fn command_name(raw: &[String]) -> Result<String, AgentError> {
    let command = raw
        .first()
        .cloned()
        .ok_or_else(|| invalid("controller", "command is required"))?;
    if COMMANDS.contains(&command.as_str()) {
        Ok(command)
    } else {
        Err(invalid(&command, "unknown command"))
    }
}

fn read_json(source: &str, command: &str) -> Result<Value, AgentError> {
    let mut text = String::new();
    if source == "-" {
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| invalid(command, &error.to_string()))?;
    } else {
        text = fs::read_to_string(source).map_err(|error| invalid(command, &error.to_string()))?;
    }
    serde_json::from_str(&text).map_err(|error| invalid(command, &error.to_string()))
}

pub(super) fn parse_toml<T: serde::de::DeserializeOwned>(
    path: &Path,
    name: &str,
) -> Result<T, AgentError> {
    let text = fs::read_to_string(path)
        .map_err(|error| invalid("init", &format!("read {name}: {error}")))?;
    toml::from_str(&text).map_err(|error| invalid("init", &format!("parse {name}: {error}")))
}

fn positional<'a>(args: &'a Args, index: usize, name: &str) -> Result<&'a str, AgentError> {
    args.positionals
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| invalid(&args.command, &format!("{name} is required")))
}

fn exact_positionals(args: &Args, count: usize) -> Result<(), AgentError> {
    if args.positionals.len() == count {
        Ok(())
    } else {
        Err(invalid(&args.command, "unexpected positional argument"))
    }
}

fn no_positionals(args: &Args) -> Result<(), AgentError> {
    exact_positionals(args, 0)
}

fn exact_values(args: &Args, allowed: &[&str]) -> Result<(), AgentError> {
    if args
        .values
        .keys()
        .all(|value| allowed.contains(&value.as_str()))
    {
        Ok(())
    } else {
        Err(invalid(&args.command, "unknown option"))
    }
}

fn task(args: &Args, value: &mut Map<String, Value>) -> Result<(), AgentError> {
    value.insert(
        "task".into(),
        Value::String(positional(args, 0, "task")?.into()),
    );
    exact_positionals(args, 1)
}

pub(super) fn validate_init(args: &Args) -> Result<(), AgentError> {
    no_positionals(args)?;
    exact_values(args, &["config", "profile"])?;
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

fn flag(args: &Args, name: &str) -> bool {
    args.values.get(name).is_some_and(|value| value == "true")
}

fn number(value: &str, command: &str) -> Result<Value, AgentError> {
    value
        .parse::<u64>()
        .map(Value::from)
        .map_err(|error| invalid(command, &error.to_string()))
}

pub(super) fn print_help(command: Option<&str>) {
    if let Some(command) = command.filter(|value| COMMANDS.contains(value)) {
        println!("controller {command} --run <directory> [--input <file|->] [--check] [--pretty]");
    } else {
        println!(
            "controller <command> --run <directory> [options]\n\nCommands:\n  {}",
            COMMANDS.join("\n  ")
        );
    }
}

#[cfg(test)]
mod tests;

pub(super) fn invalid(command: &str, message: &str) -> AgentError {
    AgentError {
        failed: command.into(),
        phase: None,
        why: Rejection::Invalid(message.into()),
        next: None,
    }
}
