use super::*;
fn refresh(f: &Fixture) {
    let tasks: Vec<_> = f.state().tasks.values().map(|t| t.spec.clone()).collect();
    f.cli("refresh", json!({"tasks":tasks})).unwrap();
}
fn closed_fixture() -> Fixture {
    let f = Fixture::new();
    fs::create_dir_all(f.repo.join("scripts")).unwrap();
    fs::write(
        f.repo.join("scripts/worktree-add.sh"),
        "#!/bin/sh\nset -eu\ngit worktree add --detach \".worktrees/$1\" origin/main\n",
    )
    .unwrap();
    git(&f.repo, &["add", "."]).unwrap();
    git(&f.repo, &["commit", "-m", "helper"]).unwrap();
    git(&f.repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]).unwrap();
    f.bound(false);
    for _ in 0..3 {
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "reviewer", "cancelled", None).unwrap();
    }
    f.changed();
    f.fake_tools();
    f.add_bare_origin();
    let mut old: Value =
        serde_json::from_slice(&fs::read(f.root.join("pr.json")).unwrap()).unwrap();
    old["state"] = json!("CLOSED");
    fs::write(f.root.join("pr.json"), serde_json::to_vec(&old).unwrap()).unwrap();
    fs::write(f.root.join("old.json"), serde_json::to_vec(&old).unwrap()).unwrap();
    fs::write(f.root.join("created"), "yes").unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    hold(&f);
    fs::write(f.root.join("exercise.txt"),"User approves one renewed exercise including independent preparation and cleanup reviews, using the external replacement.").unwrap();
    f.cli(
        "authorize-budget-adjustment",
        adjustment(&f, "additional-cycle", "exercise.txt", json!([])),
    )
    .unwrap();
    fs::write(f.repo.join("source.txt"), "accepted external replacement\n").unwrap();
    git(&f.repo, &["commit", "-am", "external replacement"]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();
    git(&f.repo, &["fetch", "origin"]).unwrap();
    let head = sha(&f.repo, "HEAD").unwrap();
    fs::write(f.root.join("external.json"),serde_json::to_vec(&json!({"number":99,"state":"MERGED","isDraft":false,"headRefOid":head,"headRefName":"external-fix","baseRefName":"main","mergeCommit":{"oid":head}})).unwrap()).unwrap();
    route_history(&f);
    refresh(&f);
    f
}
fn route_history(f: &Fixture) {
    let path = f.root.join("fake-bin/gh");
    let script = fs::read_to_string(&path).unwrap();
    fs::write(path,script.replacen("#!/bin/sh\n","#!/bin/sh\nif test \"$1 $2\" = 'pr view'; then\ncase \"$3\" in\n13) if test -f \"$FAKE_ROOT/saved13.json\"; then cat \"$FAKE_ROOT/saved13.json\"; exit 0; fi;;\n12) cat \"$FAKE_ROOT/old.json\"; exit 0;;\n99) cat \"$FAKE_ROOT/external.json\"; exit 0;;\nesac\nfi\n",1)).unwrap();
}
fn request(f: &Fixture) -> Value {
    let head = sha(&f.repo, "origin/main").unwrap();
    json!({"key":"TASK-1","source":"actual fixture exercise approval including both reviews","artifact":f.root.join("exercise.txt"),"requirements":f.state().tasks["TASK-1"].requirements,"scope":"temporary exercise only","replacement":{"pr":99,"head":head,"merge":head},"cleanup_scope":"remove temporary source control and review full outcome","cleanup_paths":["source.txt"],"cleanup_criteria":[{"id":"CLEAN1","text":"temporary control removed, normal behavior restored; outcome recorded honestly","human_only":false}]})
}
fn recover(f: &Fixture) -> u64 {
    f.cli("recover-superseded", request(f)).unwrap()["delivery"]
        .as_u64()
        .unwrap()
}
fn bind(f: &Fixture, slot: &str) {
    refresh(f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    f.cli("reserve-slot", json!({"slot":slot})).unwrap();
    f.cli(
        "provision-slot",
        json!({"slot":slot,"branch":format!("agent-test-linux/{slot}/TASK-1-exercise")}),
    )
    .unwrap();
}
#[test]
fn closed_recovery_preserves_history_replacement_and_only_transfers_unused_pair() {
    let f = closed_fixture();
    let old = f.state();
    let worker = f.repo.join(".worktrees/worker1");
    fs::write(worker.join("untracked.txt"), "preserve worker").unwrap();
    recover(&f);
    let s = f.state();
    let t = &s.tasks["TASK-1"];
    assert_eq!(t.delivery_history[0].task.pr, Some(12));
    assert!(!t.delivery_history[0].task.main_verified);
    assert!(t.delivery_history[0].task.merged_commit.is_none());
    assert_eq!(t.delivery_history[0].superseded_by.as_ref().unwrap().pr, 99);
    assert_eq!(s.launches.len(), old.launches.len());
    assert_eq!(t.reviews, 3);
    assert!(t.pr.is_none());
    assert!(epic_control::authority::current_pair(&t.authorities).is_some());
    assert_eq!(
        fs::read_to_string(worker.join("untracked.txt")).unwrap(),
        "preserve worker"
    );
    assert!(f.cli("recover-superseded", request(&f)).is_err());
    for pr in [12, 99] {
        let before = f.cli("status", json!({})).unwrap();
        f.cli("gh-observe", json!({"key":"TASK-1","pr":pr}))
            .unwrap();
        assert_eq!(f.cli("status", json!({})).unwrap(), before);
    }
    bind(&f, "worker2");
    full_plan(&f); // No earlier full plan archive change; original task base remains recorded.
    let launch = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(launch, "new-implementer", "completed", None)
        .unwrap();
    assert!(f.begin("implementer").is_err());
}
#[test]
fn recovery_rejects_changed_receipts_partial_pairs_and_wrong_replacement_without_effects() {
    for case in [
        "receipt", "partial", "external", "open", "resolved", "unknown", "stale",
    ] {
        let f = closed_fixture();
        match case {
            "receipt" => fs::write(f.root.join("exercise.txt"), "changed").unwrap(),
            "external" => {
                let p = f.root.join("external.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
                v["state"] = json!("OPEN");
                fs::write(p, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            "open" => {
                let p = f.root.join("old.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
                v["state"] = json!("OPEN");
                fs::write(p, serde_json::to_vec(&v).unwrap()).unwrap();
            }
            _ => {
                // Fixture models independent terminal Jira/process state or a previously consumed native turn.
                let mut store = Store::open(&f.root).unwrap();
                store
                    .change("external-state-fixture", |s| {
                        match case {
                            "partial" => {
                                s.tasks
                                    .get_mut("TASK-1")
                                    .unwrap()
                                    .authorities
                                    .last_mut()
                                    .unwrap()
                                    .used = Some(epic_control::authority::UseId {
                                    implementation: Some(999),
                                    review: None,
                                    receipt_only: false,
                                })
                            }
                            "stale" => {
                                s.tasks
                                    .get_mut("TASK-1")
                                    .unwrap()
                                    .authorities
                                    .last_mut()
                                    .unwrap()
                                    .requirements = "f".repeat(64).parse().unwrap()
                            }
                            "resolved" => s.tasks.get_mut("TASK-1").unwrap().spec.resolved = true,
                            _ => {
                                let id = s.id();
                                s.operations.push(Operation {
                                    id,
                                    key: "TASK-1".into(),
                                    kind: "github".into(),
                                    status: OpStatus::Unknown,
                                    created: now(),
                                    snapshot: None,
                                    attempts: 1,
                                    details: json!({}),
                                    observation: None,
                                    process: None,
                                });
                            }
                        };
                        Ok(json!({}))
                    })
                    .unwrap();
            }
        }
        let before = f.cli("status", json!({})).unwrap();
        assert!(
            f.cli("recover-superseded", request(&f)).is_err(),
            "case {case}"
        );
        assert_eq!(f.cli("status", json!({})).unwrap(), before);
    }
}
fn full_plan(f: &Fixture) {
    let plan = f.state().tasks["TASK-1"].delivery_history[0]
        .task
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .input
        .clone();
    f.cli("loop-plan", json!({"key":"TASK-1","plan":plan}))
        .unwrap();
}
fn preparation(f: &Fixture) -> u64 {
    let delivery = recover(f);
    bind(f, "worker2");
    let t = f.state().tasks["TASK-1"].clone();
    f.cli("begin-stage",json!({"request":{"key":"TASK-1","source":"same user exercise approval","artifact":f.root.join("exercise.txt"),"requirements":t.requirements,"scope":"temporary control only","operational_proof":"post-switch recovery","cleanup_plan":"reserved reviewed cleanup phase","criteria":[{"id":"TMP1","text":"safe temporary fault and restoration plan","human_only":false}]}})).unwrap();
    let base = t.delivery_history[0]
        .task
        .snapshot
        .as_ref()
        .unwrap()
        .base
        .clone();
    f.cli("loop-plan",json!({"key":"TASK-1","plan":{"baseline_commit":base,"family":"fixture","deliverable":"temporary control","components":["source.txt"],"examples":[],"baselines":[{"criterion":"TMP1","starting_condition":"no fault","target":"bounded fault","method":"execute tests","artifact":f.root.join("baseline.txt")}]}})).unwrap();
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    change(f, "worker2", "temporary control\n");
    f.finish(id, "preparation-impl", "completed", None).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(f,"TMP1")]}),
    )
    .unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "preparation-review",
        "completed",
        Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    merge_current(f, 13);
    f.cli("end-stage", json!({"key":"TASK-1"})).unwrap();
    hold(f);
    refresh(f);
    delivery
}
fn change(f: &Fixture, slot: &str, value: &str) {
    let worker = f.repo.join(".worktrees").join(slot);
    fs::write(worker.join("source.txt"), value).unwrap();
    git(&worker, &["commit", "-am", "change source behavior"]).unwrap();
}
fn merge_current(f: &Fixture, number: u64) {
    f.fake_tools();
    for name in ["pr.json", "ready.json", "merged.json"] {
        let p = f.root.join(name);
        let mut pr: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        pr["number"] = json!(number);
        fs::write(p, serde_json::to_vec(&pr).unwrap()).unwrap();
    }
    route_history(f);
    let head = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .head
        .clone();
    git(&f.repo, &["merge", "--ff-only", &head]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();
    let body = f.root.join("body.md");
    fs::write(&body, "Independent proof; full acceptance remains.").unwrap();
    for action in ["create-pr", "ready"] {
        let id = f
            .cli(
                "gh-prepare",
                json!({"key":"TASK-1","action":action,"title":"feat(test): exercise","body":body}),
            )
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        f.cli("gh-run", json!({"operation":id})).unwrap();
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
    if number == 13 {
        fs::copy(f.root.join("merged.json"), f.root.join("saved13.json")).unwrap();
    }
}
fn cleanup_request(f: &Fixture, delivery: u64) -> Value {
    json!({"key":"TASK-1","preparation":delivery,"outcome_artifact":f.root.join("outcome.json")})
}
fn terminal(f: &Fixture) {
    fs::write(f.root.join("outcome.json"),serde_json::to_vec(&json!({"terminal":true,"dispatch":"test://exercise/1","recovery_observation":"Failed exercise; native observations preserved. Required proof remains unpassed."})).unwrap()).unwrap();
}
#[test]
fn same_approval_prepares_cleanup_once_after_stage_and_keeps_full_acceptance_binding() {
    let f = closed_fixture();
    let delivery = preparation(&f);
    let request = cleanup_request(&f, delivery);
    assert!(f.cli("prepare-approved-cleanup", request.clone()).is_err());
    fs::write(f.root.join("outcome.json"), "{\"terminal\":false}").unwrap();
    assert!(f.cli("prepare-approved-cleanup", request.clone()).is_err());
    terminal(&f);
    let prior = f.state();
    f.cli("prepare-approved-cleanup", request.clone()).unwrap();
    let s = f.state();
    let t = &s.tasks["TASK-1"];
    assert_eq!(s.launches.len(), prior.launches.len());
    assert_eq!(t.reviews, 4);
    assert_eq!(t.criteria[0].id, "AC1");
    assert_ne!(t.exercise.as_ref().unwrap().cleanup, Some(delivery));
    assert!(f.cli("prepare-approved-cleanup", request).is_err());
    bind(&f, "worker3");
    full_plan(&f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    change(&f, "worker3", "accepted external replacement\n");
    f.finish(id, "cleanup-impl", "completed", None).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    assert_eq!(
        f.cli("review-readiness", json!({"key":"TASK-1"})).unwrap()["ready"],
        false
    );
    f.pass_review();
    merge_current(&f, 14);
    let head = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .head
        .clone();
    f.cli(
        "final-verify",
        json!({"key":"TASK-1","main_commit":head,"evidence":f.root.join("proof.txt")}),
    )
    .unwrap();
    assert!(f.state().tasks["TASK-1"].final_verified);
    assert_eq!(f.state().tasks["TASK-1"].reviews, 5);
    for pr in [12, 99, 13] {
        f.cli("gh-observe", json!({"key":"TASK-1","pr":pr}))
            .unwrap();
    }
}
#[test]
fn cleanup_rejects_unrelated_intermediate_change_even_when_reverted() {
    let f = closed_fixture();
    let delivery = preparation(&f);
    terminal(&f);
    f.cli("prepare-approved-cleanup", cleanup_request(&f, delivery))
        .unwrap();
    bind(&f, "worker3");
    full_plan(&f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let worker = f.repo.join(".worktrees/worker3");
    fs::write(worker.join("outside.txt"), "unrelated code").unwrap();
    git(&worker, &["add", "."]).unwrap();
    git(&worker, &["commit", "-m", "outside scope"]).unwrap();
    git(&worker, &["revert", "--no-edit", "HEAD"]).unwrap();
    change(&f, "worker3", "accepted external replacement\n");
    f.finish(id, "cleanup-impl", "completed", None).unwrap();
    let before = f.state().tasks["TASK-1"].snapshot.clone();
    assert!(f.cli("snapshot", json!({"key":"TASK-1"})).is_err());
    assert_eq!(f.state().tasks["TASK-1"].snapshot, before);
}
#[test]
fn historical_reopen_and_external_merge_drift_do_not_mutate_current_delivery() {
    let f = closed_fixture();
    recover(&f);
    for (name, pr, field, value) in [
        ("old.json", 12, "state", json!("OPEN")),
        (
            "external.json",
            99,
            "headRefOid",
            json!("0000000000000000000000000000000000000000"),
        ),
    ] {
        let path = f.root.join(name);
        let original = fs::read(&path).unwrap();
        let mut data: Value = serde_json::from_slice(&original).unwrap();
        data[field] = value;
        fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        let before = f.cli("status", json!({})).unwrap();
        assert!(
            f.cli("gh-observe", json!({"key":"TASK-1","pr":pr}))
                .is_err()
        );
        assert_eq!(f.cli("status", json!({})).unwrap(), before);
        fs::write(path, original).unwrap();
    }
}
#[test]
fn failed_exercise_can_remove_controls_under_frozen_cleanup_stage_without_claiming_full_proof() {
    let f = closed_fixture();
    let delivery = preparation(&f);
    terminal(&f);
    f.cli("prepare-approved-cleanup", cleanup_request(&f, delivery))
        .unwrap();
    bind(&f, "worker3");
    let t = f.state().tasks["TASK-1"].clone();
    let plan = t.exercise.as_ref().unwrap();
    let stage = json!({"request":{"key":"TASK-1","source":plan.source,"artifact":plan.artifact,"requirements":plan.requirements,"scope":plan.cleanup_scope,"operational_proof":"failed exercise remains unproved; no second attempt","cleanup_plan":"remove controls and preserve full acceptance hold","criteria":plan.cleanup_criteria}});
    for field in ["scope", "criteria", "artifact"] {
        let mut wrong = stage.clone();
        wrong["request"][field] = match field {
            "criteria" => json!([{"id":"FAKE","text":"weaken cleanup","human_only":false}]),
            "artifact" => {
                fs::write(f.root.join("another.txt"), "another receipt").unwrap();
                json!(f.root.join("another.txt"))
            }
            _ => json!("unrelated work"),
        };
        let before = f.cli("status", json!({})).unwrap();
        assert!(f.cli("begin-stage", wrong).is_err());
        assert_eq!(f.cli("status", json!({})).unwrap(), before);
    }
    f.cli("begin-stage", stage.clone()).unwrap();
    assert!(f.cli("begin-stage", stage).is_err());
    let base = t.delivery_history[0]
        .task
        .snapshot
        .as_ref()
        .unwrap()
        .base
        .clone();
    f.cli("loop-plan",json!({"key":"TASK-1","plan":{"baseline_commit":base,"family":"fixture","deliverable":"remove temporary control","components":["source.txt"],"examples":[],"baselines":[{"criterion":"CLEAN1","starting_condition":"temporary control active","target":"normal source restored","method":"compare exact source and reported outcome","artifact":f.root.join("baseline.txt")}]}})).unwrap();
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    change(&f, "worker3", "accepted external replacement\n");
    f.finish(id, "cleanup-impl", "completed", None).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":snap,"results":[result(&f,"CLEAN1")]}),
    )
    .unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "cleanup-review",
        "completed",
        Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
    merge_current(&f, 14);
    assert!(f.cli("final-verify",json!({"key":"TASK-1","main_commit":snap.head,"evidence":f.root.join("batch-proof.txt")})).is_err());
    f.cli("end-stage", json!({"key":"TASK-1"})).unwrap();
    assert!(f.cli("final-verify",json!({"key":"TASK-1","main_commit":snap.head,"evidence":f.root.join("batch-proof.txt")})).is_err());
    assert_eq!(f.state().tasks["TASK-1"].criteria[0].id, "AC1");
    assert_eq!(f.state().tasks["TASK-1"].reviews, 5);
    assert!(!f.state().tasks["TASK-1"].final_verified);
}
