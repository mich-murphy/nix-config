use app::UsageReport;
use domain::{
    command::AgentRole,
    event::{Launch, LaunchOutcome},
    ids::{DeliveryId, LaunchId, TaskId},
    ports::Tokens,
    state::State,
};
use std::{path::PathBuf, str::FromStr};

fn launch(id: u64) -> Launch {
    Launch {
        id: LaunchId(id),
        task: TaskId::from_str("GAIN-1").unwrap_or_else(|error| panic!("fixture: {error}")),
        delivery: DeliveryId(1),
        role: AgentRole::Implementer,
        prompt: PathBuf::from("/prompt"),
        outcome: Some(LaunchOutcome::Completed {
            session: "session".into(),
            output: String::new(),
        }),
        usage: None,
        checkpointed: false,
        process: None,
    }
}

#[test]
fn missing_usage_is_unknown() {
    let mut state = State::empty();
    state.launches.push(launch(1));
    let report = UsageReport::from_state(&state);
    assert_eq!(report.unknown, vec![LaunchId(1)]);
}

#[test]
fn cache_count_stays_null() {
    let mut state = State::empty();
    let mut item = launch(1);
    item.usage = Some(Tokens {
        input: 4,
        cached: None,
        output: 2,
    });
    state.launches.push(item);
    assert_eq!(UsageReport::from_state(&state).cached, None);
}

/// A launch that settled without usable usage (missing or malformed alike:
/// the harness adapter drops a malformed report rather than fabricating
/// one) never reports a fabricated zero; it is unknown, whether it
/// completed or, as here, failed outright.
#[test]
fn malformed_usage_is_dropped() {
    let mut state = State::empty();
    let mut item = launch(1);
    item.outcome = Some(LaunchOutcome::Failed {
        reason: "malformed usage".into(),
    });
    state.launches.push(item);
    let report = UsageReport::from_state(&state);
    assert_eq!(report.unknown, vec![LaunchId(1)]);
    assert_eq!(report.input, 0);
}

#[test]
fn report_has_no_cost() {
    let encoded = serde_json::to_value(UsageReport::default())
        .unwrap_or_else(|error| panic!("serialize report: {error}"));
    assert!(encoded.get("cost").is_none());
    assert!(encoded.get("billing_cost").is_none());
}
