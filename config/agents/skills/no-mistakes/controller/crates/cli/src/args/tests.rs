use super::*;

#[test]
fn command_surface_has_36_commands() {
    assert_eq!(COMMANDS.len(), 36);
    let mut sorted = COMMANDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 36);
}

#[test]
fn unknown_field_fails_closed() {
    let text = r#"{
        "command":"hold",
        "task":"GAIN-1",
        "reason":{"NeedsHuman":{"diagnosis":"blocked","remaining":[],"extra":true}}
    }"#;
    assert!(serde_json::from_str::<Command>(text).is_err());
}

#[test]
fn unknown_top_level_field_fails_closed() {
    let text = r#"{"command":"claim","task":"GAIN-1","extra":true}"#;
    assert!(serde_json::from_str::<Command>(text).is_err());
}
