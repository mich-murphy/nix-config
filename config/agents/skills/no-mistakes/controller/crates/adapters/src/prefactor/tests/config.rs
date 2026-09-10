//! Capture flags, text caps and `Tracing` configuration.

use super::*;

#[test]
fn capture_flags_drop_prompt_and_output() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let prompt = directory.path().join("prompt.md");
    std::fs::write(&prompt, "secret")?;
    let fake = Fake::new();
    let mut state = Memory::default();
    let quiet = Tracing {
        capture_inputs: false,
        capture_outputs: false,
        ..tracing()
    };
    let tracer = Prefactor::new(&fake, Fixed, Some(quiet), Some(SCHEMA.into()));
    tracer.record(
        &mut state,
        &[record(1, started(&prompt)?), record(2, ended("out"))],
    );
    assert!(
        fake.item(2)["details"]["payload"]
            .get("prompt_text")
            .is_none()
    );
    let result = &fake.item(3)["result_payload"];
    assert!(result["result"]["completed"].get("output").is_none());
    assert_eq!(result["result"]["completed"]["session"], "s");
    Ok(())
}

#[test]
fn long_text_is_capped() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let long = "x".repeat(spans::MAX_TEXT + 10);
    tracer(&fake).record(&mut state, &[record(1, ended(&long))]);
    let output =
        fake.item(2)["details"]["payload"]["result"]["completed"]["output"]["value"].clone();
    let text = output.as_str().unwrap_or_default();
    assert!(text.ends_with("[truncated]"));
    assert!(text.chars().count() < spans::MAX_TEXT + 20);
    Ok(())
}

#[test]
fn tracing_reads_identity_and_capture_flags() {
    let complete = |key: &str| match key {
        "PREFACTOR_API_TOKEN" => Some("t".to_owned()),
        "PREFACTOR_AGENT_ID" => Some("a1".to_owned()),
        "PREFACTOR_AGENT_IDENTIFIER" => Some("1.0.0".to_owned()),
        "PREFACTOR_CAPTURE_OUTPUTS" => Some("false".to_owned()),
        _ => None,
    };
    let tracing = Tracing::from_lookup(complete);
    assert_eq!(
        tracing.as_ref().map(|value| value.agent_id.as_str()),
        Some("a1")
    );
    assert_eq!(
        tracing.as_ref().map(|value| value.capture_inputs),
        Some(true)
    );
    assert_eq!(
        tracing.as_ref().map(|value| value.capture_outputs),
        Some(false)
    );
    let no_token = |key: &str| {
        (key != "PREFACTOR_API_TOKEN")
            .then(|| complete(key))
            .flatten()
    };
    assert_eq!(Tracing::from_lookup(no_token), None);
}

#[test]
fn rfc3339_formats_utc() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339(1_757_415_393), "2025-09-09T10:56:33Z");
}
