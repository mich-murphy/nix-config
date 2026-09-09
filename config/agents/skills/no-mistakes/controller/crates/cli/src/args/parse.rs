use super::{Args, invalid};
use app::AgentError;
use std::{collections::BTreeMap, path::PathBuf};

pub(super) fn parse_raw(raw: &[String], command: String) -> Result<Args, AgentError> {
    let mut run = None;
    let mut input = None;
    let mut check = false;
    let mut pretty = false;
    let mut values = BTreeMap::new();
    let mut positionals = Vec::new();
    let mut index = 1;
    while index < raw.len() {
        let arg = &raw[index];
        match classify(arg) {
            ArgKind::Check => set_switch(&command, arg, &mut check)?,
            ArgKind::Pretty => set_switch(&command, arg, &mut pretty)?,
            ArgKind::Flag => set_flag_value(&command, arg, &mut values)?,
            ArgKind::Option => {
                index += 1;
                let next = option_value(raw, index, &command, arg)?;
                parse_value(arg, next, &command, &mut run, &mut input, &mut values)?;
            }
            ArgKind::Positional => positionals.push(arg.clone()),
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

enum ArgKind {
    Check,
    Pretty,
    Flag,
    Option,
    Positional,
}

fn classify(argument: &str) -> ArgKind {
    match argument {
        "--check" => ArgKind::Check,
        "--pretty" => ArgKind::Pretty,
        "--delete" | "--wait" | "--terminate" => ArgKind::Flag,
        value if value.starts_with("--") => ArgKind::Option,
        _ => ArgKind::Positional,
    }
}

fn set_flag_value(
    command: &str,
    option: &str,
    values: &mut BTreeMap<String, String>,
) -> Result<(), AgentError> {
    let name = option.trim_start_matches("--").to_owned();
    if values.insert(name, "true".into()).is_some() {
        Err(invalid(command, &format!("duplicate option {option}")))
    } else {
        Ok(())
    }
}

fn option_value<'a>(
    raw: &'a [String],
    index: usize,
    command: &str,
    option: &str,
) -> Result<&'a str, AgentError> {
    raw.get(index)
        .map(String::as_str)
        .ok_or_else(|| invalid(command, &format!("{option} needs a value")))
}

fn set_switch(command: &str, option: &str, value: &mut bool) -> Result<(), AgentError> {
    if *value {
        Err(invalid(command, &format!("duplicate option {option}")))
    } else {
        *value = true;
        Ok(())
    }
}

fn parse_value(
    option: &str,
    value: &str,
    command: &str,
    run: &mut Option<PathBuf>,
    input: &mut Option<String>,
    values: &mut BTreeMap<String, String>,
) -> Result<(), AgentError> {
    let duplicate = match option {
        "--run" => run.replace(PathBuf::from(value)).is_some(),
        "--input" => input.replace(value.to_owned()).is_some(),
        _ => values
            .insert(option.trim_start_matches("--").into(), value.to_owned())
            .is_some(),
    };
    if duplicate {
        Err(invalid(command, &format!("duplicate option {option}")))
    } else {
        Ok(())
    }
}
