//! Register native log boundaries once; terminal settlement imports best-effort.
use crate::{model::*, telemetry};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub session: String,
    pub artifact: PathBuf,
    pub start_line: usize,
    pub end_line: Option<usize>,
}
pub fn bind(s: &mut State, id: u64, source: Source) -> Result<Value> {
    ensure!(
        source.artifact.is_absolute() && source.start_line > 0,
        "absolute log path and one-based interval required"
    );
    let l = s
        .launches
        .iter_mut()
        .find(|l| l.id == id)
        .context("unknown launch")?;
    ensure!(
        l.usage.is_none() && l.usage_source.is_none(),
        "usage or source already recorded"
    );
    ensure!(
        l.session
            .as_ref()
            .is_none_or(|session| session == &source.session),
        "session mismatch"
    );
    ensure!(
        l.ended.is_none() || source.end_line.is_some(),
        "terminal binding requires an exact end line"
    );
    let first = BufReader::new(File::open(&source.artifact)?)
        .lines()
        .next()
        .context("empty session log")??;
    let meta: Value = serde_json::from_str(&first)?;
    ensure!(
        meta["type"] == "session_meta" && meta["payload"]["id"] == source.session,
        "session log identity mismatch"
    );
    ensure!(
        source.end_line.is_none_or(|end| end >= source.start_line),
        "invalid usage interval"
    );
    l.usage_source = Some(source);
    s.version = s.version.max(8);
    Ok(json!({"bound":id,"import_on_terminal_settlement":true}))
}
pub fn freeze(s: &mut State, id: u64) -> Result<()> {
    let l = s
        .launches
        .iter_mut()
        .find(|l| l.id == id)
        .context("launch")?;
    if let Some(source) = &mut l.usage_source {
        ensure!(
            l.session.as_ref() == Some(&source.session),
            "bound usage session differs from terminal receipt"
        );
        if source.end_line.is_none() {
            source.end_line = Some(
                BufReader::new(File::open(&source.artifact)?)
                    .lines()
                    .collect::<std::io::Result<Vec<_>>>()?
                    .len(),
            );
        }
    }
    Ok(())
}
fn import_one(s: &mut State, id: u64) -> Result<()> {
    let l = s.launches.iter().find(|l| l.id == id).context("launch")?;
    let source = l
        .usage_source
        .as_ref()
        .context("native usage source not bound")?;
    let request = telemetry::ImportRequest {
        session: source.session.clone(),
        artifact: source.artifact.clone(),
        start_line: source.start_line,
        end_line: source
            .end_line
            .context("terminal boundary unknown; use usage-import with an exact interval")?,
        launch: Some(id),
        category: telemetry::category(s, l).into(),
    };
    telemetry::import(s, request)?;
    Ok(())
}
pub fn settle(s: &mut State, id: u64) -> Option<String> {
    if s.launches
        .iter()
        .find(|l| l.id == id)
        .is_some_and(|l| l.usage.is_some())
    {
        return None;
    }
    freeze(s,id).and_then(|()| import_one(s,id)).err().map(|error| format!("Usage unknown: {error}. Bind/import the exact terminal interval; delivery is unaffected."))
}
pub fn sync(s: &mut State) -> Value {
    let ids: Vec<_> = s
        .launches
        .iter()
        .filter(|l| l.ended.is_some() && l.usage.is_none())
        .map(|l| l.id)
        .collect();
    let mut unavailable = Vec::new();
    for id in ids {
        // Never expand a terminal interval during a later sync; a resumed turn may exist.
        if let Err(error) = import_one(s, id) {
            unavailable.push(json!({"launch":id,"reason":error.to_string()}));
        }
    }
    json!({"usage":telemetry::report(s),"unavailable":unavailable})
}
