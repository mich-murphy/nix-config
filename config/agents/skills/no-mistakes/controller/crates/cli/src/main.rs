use adapters::{
    SystemClock, SystemProcess, codex::Codex, git::Git, github::Gh, pi::Pi, sqlite::Store,
};
use app::{AgentError, App, Rejection, Services, initialize};
use domain::{
    command::{Command, RunConfig},
    ports::Harness,
    risk::{HarnessKind, Profile},
};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::ExitCode,
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

struct Args {
    command: String,
    run: PathBuf,
    input: Option<String>,
    check: bool,
    pretty: bool,
    values: BTreeMap<String, String>,
    positionals: Vec<String>,
}

fn main() -> ExitCode {
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    if raw.is_empty() || raw.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help(raw.first().map(String::as_str));
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok((output, pretty)) => {
            let encoded = if pretty {
                serde_json::to_string_pretty(&output)
            } else {
                serde_json::to_string(&output)
            };
            match encoded {
                Ok(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                Err(error) => fail(internal("output", error.to_string())),
            }
        }
        Err(error) => fail(error),
    }
}

fn run() -> Result<(app::Output, bool), AgentError> {
    let args = parse_args()?;
    if args.command == "init" {
        let output = init(&args)?;
        return Ok((output, args.pretty));
    }
    let store =
        Store::open(&args.run).map_err(|error| internal(&args.command, error.to_string()))?;
    let state = store
        .state()
        .map_err(|error| internal(&args.command, error.to_string()))?;
    let config = state
        .config
        .as_ref()
        .ok_or_else(|| internal(&args.command, "run is not initialized".into()))?;
    let profile = state
        .profile
        .as_ref()
        .ok_or_else(|| internal(&args.command, "profile is missing".into()))?;
    let process = SystemProcess;
    let clock = SystemClock;
    let git = Git::new(process, config.repo.clone());
    let github = Gh::new(process, config.repo.clone(), config.github_repo.clone());
    let harness = harness(profile.harness.kind, process);
    let services = Services {
        vcs: &git,
        github: &github,
        harness: harness.as_ref(),
        process: &process,
        clock: &clock,
    };
    let command = command(&args)?;
    let mut app = App::new(store, services);
    Ok((app.execute(command, args.check)?, args.pretty))
}

fn init(args: &Args) -> Result<app::Output, AgentError> {
    let config_path = required_value(args, "config")?;
    let profile_path = required_value(args, "profile")?;
    let config: RunConfig = parse_toml(Path::new(config_path), "config")?;
    let profile: Profile = parse_toml(Path::new(profile_path), "profile")?;
    let process = SystemProcess;
    let clock = SystemClock;
    let git = Git::new(process, config.repo.clone());
    let github = Gh::new(process, config.repo.clone(), config.github_repo.clone());
    let harness = harness(profile.harness.kind, process);
    initialize(
        &args.run,
        config,
        profile,
        Services {
            vcs: &git,
            github: &github,
            harness: harness.as_ref(),
            process: &process,
            clock: &clock,
        },
    )
}

fn harness(kind: HarnessKind, process: SystemProcess) -> Box<dyn Harness> {
    match kind {
        HarnessKind::Codex => Box::new(Codex::new(process)),
        HarnessKind::Pi => Box::new(Pi::new(process)),
    }
}

fn command(args: &Args) -> Result<Command, AgentError> {
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
        "claim" | "snapshot" | "complete" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            exact_positionals(args, 1)?;
            exact_values(args, &[])?;
        }
        "resume" => {
            value.insert(
                "task".into(),
                Value::String(positional(args, 0, "task")?.into()),
            );
            value.insert(
                "final_revisit".into(),
                Value::Bool(flag(args, "final-revisit")),
            );
            exact_positionals(args, 1)?;
            exact_values(args, &["final-revisit"])?;
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
            value.insert(
                "operation".into(),
                number(positional(args, 0, "operation")?, &args.command)?,
            );
            value.insert("terminate".into(), Value::Bool(flag(args, "terminate")));
            exact_positionals(args, 1)?;
            exact_values(args, &["terminate"])?;
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

fn parse_args() -> Result<Args, AgentError> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let command = raw
        .first()
        .cloned()
        .ok_or_else(|| invalid("controller", "command is required"))?;
    if !COMMANDS.contains(&command.as_str()) {
        return Err(invalid(&command, "unknown command"));
    }
    let mut run = None;
    let mut input = None;
    let mut check = false;
    let mut pretty = false;
    let mut values = BTreeMap::new();
    let mut positionals = Vec::new();
    let mut index = 1;
    while index < raw.len() {
        let arg = &raw[index];
        match arg.as_str() {
            "--check" => check = true,
            "--pretty" => pretty = true,
            "--delete" | "--wait" | "--terminate" | "--final-revisit" => {
                values.insert(arg.trim_start_matches("--").into(), "true".into());
            }
            value if value.starts_with("--") => {
                index += 1;
                let Some(next) = raw.get(index) else {
                    return Err(invalid(&command, &format!("{value} needs a value")));
                };
                match value {
                    "--run" => run = Some(PathBuf::from(next)),
                    "--input" => input = Some(next.clone()),
                    _ => {
                        values.insert(value.trim_start_matches("--").into(), next.clone());
                    }
                }
            }
            _ => positionals.push(arg.clone()),
        }
        index += 1;
    }
    let run = run.ok_or_else(|| invalid(&command, "--run is required"))?;
    Ok(Args {
        command,
        run,
        input,
        check,
        pretty,
        values,
        positionals,
    })
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

fn parse_toml<T: serde::de::DeserializeOwned>(path: &Path, name: &str) -> Result<T, AgentError> {
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

fn required_value<'a>(args: &'a Args, name: &str) -> Result<&'a str, AgentError> {
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

fn print_help(command: Option<&str>) {
    if let Some(command) = command.filter(|value| COMMANDS.contains(value)) {
        println!("controller {command} --run <directory> [--input <file|->] [--check] [--pretty]");
    } else {
        println!(
            "controller <command> --run <directory> [options]\n\nCommands:\n  {}",
            COMMANDS.join("\n  ")
        );
    }
}

fn invalid(command: &str, message: &str) -> AgentError {
    AgentError {
        failed: command.into(),
        phase: None,
        why: Rejection::Invalid(message.into()),
        next: None,
    }
}

fn internal(command: &str, message: String) -> AgentError {
    AgentError {
        failed: command.into(),
        phase: None,
        why: Rejection::Internal(message),
        next: None,
    }
}

fn fail(error: AgentError) -> ExitCode {
    eprintln!(
        "{}",
        serde_json::to_string(&error).unwrap_or_else(|_| "{\"class\":\"internal\"}".into())
    );
    ExitCode::from(error.exit_code())
}
