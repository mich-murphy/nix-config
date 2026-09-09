use super::{Args, invalid};
use app::{AgentError, schema::FieldKind};
use clap::{Arg, ArgAction, Args as ClapArgs, Command as Cli, FromArgMatches};
use std::{collections::BTreeMap, path::PathBuf};

/// The static "frame" every subcommand shares: `clap` derive handles
/// these four, validating type, presence and (for `--run`) requiredness
/// on its own. Every other field a command accepts is added to the
/// dynamic `Command` built in `build_command`, one per schema property.
#[derive(clap::Args)]
struct Frame {
    #[arg(long)]
    run: PathBuf,
    #[arg(long)]
    input: Option<String>,
    #[arg(long)]
    check: bool,
    #[arg(long)]
    pretty: bool,
}

pub(super) fn parse(command: String, rest: &[String]) -> Result<Args, AgentError> {
    let built = build_command(&command);
    let matches = built.try_get_matches_from(rest).map_err(|error| {
        invalid(
            &command,
            &format!("{error}\nnon-scalar fields need --input"),
        )
    })?;
    let frame =
        Frame::from_arg_matches(&matches).map_err(|error| invalid(&command, &error.to_string()))?;
    let values = scalar_values(&command, &matches);
    Ok(Args {
        command,
        run: frame.run,
        input: frame.input,
        check: frame.check,
        pretty: frame.pretty,
        values,
    })
}

/// The `clap::Command` for `name`: the shared frame plus, for `init`,
/// the two TOML file flags it alone takes, or otherwise one `Arg` per
/// scalar field of `name`'s real schema (the field named `task` as a
/// positional, since `clap` forbids a single argument from being both;
/// every other field as `--field value`). A field the schema does not
/// declare, or declares as non-scalar, is simply not a valid `clap`
/// argument here, so `clap` itself rejects it.
fn build_command(name: &str) -> Cli {
    let base = Cli::new(name.to_owned())
        .no_binary_name(true)
        .disable_help_flag(true)
        .disable_version_flag(true);
    let base = Frame::augment_args(base);
    if name == "init" {
        return base
            .arg(Arg::new("config").long("config").required(true))
            .arg(Arg::new("profile").long("profile").required(true));
    }
    let Some(schema) = app::schema::command_schema(name) else {
        return base;
    };
    app::schema::fields(&schema)
        .into_iter()
        .filter(|(_, info)| info.scalar())
        .fold(base, |built, (field, info)| {
            built.arg(field_arg(&field, info))
        })
}

fn field_arg(field: &str, info: app::schema::Field) -> Arg {
    let arg = Arg::new(field.to_owned())
        .required(info.required)
        .action(ArgAction::Set);
    let arg = match info.kind {
        FieldKind::Integer => arg.value_parser(clap::value_parser!(u64)),
        FieldKind::Boolean => arg.value_parser(clap::value_parser!(bool)),
        FieldKind::String | FieldKind::Other => arg.value_parser(clap::value_parser!(String)),
    };
    if field == "task" {
        arg.index(1)
    } else {
        arg.long(field.to_owned())
    }
}

/// Every scalar field `build_command` registered, read back out of
/// `matches` in the type `clap` validated it as and rendered to the
/// same string form `--input` would carry, so both sources merge through
/// one representation.
fn scalar_values(command: &str, matches: &clap::ArgMatches) -> BTreeMap<String, String> {
    if command == "init" {
        return ["config", "profile"]
            .into_iter()
            .filter_map(|field| {
                matches
                    .get_one::<String>(field)
                    .cloned()
                    .map(|value| (field.to_owned(), value))
            })
            .collect();
    }
    let Some(schema) = app::schema::command_schema(command) else {
        return BTreeMap::new();
    };
    app::schema::fields(&schema)
        .into_iter()
        .filter(|(_, info)| info.scalar())
        .filter_map(|(field, info)| {
            value_of(matches, &field, info.kind).map(|value| (field, value))
        })
        .collect()
}

fn value_of(matches: &clap::ArgMatches, field: &str, kind: FieldKind) -> Option<String> {
    match kind {
        FieldKind::Integer => matches.get_one::<u64>(field).map(u64::to_string),
        FieldKind::Boolean => matches.get_one::<bool>(field).map(bool::to_string),
        FieldKind::String | FieldKind::Other => matches.get_one::<String>(field).cloned(),
    }
}
