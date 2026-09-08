use super::*;

#[test]
fn readiness_rejects_missing_evidence_without_spending_review_budget() {
    let f = Fixture::new();
    f.bound(false);
    assert_eq!(
        f.cli("review-readiness", json!({"key":"TASK-1"})).unwrap()["ready"],
        false
    );
    let request = json!({"key":"TASK-1","role":"reviewer","model":"gpt-5.6-sol","effort":"medium"});
    assert!(f.cli("agent-begin", request.clone()).is_err());
    assert!(f.state().launches.is_empty());
    f.evidence();
    assert_eq!(
        f.cli("review-readiness", json!({"key":"TASK-1"})).unwrap()["ready"],
        true
    );
    f.cli("agent-begin", request).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].reviews, 1);
}

#[test]
fn native_pass_receipt_survives_failed_settlement_and_same_launch_can_retry() {
    let f = Fixture::new();
    f.bound(false);
    f.evidence();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    let proof = f.root.join("proof.txt");
    let original = fs::read(&proof).unwrap();
    fs::write(&proof, "accidental artifact replacement").unwrap();
    let review = json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]});
    assert!(
        f.finish(id, "reviewer", "completed", Some(review.clone()))
            .is_err()
    );
    assert!(fs::read_dir(&f.root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("native-review-")
    }));
    fs::write(&proof, original).unwrap();
    f.finish(id, "reviewer", "completed", Some(review)).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].reviews, 1);
    assert!(f.state().launches[0].ended.is_some());
}

fn stage(f: &Fixture) -> Value {
    let auth = f.root.join("stage-approval.txt");
    fs::write(&auth,"User authorizes code merge, then native proof, then mandatory cleanup; unchanged full criteria.").unwrap();
    json!({"request":{"key":"TASK-1","source":"fixture explicit sequencing approval","artifact":auth,
        "requirements":f.state().tasks["TASK-1"].requirements,"scope":"temporary diagnostic code only",
        "operational_proof":"native failure then healthy predecessor restoration","cleanup_plan":"reviewed separate revert, full acceptance afterwards",
        "criteria":[{"id":"STAGE1","text":"temporary code behavior and removal plan","human_only":false}]}})
}
fn stage_plan(f: &Fixture) {
    let base = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .base
        .clone();
    f.cli("loop-plan",json!({"key":"TASK-1","plan":{"baseline_commit":base,"family":"fixture","deliverable":"temporary diagnostic",
        "components":["source.txt"],"examples":[],"baselines":[{"criterion":"STAGE1","starting_condition":"no diagnostic","target":"safe diagnostic",
        "method":"inspect real source and failure boundary","artifact":f.root.join("baseline.txt")}]}})).unwrap();
}
#[test]
fn stage_pass_cannot_complete_full_task_and_end_invalidates_stage_evidence() {
    let f = Fixture::new();
    f.bound(false);
    let full = f.state().tasks["TASK-1"].requirements.clone();
    let old = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let request = stage(&f);
    f.cli("begin-stage", request.clone()).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].requirements, full);
    assert_ne!(
        f.state().tasks["TASK-1"]
            .snapshot
            .as_ref()
            .unwrap()
            .requirements,
        old.requirements
    );
    assert!(f.cli("begin-stage", request).is_err());
    assert!(f.cli("brief",json!({"key":"TASK-1","criteria":[{"id":"AC1","text":"weaker","human_only":false}],"tier":"routine","reason":"replace criteria"})).is_err());
    stage_plan(&f);
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"STAGE1")]}),
    )
    .unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    assert!(f.cli("final-verify",json!({"key":"TASK-1","main_commit":snap.head,"evidence":f.root.join("batch-proof.txt")})).is_err());
    // External merge observation fixture. The no-diff base is already on main.
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("external-merge-fixture", |s| {
            let t = s.tasks.get_mut("TASK-1").unwrap();
            t.main_verified = true;
            t.pr = Some(10);
            t.pr_open = false;
            t.merged_commit = Some(snap.head.clone());
            Ok(json!({}))
        })
        .unwrap();
    drop(store);
    f.cli("end-stage", json!({"key":"TASK-1"})).unwrap();
    let t = &f.state().tasks["TASK-1"];
    assert_eq!(t.criteria[0].id, "AC1");
    assert!(t.review.is_none());
    assert!(t.evidence.is_empty());
    assert!(t.stages[0].review.is_some());
    assert_eq!(t.reviews, 1);
    f.loop_plan();
    f.evidence();
    let full_snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(json!({"snapshot":full_snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    f.cli(
        "final-verify",
        json!({"key":"TASK-1","main_commit":full_snap.head,"evidence":f.root.join("proof.txt")}),
    )
    .unwrap();
    assert!(f.state().tasks["TASK-1"].final_verified);
}
#[test]
fn changed_stage_authority_and_pending_stage_block_followup_and_launch() {
    let f = Fixture::new();
    f.bound(false);
    f.cli("begin-stage", stage(&f)).unwrap();
    stage_plan(&f);
    fs::write(f.root.join("stage-approval.txt"), "changed approval").unwrap();
    assert!(f.begin("implementer").is_err());
    assert!(f.state().launches.is_empty());
    assert!(f.cli("end-stage", json!({"key":"TASK-1"})).is_err());
}
#[test]
fn bound_usage_imports_on_finish_and_sync_never_counts_later_turns() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let path = session_log(&f, "implementation-session");
    f.cli("usage-bind",json!({"launch":id,"source":{"session":"implementation-session","artifact":path,"start_line":4,"end_line":null}})).unwrap();
    f.finish(id, "implementation-session", "completed", None)
        .unwrap();
    assert_eq!(
        f.state().launches[0].usage.as_ref().unwrap()["input_tokens"],
        60
    );
    use std::io::Write;
    writeln!(fs::OpenOptions::new().append(true).open(path).unwrap(),"\n{}",json!({"type":"token_usage_record","payload":{"last_thread_token_usage":{"input_tokens":900,"cached_input_tokens":800,"output_tokens":90}}})).unwrap();
    let sync = f.cli("usage-sync", json!({})).unwrap();
    assert_eq!(sync["usage"]["tokens"]["input_tokens"], 60);
    assert_eq!(sync["usage"]["imports"], 1);
}
#[test]
fn missing_usage_does_not_block_settlement_and_empty_report_marks_unknown() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let receipt = f
        .finish(id, "implementation-session", "completed", None)
        .unwrap();
    assert!(receipt["usage_warning"].is_string());
    assert!(f.state().launches[0].ended.is_some());
    let sync = f.cli("usage-sync", json!({})).unwrap();
    assert_eq!(sync["unavailable"][0]["launch"], id);
}
#[test]
fn reconciliation_reuses_complete_prs_but_selects_uncertain_operations() {
    let f = Fixture::new();
    f.bound(false);
    // A completed delivery receipt may outlive its worker. No filesystem fixture needed.
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("completed-receipt-fixture", |s| {
            let t = s.tasks.get_mut("TASK-2").unwrap();
            t.main_verified = true;
            t.final_verified = true;
            t.pr = Some(9);
            t.pr_open = false;
            t.draft = false;
            t.delivery = Delivery::Merged;
            t.sync = Sync::Confirmed;
            t.spec.jira_status = "4".into();
            t.spec.resolved = true;
            t.merged_commit = Some(sha(&s.repo, "HEAD")?.parse()?);
            Ok(json!({}))
        })
        .unwrap();
    drop(store);
    let plan = f.cli("reconcile-plan", json!({"key":"TASK-1"})).unwrap();
    assert!(plan["inspect"]["TASK-1"].is_array());
    assert!(plan["inspect"]["TASK-2"].is_null());
    assert_eq!(plan["reuse_completed"][0]["pr"], 9);
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("unknown-operation-fixture", |s| {
            let id = s.id();
            s.operations.push(Operation {
                id,
                key: "TASK-2".into(),
                kind: "github".into(),
                status: OpStatus::Unknown,
                created: now(),
                snapshot: None,
                attempts: 1,
                details: json!({}),
                observation: None,
                process: None,
            });
            Ok(json!({}))
        })
        .unwrap();
    drop(store);
    let plan = f.cli("reconcile-plan", json!({})).unwrap();
    assert!(plan["inspect"]["TASK-2"].is_array());
    assert_eq!(plan["reuse_completed"], json!([]));
}

fn merge_stage_fixture(f: &Fixture) {
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(f,"STAGE1")]}),
    )
    .unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    // External merge fixture uses the already-on-main no-diff snapshot.
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("external-stage-merge", |s| {
            let t = s.tasks.get_mut("TASK-1").unwrap();
            t.main_verified = true;
            t.pr = Some(10);
            t.pr_open = false;
            t.merged_commit = Some(snap.head.clone());
            Ok(json!({}))
        })
        .unwrap();
    drop(store);
    f.cli("end-stage", json!({"key":"TASK-1"})).unwrap();
}
#[test]
fn staged_last_review_and_extra_stage_cycle_recover_with_fresh_final_review_authority() {
    for previous in [2, 3] {
        let f = Fixture::new();
        f.bound(false);
        for _ in 0..previous {
            let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
            f.finish(id, "reviewer", "cancelled", None).unwrap();
        }
        f.cli("begin-stage", stage(&f)).unwrap();
        stage_plan(&f);
        if previous == 3 {
            hold(&f);
            fs::write(
                f.root.join("stage-cycle.txt"),
                "User authorizes an extra stage review",
            )
            .unwrap();
            f.cli(
                "authorize-budget-adjustment",
                adjustment(&f, "additional-cycle", "stage-cycle.txt", json!([])),
            )
            .unwrap();
            resume(&f);
        }
        merge_stage_fixture(&f);
        f.loop_plan();
        f.evidence();
        assert!(f.begin("reviewer").is_err());
        hold(&f);
        fs::write(
            f.root.join("final-cycle.txt"),
            "User authorizes one independent final proof review",
        )
        .unwrap();
        let request = adjustment(&f, "additional-cycle", "final-cycle.txt", json!([]));
        f.cli("authorize-budget-adjustment", request.clone())
            .unwrap();
        assert!(
            f.cli("authorize-budget-adjustment", request.clone())
                .is_err()
        );
        resume(&f);
        assert!(f.begin("implementer").is_err());
        let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(
            id,
            "reviewer",
            "completed",
            Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
        )
        .unwrap();
        assert_eq!(f.state().tasks["TASK-1"].reviews, previous + 2);
        assert!(f.begin("reviewer").is_err());
        f.cli(
            "final-verify",
            json!({"key":"TASK-1","main_commit":snap.head,"evidence":f.root.join("proof.txt")}),
        )
        .unwrap();
        assert!(f.state().tasks["TASK-1"].final_verified);
    }
}

#[test]
#[cfg(target_os = "linux")]
fn staged_github_delivery_merges_only_the_reviewed_stage_and_keeps_full_acceptance_open() {
    let f = Fixture::new();
    f.bound(false);
    f.cli("begin-stage", stage(&f)).unwrap();
    stage_plan(&f);
    f.changed();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"STAGE1")]}),
    )
    .unwrap();
    let id = f
        .cli(
            "agent-begin",
            json!({"key":"TASK-1","role":"reviewer","model":"gpt-5.6-sol","effort":"medium"}),
        )
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    f.fake_tools();
    git(&f.repo, &["merge", "--ff-only", &snap.head]).unwrap();
    f.add_bare_origin();
    let body = f.root.join("body.md");
    fs::write(
        &body,
        "Temporary stage acceptance; runtime proof and cleanup remain.",
    )
    .unwrap();
    for action in ["create-pr", "ready"] {
        let id=f.cli("gh-prepare",json!({"key":"TASK-1","action":action,"title":"feat(test): temporary exercise","body":body})).unwrap()["id"].as_u64().unwrap();
        f.cli("gh-run", json!({"operation":id})).unwrap();
        if action == "create-pr" {
            f.cli("jira-prepare",json!({"key":"TASK-1","issue":"TASK-1","current":"3","resolved":false,"target":"3","transitions":[]})).unwrap();
        }
    }
    f.cli("poll-checks", json!({"key":"TASK-1"})).unwrap();
    let id = f
        .cli(
            "gh-prepare",
            json!({"key":"TASK-1","action":"merge","method":"merge"}),
        )
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    f.cli("gh-run", json!({"operation":id})).unwrap();
    assert!(f.state().tasks["TASK-1"].main_verified);
    assert!(f.cli("complete", json!({"key":"TASK-1"})).is_err());
    assert!(f.cli("jira-prepare",json!({"key":"TASK-1","issue":"TASK-1","current":"3","resolved":false,"target":"4","transitions":[{"id":"done","to":"4"}]})).is_err());
    f.cli("end-stage", json!({"key":"TASK-1"})).unwrap();
    let task = &f.state().tasks["TASK-1"];
    assert_eq!(task.stages[0].merge.as_deref(), Some(snap.head.as_ref()));
    assert_eq!(task.criteria[0].id, "AC1");
    assert!(!task.final_verified);
    assert!(
        f.cli(
            "final-verify",
            json!({"key":"TASK-1","main_commit":snap.head,"evidence":body})
        )
        .is_err()
    );
}
