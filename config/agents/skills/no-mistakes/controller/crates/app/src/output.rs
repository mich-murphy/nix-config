use domain::{
    command::ActionEnvelope,
    event::EventRecord,
    ids::{LaunchId, TransitionId},
    ports::{Capabilities, Tokens},
    state::State,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Output {
    pub events: Vec<EventRecord>,
    #[serde(flatten)]
    pub result: ResultData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum ResultData {
    State { state: Box<State> },
    Next { next: Option<ActionEnvelope> },
    Usage { usage: UsageReport },
    Transition { transition: TransitionId },
    Launch { launch: LaunchId, output: String },
    ReviewSchema { schema: serde_json::Value },
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
            report.capture(launch, state.usage.get(&launch.id));
        }
        report
    }

    fn capture(&mut self, launch: &domain::event::Launch, usage: Option<&Option<Tokens>>) {
        match usage {
            Some(Some(tokens)) => self.add(*tokens),
            Some(None) | None if launch.session.is_some() => self.unknown.push(launch.id),
            Some(None) | None => {}
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
