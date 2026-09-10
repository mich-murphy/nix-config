use adapters::{
    SystemClock, SystemProcess,
    codex::Codex,
    git::Git,
    github::Gh,
    pi::Pi,
    prefactor::{Prefactor, Tracing},
    sqlite::Store,
};
use app::{AgentError, App, Rejection, Services, initialize};
use domain::{
    command::RunConfig,
    ports::Harness,
    risk::{HarnessKind, Profile},
};
use std::{path::Path, process::ExitCode};

mod args;

use args::{Args, command, parse_args, parse_toml, print_help, required_value, validate_init};

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
    let tracer = tracer(process);
    let services = Services {
        vcs: &git,
        github: &github,
        harness: harness.as_ref(),
        process: &process,
        clock: &clock,
        tracer: &tracer,
    };
    let command = command(&args)?;
    let mut app = App::new(store, services);
    Ok((app.execute(command, args.check)?, args.pretty))
}

fn init(args: &Args) -> Result<app::Output, AgentError> {
    validate_init(args)?;
    let config_path = required_value(args, "config")?;
    let profile_path = required_value(args, "profile")?;
    let config: RunConfig = parse_toml(Path::new(config_path), "config")?;
    let profile: Profile = parse_toml(Path::new(profile_path), "profile")?;
    let process = SystemProcess;
    let clock = SystemClock;
    let git = Git::new(process, config.repo.clone());
    let github = Gh::new(process, config.repo.clone(), config.github_repo.clone());
    let harness = harness(profile.harness.kind, process);
    let tracer = tracer(process);
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
            tracer: &tracer,
        },
    )
}

fn harness(kind: HarnessKind, process: SystemProcess) -> Box<dyn Harness> {
    match kind {
        HarnessKind::Codex => Box::new(Codex::new(process)),
        HarnessKind::Pi => Box::new(Pi::new(process)),
    }
}

/// The trace mirror: every committed event, sent through the `prefactor`
/// CLI when the environment names an agent. The event enum's schema goes
/// with it so each span type is declared.
fn tracer(process: SystemProcess) -> Prefactor<SystemProcess, SystemClock> {
    let schema = serde_json::to_string(&schemars::schema_for!(domain::event::Event)).ok();
    Prefactor::new(process, SystemClock, Tracing::from_env(), schema)
}

fn internal(command: &str, message: String) -> AgentError {
    AgentError::new(command, None, Rejection::Internal(message), None)
}

fn fail(error: AgentError) -> ExitCode {
    eprintln!(
        "{}",
        serde_json::to_string(&error).unwrap_or_else(|_| "{\"class\":\"internal\"}".into())
    );
    ExitCode::from(error.exit_code())
}
