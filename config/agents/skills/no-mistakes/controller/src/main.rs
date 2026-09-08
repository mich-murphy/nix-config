use anyhow::{Context, Result, bail, ensure};
use epic_control::{engine, model::*, runtime, store::*};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

fn run() -> Result<Value> {
    let mut args = std::env::args().skip(1);
    let first = args.next().unwrap_or_default();
    if first == "__owned-child" {
        runtime::owned_child(std::env::args_os().skip(2).collect())?;
        return Ok(json!({"child":true}));
    }
    if first == "--help" || first.is_empty() {
        println!(
            "epic-control --state /absolute/ignored/run COMMAND [--input request.json]\n\nCommands: recover-superseded prepare-approved-cleanup usage-bind usage-sync begin-stage end-stage review-readiness reconcile-plan prepare-followup authorize-budget-adjustment evidence-batch usage-import usage-report review-schema validate-review\ninit configure-loop loop-plan measure checkpoint human-review lesson-record discover refresh claim brief reserve-slot bind snapshot evidence subtask-record\nagent-begin agent-finish disposition hold resume jira-prepare dispatch jira-observe\njira-retry sync-failed gh-prepare final-verify complete cleanup-check status next\n\nRuntime commands: gh-run (operation), gh-observe (key), poll-checks (key),\nrun-agent, recover-agent, run-check, recover-operation, gh-resolve-failure,\nprovision-slot, cleanup, reconcile-helper, drive (waits on required CI only).\n\nJSON input comes from --input or stdin. status, next and drive need no input.\nSee ../references/controller.md for schemas, invocation points and recovery."
        );
        return Ok(json!({"help":true}));
    }
    ensure!(first == "--state", "expected --state");
    let root = PathBuf::from(args.next().context("state path required")?);
    let command = args.next().context("command required")?;
    let mut value = read_request(&mut args, &command)?;
    if command == "review-schema" {
        ensure!(value == json!({}), "review-schema takes no fields");
        return Ok(epic_control::review_format::schema());
    }
    if command == "validate-review" {
        return epic_control::review_format::validate(value);
    }
    if let Some(allowed) = runtime_fields(&command) {
        ensure!(
            value
                .as_object()
                .unwrap()
                .keys()
                .all(|key| allowed.contains(&key.as_str())),
            "unknown runtime request field"
        );
        return execute_runtime(&root, &command, value);
    }
    ensure!(
        value.get("command").is_none(),
        "command is supplied by argv, not payload"
    );
    value["command"] = json!(command);
    let input: Input = serde_json::from_value(value)?;
    if matches!(input, Input::Init { .. }) {
        return initialize(&root, input);
    }
    let mut store = Store::open(&root)?;
    epic_control::readiness::preserve(&store.root, &store.read()?, &input)?;
    execute_input(&mut store, &command, input)
}

fn execute_input(store: &mut Store, command: &str, input: Input) -> Result<Value> {
    match input {
        Input::Status => {
            let state = store.read()?;
            store.render(&state)?;
            Ok(json!(state))
        }
        Input::UsageReport => Ok(epic_control::telemetry::report(&store.read()?)),
        Input::ReviewReadiness { key } => epic_control::readiness::report(&store.read()?, &key),
        Input::ReconcilePlan { key } => {
            epic_control::reconciliation::plan(&store.read()?, key.as_deref())
        }
        Input::Next => Ok(engine::next(&store.read()?, now())),
        input => store.change(command, |s| engine::apply(s, input, now())),
    }
}

fn main() {
    match run() {
        Ok(result) => println!("{}", json!({"ok":true,"result":result})),
        Err(e) => {
            eprintln!("{}", json!({"ok":false,"error":format!("{e:#}")}));
            std::process::exit(1);
        }
    }
}

fn runtime_fields(command: &str) -> Option<&'static [&'static str]> {
    Some(match command {
        "gh-run" | "reconcile-helper" => &["operation"],
        "gh-observe" => &["key", "pr"],
        "poll-checks" => &["key"],
        "prepare-followup" => &["key", "source", "artifact", "scope", "requirements"],
        "recover-superseded" => &[
            "key",
            "source",
            "artifact",
            "scope",
            "requirements",
            "replacement",
            "cleanup_scope",
            "cleanup_paths",
            "cleanup_criteria",
        ],
        "prepare-approved-cleanup" => &["key", "preparation", "outcome_artifact"],
        "drive" => &[],
        "recover-agent" => &["launch", "terminate"],
        "recover-operation" => &["operation", "terminate"],
        "gh-resolve-failure" => &["operation", "evidence"],
        "run-check" => &[
            "key",
            "criteria",
            "argv",
            "cwd",
            "timeout_seconds",
            "implementation",
        ],
        "provision-slot" => &["slot", "branch"],
        "cleanup" => &["key", "delete"],
        "run-agent" => &[
            "key",
            "role",
            "model",
            "effort",
            "reason",
            "ci_repair",
            "allow_evidence_gaps",
            "prompt",
            "timeout_seconds",
        ],
        _ => return None,
    })
}

fn execute_runtime(root: &Path, command: &str, value: Value) -> Result<Value> {
    let mut store = Store::open(root)?;
    match command {
        "gh-observe" => observe_request(&mut store, value),
        "prepare-followup" => runtime::prepare_followup(&mut store, serde_json::from_value(value)?),
        "poll-checks" => {
            runtime::poll_checks(&mut store, value["key"].as_str().context("key required")?)
        }
        "drive" => runtime::drive(&mut store),
        "run-agent" => runtime::run_agent(&mut store, serde_json::from_value(value)?),
        "provision-slot" => runtime::provision_slot(
            &mut store,
            value["slot"].as_str().context("slot required")?,
            value["branch"].as_str().context("branch required")?,
        ),
        "cleanup" => runtime::cleanup(
            &mut store,
            value["key"].as_str().context("key required")?,
            value["delete"]
                .as_bool()
                .context("delete boolean required")?,
        ),
        "reconcile-helper" => runtime::reconcile_helper(
            &mut store,
            value["operation"].as_u64().context("operation required")?,
        ),
        _ => execute_process(&mut store, command, value),
    }
}
fn execute_process(store: &mut Store, command: &str, value: Value) -> Result<Value> {
    match command {
        "recover-superseded" => {
            epic_control::superseded::recover(store, serde_json::from_value(value)?)
        }
        "prepare-approved-cleanup" => {
            epic_control::superseded::cleanup(store, serde_json::from_value(value)?)
        }
        "gh-run" => runtime::run_gh(
            store,
            value["operation"].as_u64().context("operation required")?,
        ),
        "recover-agent" => runtime::recover_agent(
            store,
            value["launch"].as_u64().context("launch required")?,
            value["terminate"]
                .as_bool()
                .context("terminate boolean required")?,
        ),
        "gh-resolve-failure" => runtime::resolve_gh_failure(
            store,
            value["operation"].as_u64().context("operation required")?,
            value["evidence"].as_str().context("evidence required")?,
        ),
        "run-check" => runtime::run_check(store, serde_json::from_value(value)?),
        "recover-operation" => runtime::recover_operation(
            store,
            value["operation"].as_u64().context("operation required")?,
            value["terminate"]
                .as_bool()
                .context("terminate boolean required")?,
        ),
        _ => bail!("unknown process command"),
    }
}

fn initialize(root: &Path, input: Input) -> Result<Value> {
    let Input::Init {
        repo,
        github_repo,
        epic,
        statuses,
        coordinator_model,
        coordinator_effort,
        preflight,
        loop_policy,
    } = input
    else {
        bail!("expected init command");
    };
    required_text(&epic, "epic identity")?;
    required_text(&preflight, "prerequisite evidence")?;
    ensure!(
        github_repo.split('/').count() == 2
            && !github_repo.contains(char::is_whitespace)
            && !github_repo.starts_with('-'),
        "GitHub repository must be owner/name"
    );
    let ids = [
        &statuses.todo,
        &statuses.progress,
        &statuses.review,
        &statuses.done,
    ];
    ensure!(
        ids.iter().all(|v| !v.is_empty())
            && ids.iter().collect::<std::collections::BTreeSet<_>>().len() == 4,
        "four distinct configured Jira status IDs required"
    );
    required_text(&coordinator_model, "actual coordinator model")?;
    required_text(&coordinator_effort, "actual coordinator effort")?;
    epic_control::loops::validate_policy(&loop_policy)?;
    let state = State {
        usage_imports: Vec::new(),
        lessons: BTreeMap::new(),
        loop_policy: Some(loop_policy),
        version: 2,
        run_id: hash(root.to_string_lossy().as_bytes()),
        repo: repo.canonicalize()?,
        github_repo,
        epic,
        statuses,
        coordinator_model,
        coordinator_effort,
        preflight,
        tasks: BTreeMap::new(),
        frozen: false,
        order: vec![],
        refreshed: 0,
        refresh_generation: 0,
        selection_generation: 0,
        active: None,
        launches: vec![],
        operations: vec![],
        slots: BTreeMap::new(),
        serial: 0,
        followups: vec![],
    };
    let mismatch = state.coordinator_model != "gpt-5.6-sol" || state.coordinator_effort != "medium";
    let store = Store::create(root, state)?;
    Ok(json!({"state":store.root,"coordinator_mismatch":mismatch,"next":"discover"}))
}

fn read_request(args: &mut impl Iterator<Item = String>, command: &str) -> Result<Value> {
    let mut bytes = String::new();
    if let Some(flag) = args.next() {
        ensure!(flag == "--input", "only --input is accepted");
        bytes = fs::read_to_string(args.next().context("input file required")?)?;
        ensure!(args.next().is_none(), "unexpected argument");
    } else if ![
        "status",
        "next",
        "drive",
        "usage-report",
        "usage-sync",
        "review-schema",
    ]
    .contains(&command)
    {
        io::stdin().read_to_string(&mut bytes)?;
    }
    let value: Value = if bytes.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&bytes)?
    };
    ensure!(value.is_object(), "request must be an object");

    Ok(value)
}

fn observe_request(store: &mut Store, value: Value) -> Result<Value> {
    let key = value["key"].as_str().context("key required")?;
    let pr = value
        .get("pr")
        .map(|p| p.as_u64().context("invalid PR number"))
        .transpose()?;
    runtime::observe_selected_pr(store, key, pr)
}
