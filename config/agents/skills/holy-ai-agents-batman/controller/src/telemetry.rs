//! Import cumulative session counters once per non-overlapping line interval.
//! Receipts contain counters and provenance, never conversation or credentials.
use crate::{ids::Digest, model::*, store::*};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub input_tokens: u64,
    #[serde(default)]
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: u64,
}

impl Default for Tokens {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            cached_input_tokens: Some(0),
            output_tokens: 0,
        }
    }
}

pub fn normalize(value: &Value) -> Result<Tokens> {
    let tokens: Tokens = serde_json::from_value(value.clone())?;
    ensure!(
        tokens
            .cached_input_tokens
            .is_none_or(|cached| cached <= tokens.input_tokens),
        "cached input exceeds input"
    );
    Ok(tokens)
}

pub fn settlement_usage(usage: Option<Value>) -> (Option<Value>, Option<&'static str>) {
    match usage {
        Some(value) => match normalize(&value) {
            Ok(tokens) => (Some(json!(tokens)), None),
            Err(_) => (
                None,
                Some(
                    "Invalid optional usage was not recorded; import terminal session counters when available.",
                ),
            ),
        },
        None => (None, None),
    }
}

impl Tokens {
    fn difference(&self, baseline: &Self) -> Result<Self> {
        let delta = Self {
            input_tokens: self
                .input_tokens
                .checked_sub(baseline.input_tokens)
                .context("input counter decreased")?,
            cached_input_tokens: match (self.cached_input_tokens, baseline.cached_input_tokens) {
                (Some(end), Some(start)) => {
                    Some(end.checked_sub(start).context("cache counter decreased")?)
                }
                _ => None,
            },
            output_tokens: self
                .output_tokens
                .checked_sub(baseline.output_tokens)
                .context("output counter decreased")?,
        };
        ensure!(
            delta
                .cached_input_tokens
                .is_none_or(|cached| cached <= delta.input_tokens),
            "cached input exceeds input"
        );
        Ok(delta)
    }

    fn add(&mut self, other: &Self) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.cached_input_tokens = self
            .cached_input_tokens
            .zip(other.cached_input_tokens)
            .map(|(left, right)| left.saturating_add(right));
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    pub session: String,
    pub artifact: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub launch: Option<u64>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReceipt {
    pub request: ImportRequest,
    pub model: String,
    pub counters: Tokens,
    pub digest: Digest,
}

pub fn import(s: &mut State, request: ImportRequest) -> Result<Value> {
    required_text(&request.session, "actual session id")?;
    ensure!(
        request.artifact.is_absolute(),
        "session log path must be absolute"
    );
    ensure!(
        request.start_line > 0 && request.end_line >= request.start_line,
        "invalid line interval"
    );
    for old in &s.usage_imports {
        if old.request.session == request.session {
            ensure!(
                request.end_line < old.request.start_line
                    || request.start_line > old.request.end_line,
                "session interval already accounted for"
            );
        }
    }
    validate_target(s, &request)?;
    let receipt = read_interval(request)?;
    if let Some(id) = receipt.request.launch {
        let launch = s
            .launches
            .iter_mut()
            .find(|l| l.id == id)
            .context("unknown launch")?;
        launch.usage = Some(serde_json::to_value(&receipt.counters)?);
    }
    s.usage_imports.push(receipt.clone());
    s.version = s.version.max(6);
    Ok(json!({"recorded":receipt,"billing_cost":null}))
}

fn validate_target(s: &State, request: &ImportRequest) -> Result<()> {
    if let Some(id) = request.launch {
        let launch = s
            .launches
            .iter()
            .find(|l| l.id == id)
            .context("unknown launch")?;
        ensure!(
            launch.ended.is_some() && launch.session.as_deref() == Some(&request.session),
            "import requires a terminal launch in the matching session"
        );
        ensure!(
            request.start_line > 1
                || !s
                    .launches
                    .iter()
                    .any(|prior| prior.id < id && prior.session == launch.session),
            "resumed launch requires its own interval, not the whole cumulative session"
        );
        ensure!(
            launch
                .usage
                .as_ref()
                .is_none_or(|value| normalize(value).is_err()),
            "launch already has usage; do not count it twice"
        );
        ensure!(
            request.category == category(s, launch),
            "category must match the launch role and repair history"
        );
    } else {
        ensure!(
            ["coordinator", "controller-maintenance"].contains(&request.category.as_str()),
            "task agent usage must identify its launch"
        );
        ensure!(
            !s.launches
                .iter()
                .any(|l| l.session.as_deref() == Some(&request.session)),
            "task session must be attributed to its launch"
        );
    }
    Ok(())
}

fn cumulative(value: &Value) -> Option<Value> {
    let payload = &value["payload"];
    if value["type"] == "event_msg" && payload["type"] == "token_count" {
        return payload["info"]
            .get("total_token_usage")
            .filter(|v| v.is_object())
            .cloned();
    }
    if value["type"] == "token_usage_record" {
        return payload
            .get("last_thread_token_usage")
            .filter(|v| v.is_object())
            .cloned();
    }
    None
}

fn read_interval(request: ImportRequest) -> Result<UsageReceipt> {
    let mut baseline = None;
    let mut latest = None;
    let mut session = None;
    let mut model = None;
    let mut proof = Vec::new();
    let mut count = 0;
    for (index, line) in BufReader::new(File::open(&request.artifact)?)
        .lines()
        .take(request.end_line)
        .enumerate()
    {
        count = index + 1;
        let value: Value = serde_json::from_str(&line?)?;
        if value["type"] == "session_meta" {
            session = value["payload"]["id"].as_str().map(str::to_owned);
        }
        if value["type"] == "turn_context" {
            model = value["payload"]["model"].as_str().map(str::to_owned);
        }
        if let Some(raw) = cumulative(&value) {
            let tokens = normalize(&raw)?;
            if count < request.start_line {
                baseline = Some(tokens);
            } else {
                proof.push(json!({"line":count,"tokens":tokens}));
                latest = Some(tokens);
            }
        }
    }
    ensure!(
        count == request.end_line,
        "requested interval exceeds the recorded session"
    );
    ensure!(
        session.as_deref() == Some(&request.session),
        "session log identity mismatch"
    );
    let baseline = if request.start_line == 1 {
        Tokens::default()
    } else {
        baseline.context("interval needs a preceding cumulative counter")?
    };
    let counters = latest
        .context("no cumulative usage in interval")?
        .difference(&baseline)?;
    let digest = hash(&serde_json::to_vec(
        &json!({"request":request,"baseline":baseline,"observations":proof}),
    )?);
    Ok(UsageReceipt {
        request,
        model: model.context("actual model not recorded")?,
        counters,
        digest: digest.parse()?,
    })
}

pub fn category(s: &State, launch: &Launch) -> &'static str {
    match launch.role {
        Role::Reviewer => "review",
        Role::Escalation => "escalation",
        Role::Implementer => {
            if s.launches
                .iter()
                .any(|l| l.key == launch.key && l.role == Role::Reviewer && l.id < launch.id)
            {
                "repair"
            } else {
                "implementation"
            }
        }
    }
}

pub fn report(s: &State) -> Value {
    let mut totals = Tokens::default();
    let mut categories: BTreeMap<String, Tokens> = BTreeMap::new();
    let mut missing = Vec::new();
    for launch in &s.launches {
        if let Some(tokens) = launch.usage.as_ref().and_then(|v| normalize(v).ok()) {
            totals.add(&tokens);
            categories
                .entry(category(s, launch).into())
                .or_default()
                .add(&tokens);
        } else {
            missing.push(launch.id);
        }
    }
    for receipt in s
        .usage_imports
        .iter()
        .filter(|r| r.request.launch.is_none())
    {
        totals.add(&receipt.counters);
        categories
            .entry(receipt.request.category.clone())
            .or_default()
            .add(&receipt.counters);
    }
    json!({"tokens":totals,"categories":categories,"launches_without_usage":missing,
        "coordinator_usage_recorded":categories.contains_key("coordinator"),
        "imports":s.usage_imports.len(),"billing_cost":null,
        "note":"Recorded tokens include repeated cached input. Reasoning is not added to output. Missing usage and cache counts are unknown, not zero; no billing conversion is inferred."})
}
