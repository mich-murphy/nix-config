use super::*;

#[test]
fn command_catalog_is_exact() {
    assert_eq!(
        COMMANDS,
        [
            "init",
            "status",
            "next",
            "usage-report",
            "discover",
            "refresh",
            "claim",
            "hold",
            "resume",
            "brief",
            "plan",
            "checkpoint",
            "snapshot",
            "subtask-record",
            "escalate-tier",
            "bind-slot",
            "cleanup",
            "record-proof",
            "run-check",
            "run-agent",
            "review-schema",
            "validate-review",
            "disposition",
            "human-review",
            "lesson-record",
            "publish",
            "observe-pr",
            "poll-checks",
            "final-verify",
            "complete",
            "set-status",
            "observe-status",
            "grant",
            "open-delivery",
            "narrow-acceptance",
            "recover-operation",
        ]
    );
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
    let raw = vec![
        "claim".into(),
        "GAIN-1".into(),
        "--run".into(),
        "/one".into(),
        "--run".into(),
        "/two".into(),
    ];
    assert!(parse_raw(&raw, "claim".into()).is_err());
}

#[test]
fn init_rejects_unhandled_fields() {
    let args = Args {
        command: "init".into(),
        run: "/run".into(),
        input: None,
        check: false,
        pretty: false,
        values: BTreeMap::from([
            ("config".into(), "/config".into()),
            ("profile".into(), "/profile".into()),
            ("extra".into(), "bad".into()),
        ]),
        positionals: Vec::new(),
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
