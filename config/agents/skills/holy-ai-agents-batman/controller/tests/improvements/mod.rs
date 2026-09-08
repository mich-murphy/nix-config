#[cfg(target_os = "linux")]
mod followup;
mod round2;
#[cfg(target_os = "linux")]
mod superseded;
use super::*;

fn hold_exhausted(f: &Fixture) {
    for _ in 0..3 {
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "reviewer", "cancelled", None).unwrap();
    }
    hold(f);
}
fn hold(f: &Fixture) {
    f.cli("hold", json!({"key":"TASK-1","outcome":"needs-human","reason":"remaining criterion needs authorized repair"})).unwrap();
}
fn resume(f: &Fixture) {
    let mut task = Fixture::spec("TASK-1", 1);
    task["jira_status"] = json!("2");
    f.cli("refresh", json!({"tasks":[task,Fixture::spec("TASK-2",2)]}))
        .unwrap();
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
}
fn adjustment(f: &Fixture, mode: &str, receipt: &str, paths: Value) -> Value {
    json!({"key":"TASK-1","mode":mode,"source":"explicit fixture user instruction",
        "artifact":f.root.join(receipt),"scope":"Bounded repair without changing acceptance","paths":paths})
}

fn documentation_fixture() -> Fixture {
    let f = Fixture::new();
    f.bound(false);
    let worker = f.repo.join(".worktrees/worker1");
    fs::write(worker.join("README.md"), "Existing operator instructions\n").unwrap();
    git(&worker, &["add", "."]).unwrap();
    git(&worker, &["commit", "-m", "documentation baseline"]).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    hold_exhausted(&f);
    fs::write(
        f.root.join("docs-approval.txt"),
        "User excludes this documentation repair from counting",
    )
    .unwrap();
    f
}

#[test]
fn explicit_root_documentation_is_exempt_but_still_needs_acceptance() {
    let f = documentation_fixture();
    let request = adjustment(
        &f,
        "documentation-cycle",
        "docs-approval.txt",
        json!(["README.md"]),
    );
    f.cli("authorize-budget-adjustment", request).unwrap();
    resume(&f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let worker = f.repo.join(".worktrees/worker1");
    fs::write(
        worker.join("README.md"),
        "Correct current operator instructions\n",
    )
    .unwrap();
    git(&worker, &["commit", "-am", "documentation correction"]).unwrap();
    f.finish(id, "implementer", "completed", None).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    f.pass_review();
    assert_eq!(f.state().tasks["TASK-1"].reviews, 3);
    assert!(!f.state().tasks["TASK-1"].final_verified);
}

#[test]
fn prospective_documentation_rejects_source_changes_even_when_reverted() {
    let f = documentation_fixture();
    let bad = adjustment(
        &f,
        "documentation-cycle",
        "docs-approval.txt",
        json!(["source.txt"]),
    );
    assert!(f.cli("authorize-budget-adjustment", bad).is_err());
    f.cli(
        "authorize-budget-adjustment",
        adjustment(
            &f,
            "documentation-cycle",
            "docs-approval.txt",
            json!(["README.md"]),
        ),
    )
    .unwrap();
    resume(&f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let worker = f.repo.join(".worktrees/worker1");
    fs::write(worker.join("source.txt"), "hidden executable change\n").unwrap();
    git(&worker, &["commit", "-am", "source change"]).unwrap();
    git(&worker, &["revert", "--no-edit", "HEAD"]).unwrap();
    fs::write(worker.join("README.md"), "Documentation only at endpoint\n").unwrap();
    git(&worker, &["commit", "-am", "documentation correction"]).unwrap();
    f.finish(id, "implementer", "completed", None).unwrap();
    let before = f.state().tasks["TASK-1"].snapshot.clone();
    assert!(f.cli("snapshot", json!({"key":"TASK-1"})).is_err());
    assert_eq!(f.state().tasks["TASK-1"].snapshot, before);
    assert!(f.begin("reviewer").is_err());
}

fn result(f: &Fixture, criterion: &str) -> Value {
    let log = f.root.join("batch-proof.txt");
    fs::write(&log, "observed contract outcome\n").unwrap();
    json!({"observed":"base condition corrected","satisfied":true,
        "evidence":{"criterion":criterion,"status":"passed","kind":"inspection",
        "artifact":log,"implementation":["source.txt:1"],"description":"contract proof","human_source":null}})
}

#[test]
fn batch_is_atomic_and_invalidates_review_only_on_success() {
    let f = Fixture::new();
    f.bound(false);
    f.pass_review();
    let before = f.state();
    let snap = before.tasks["TASK-1"].snapshot.clone().unwrap();
    let invalid =
        json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"AC1"),result(&f,"UNKNOWN")]});
    assert!(f.cli("evidence-batch", invalid).is_err());
    assert_eq!(
        serde_json::to_value(f.state().tasks["TASK-1"].review.clone()).unwrap(),
        serde_json::to_value(before.tasks["TASK-1"].review.clone()).unwrap()
    );
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"AC1")]}),
    )
    .unwrap();
    let s = f.state();
    assert!(s.tasks["TASK-1"].review.is_none());
    assert!(engine::acceptance(&s, &s.tasks["TASK-1"]).is_ok());
    assert_eq!(s.tasks["TASK-1"].reviews, 1);
}

#[test]
fn batch_cannot_satisfy_human_criteria_with_automated_evidence() {
    let f = Fixture::new();
    f.bound(true);
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    assert!(
        f.cli(
            "evidence-batch",
            json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"AC1")]})
        )
        .is_err()
    );
    assert!(f.state().tasks["TASK-1"].loop_state.measurements.is_empty());
    assert!(f.state().tasks["TASK-1"].evidence.is_empty());
}

#[test]
fn review_prevalidation_rejects_malformed_results_without_consuming_launches() {
    let f = Fixture::new();
    f.bound(false);
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let mut review = json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]});
    f.cli("validate-review", json!({"snapshot":snap,"review":review}))
        .unwrap();
    review["criteria"] = json!({"AC1":"PASS"});
    assert!(
        f.cli("validate-review", json!({"snapshot":snap,"review":review}))
            .is_err()
    );
    review.as_object_mut().unwrap().remove("criteria");
    review["evidence_gaps"] = json!(["missing human proof"]);
    assert!(
        f.cli("validate-review", json!({"snapshot":snap,"review":review}))
            .is_err()
    );
    assert!(f.state().launches.is_empty());
    assert!(f.state().tasks["TASK-1"].review.is_none());
}

fn session_log(f: &Fixture, session: &str) -> PathBuf {
    let path = f.root.join(format!("{session}.jsonl"));
    let rows = [
        json!({"type":"session_meta","payload":{"id":session}}),
        json!({"type":"turn_context","payload":{"model":"gpt-5.6-sol"}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":60,"output_tokens":10,"reasoning_output_tokens":4}}}}),
        json!({"type":"token_usage_record","payload":{"last_thread_token_usage":{"input_tokens":100,"cached_input_tokens":60,"output_tokens":10}}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":160,"cached_input_tokens":80,"output_tokens":20,"reasoning_output_tokens":8}}}}),
    ];
    fs::write(
        &path,
        rows.iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    path
}

#[test]
fn usage_import_subtracts_cumulative_counters_and_counts_each_launch_once() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "implementation-session", "completed", None)
        .unwrap();
    let path = session_log(&f, "implementation-session");
    let request = json!({"request":{"session":"implementation-session","artifact":path,"start_line":4,"end_line":5,"launch":id,"category":"implementation"}});
    f.cli("usage-import", request.clone()).unwrap();
    assert!(f.cli("usage-import", request).is_err());
    let path = session_log(&f, "coordinator-session");
    let first = json!({"request":{"session":"coordinator-session","artifact":path,"start_line":1,"end_line":3,"launch":null,"category":"coordinator"}});
    f.cli("usage-import", first.clone()).unwrap();
    assert!(f.cli("usage-import", first).is_err());
    f.cli("usage-import", json!({"request":{"session":"coordinator-session","artifact":path,"start_line":4,"end_line":5,"launch":null,"category":"coordinator"}})).unwrap();
    let report = f.cli("usage-report", json!({})).unwrap();
    assert_eq!(report["tokens"]["input_tokens"], 220);
    assert_eq!(report["tokens"]["cached_input_tokens"], 100);
    assert_eq!(report["tokens"]["output_tokens"], 30);
    assert_eq!(report["launches_without_usage"], json!([]));
    assert!(report["billing_cost"].is_null());
}

#[test]
fn usage_rejects_wrong_session_missing_counter_and_task_as_overhead() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "implementation-session", "completed", None)
        .unwrap();
    let path = session_log(&f, "another-session");
    let mut request = json!({"request":{"session":"implementation-session","artifact":path,"start_line":1,"end_line":5,"launch":id,"category":"implementation"}});
    assert!(f.cli("usage-import", request.clone()).is_err());
    request["request"]["artifact"] = json!(session_log(&f, "implementation-session"));
    request["request"]["end_line"] = json!(2);
    assert!(f.cli("usage-import", request.clone()).is_err());
    request["request"]["end_line"] = json!(5);
    request["request"]["launch"] = Value::Null;
    request["request"]["category"] = json!("coordinator");
    assert!(f.cli("usage-import", request).is_err());
    assert!(f.state().usage_imports.is_empty());
    assert!(f.state().launches[0].usage.is_none());
}

#[test]
fn additional_lesson_tags_select_only_accepted_relevant_guidance() {
    let f = Fixture::new();
    let lesson = |id: &str, families: Value, status: &str| {
        json!({"id":id,"families":families,
        "source":"verified fixture review","instruction":"Search current references","example":"source.txt",
        "status":status,"acceptance_source":"fixture approved source instruction"})
    };
    fs::write(
        f.repo.join("feedback.json"),
        serde_json::to_vec(&json!([
            lesson(
                "shared",
                json!(["fixture", "reference-propagation"]),
                "accepted"
            ),
            lesson("transferred", json!(["reference-propagation"]), "accepted"),
            lesson("unrelated", json!(["other"]), "accepted"),
            lesson("proposal", json!(["reference-propagation"]), "proposed")
        ]))
        .unwrap(),
    )
    .unwrap();
    git(&f.repo, &["add", "."]).unwrap();
    git(&f.repo, &["commit", "-m", "approved lesson library"]).unwrap();
    git(
        &f.repo,
        &[
            "update-ref",
            "refs/remotes/origin/main",
            &sha(&f.repo, "HEAD").unwrap(),
        ],
    )
    .unwrap();
    f.cli(
        "configure-loop",
        json!({"policy":{"feedback_file":"feedback.json"}}),
    )
    .unwrap();
    f.bound(false);
    f.pass_review();
    let mut plan = serde_json::to_value(
        f.state().tasks["TASK-1"]
            .loop_state
            .plan
            .as_ref()
            .unwrap()
            .input
            .clone(),
    )
    .unwrap();
    plan["lesson_families"] = json!(["reference-propagation"]);
    f.cli("loop-plan", json!({"key":"TASK-1","plan":plan}))
        .unwrap();
    let state = f.state();
    let lessons = &state.tasks["TASK-1"]
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .feedback
        .lessons;
    assert_eq!(
        lessons.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
        vec!["shared", "transferred"]
    );
    assert!(state.tasks["TASK-1"].review.is_none());
    assert_eq!(state.tasks["TASK-1"].reviews, 1);
    assert_eq!(state.version, 6);
}

#[test]
fn empty_tags_preserve_the_legacy_plan_wire_contract_after_state_rewrite() {
    // Frozen pre-schema6 plan reader. This is a persistence compatibility boundary.
    #[derive(serde::Deserialize, serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyPlan {
        baseline_commit: String,
        family: String,
        deliverable: String,
        components: Vec<String>,
        examples: Vec<String>,
        baselines: Vec<epic_control::loops::BaselineInput>,
    }
    let f = Fixture::new();
    f.bound(false);
    f.evidence();
    let wire = f.cli("status", json!({})).unwrap();
    let plan: LegacyPlan =
        serde_json::from_value(wire["tasks"]["TASK-1"]["loop_state"]["plan"]["input"].clone())
            .unwrap();
    assert_eq!(plan.family, "fixture");
    assert_eq!(plan.baselines[0].criterion, "AC1");
    assert_eq!(wire["version"], 2);
}

#[test]
fn absent_cache_counts_remain_unknown_and_malformed_usage_can_be_recovered() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.cli(
        "agent-finish",
        json!({"launch":id,"session":"implementation-session","outcome":"completed",
        "review":null,"usage":{"input_tokens":100,"output_tokens":10}}),
    )
    .unwrap();
    let report = f.cli("usage-report", json!({})).unwrap();
    assert_eq!(report["tokens"]["input_tokens"], 100);
    assert!(report["tokens"]["cached_input_tokens"].is_null());
    f.checkpoint();
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let receipt = f
        .cli(
            "agent-finish",
            json!({"launch":id,"session":"implementation-session","outcome":"completed",
        "review":null,"usage":{"input_tokens":10,"cached_input_tokens":500,"output_tokens":1}}),
        )
        .unwrap();
    assert!(receipt["usage_warning"].is_string());
    assert!(f.state().launches[1].ended.is_some());
    assert!(f.state().launches[1].usage.is_none());
    let path = session_log(&f, "implementation-session");
    assert!(
        f.cli(
            "usage-import",
            json!({"request":{"session":"implementation-session","artifact":path,
        "start_line":1,"end_line":5,"launch":id,"category":"implementation"}})
        )
        .is_err()
    );
    f.cli(
        "usage-import",
        json!({"request":{"session":"implementation-session","artifact":path,
        "start_line":4,"end_line":5,"launch":id,"category":"implementation"}}),
    )
    .unwrap();
    let report = f.cli("usage-report", json!({})).unwrap();
    assert_eq!(report["tokens"]["input_tokens"], 160);
    assert_eq!(report["tokens"]["output_tokens"], 20);
    assert_eq!(report["launches_without_usage"], json!([]));
    assert!(report["tokens"]["cached_input_tokens"].is_null());
}
