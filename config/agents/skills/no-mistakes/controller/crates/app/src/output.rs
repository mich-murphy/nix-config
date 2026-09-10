use crate::schema::AnnotatedAction;
use domain::{
    event::{EventRecord, Launch},
    ids::{LaunchId, TransitionId},
    ports::{Capabilities, Tokens},
    review::Verdict,
    state::State,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Output {
    pub events: Vec<EventRecord>,
    /// The action `next` would name after this command's events were
    /// written, attached to every command that wrote any, so the
    /// coordinator does not need a separate `next` call between steps.
    /// Absent on read-only commands and on `--check`, whose events were
    /// not written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<AnnotatedAction>,
    #[serde(flatten)]
    pub result: ResultData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum ResultData {
    State { state: Box<State>, verified: bool },
    Next { next: Option<AnnotatedAction> },
    Usage { usage: UsageReport },
    Transition { transition: TransitionId },
    Launch { launch: LaunchId, output: String },
    ReviewSchema { schema: schemars::Schema },
    Verdict { verdict: Verdict },
    Valid,
    Applied,
    Initialized { capabilities: Capabilities },
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageReport {
    pub input: u64,
    pub cached: Option<u64>,
    pub output: u64,
    pub unknown: Vec<LaunchId>,
}

impl UsageReport {
    #[must_use]
    pub fn from_state(state: &State) -> Self {
        let mut report = Self {
            cached: Some(0),
            ..Self::default()
        };
        for launch in &state.launches {
            report.capture(launch);
        }
        report
    }

    /// Usage now lives on the launch itself, captured once by
    /// `LaunchEnded`. `None` there means unknown, whether because the
    /// harness never reported it or because what it reported was
    /// malformed and dropped; either way a settled launch with no usage
    /// is reported unknown, never fabricated as zero.
    fn capture(&mut self, launch: &Launch) {
        match launch.usage {
            Some(tokens) => self.add(tokens),
            None if launch.outcome.is_some() => self.unknown.push(launch.id),
            None => {}
        }
    }

    pub fn add(&mut self, tokens: Tokens) {
        self.input = self.input.saturating_add(tokens.input);
        self.output = self.output.saturating_add(tokens.output);
        self.cached = match (self.cached, tokens.cached) {
            (Some(total), Some(value)) => Some(total.saturating_add(value)),
            _ => None,
        };
    }
}
