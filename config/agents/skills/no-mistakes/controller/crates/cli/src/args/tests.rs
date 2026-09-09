use super::*;

#[test]
fn every_command_variant_is_a_subcommand() {
    assert_eq!(app::schema::command_names().len(), 36);
}

#[test]
fn scalar_field_round_trips_from_argv() -> Result<(), Box<dyn std::error::Error>> {
    let args = build::parse(
        "claim".into(),
        &["GAIN-1".into(), "--run".into(), "/run".into()],
    )?;
    let command = command(&args)?;
    assert!(matches!(command, Command::Claim { task } if task.as_ref() == "GAIN-1"));
    Ok(())
}

#[test]
fn non_scalar_field_on_argv_is_rejected_naming_input() {
    let result = build::parse(
        "hold".into(),
        &[
            "GAIN-1".into(),
            "--reason".into(),
            "x".into(),
            "--run".into(),
            "/run".into(),
        ],
    );
    let Err(error) = result else {
        panic!("non-scalar field must be rejected");
    };
    assert!(error.message.contains("--input"));
}

#[test]
fn help_for_subcommand_lists_required_field() {
    let text = help_text(Some("claim"));
    assert!(text.contains("task"));
    assert!(text.contains("required"));
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

#[test]
fn duplicate_option_fails_closed() {
    let result = build::parse(
        "claim".into(),
        &[
            "GAIN-1".into(),
            "--run".into(),
            "/one".into(),
            "--run".into(),
            "/two".into(),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn init_rejects_input_and_check() {
    let args = Args {
        command: "init".into(),
        run: "/run".into(),
        input: Some("x".into()),
        check: false,
        pretty: false,
        values: BTreeMap::new(),
    };
    assert!(validate_init(&args).is_err());
}

#[test]
fn bundled_profiles_validate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["codex", "pi"] {
        let path = root.join("../profiles").join(format!("{name}.toml"));
        let profile = parse_toml::<domain::risk::Profile>(&path, "profile")
            .unwrap_or_else(|error| panic!("{name} profile failed: {error:?}"));
        assert!(domain::risk::validate(&profile).is_ok());
    }
}
