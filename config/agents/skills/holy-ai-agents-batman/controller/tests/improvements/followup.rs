//! External merge and removal are real Git/GitHub observations, not fabricated
//! controller success. Every normal transition uses a fresh JSON CLI process.
use super::*;

fn refresh(f: &Fixture) {
    let tasks: Vec<_> = f.state().tasks.values().map(|t| t.spec.clone()).collect();
    f.cli("refresh", json!({"tasks":tasks})).unwrap();
}
fn request(f: &Fixture) -> Value {
    json!({"key":"TASK-1","source":"actual test user approval of one diagnostic repair and review",
        "artifact":f.root.join("followup-approval.txt"),"scope":"Correct diagnostic; all operational and cleanup criteria remain binding",
        "requirements":f.state().tasks["TASK-1"].requirements})
}
fn budget(f: &Fixture) -> Value {
    json!({"key":"TASK-1","mode":"additional-cycle","source":"actual test user approval",
        "artifact":f.root.join("followup-approval.txt"),"scope":"One diagnostic implementation/review pair; full criteria unchanged","paths":[]})
}
fn unchanged_rejection(f: &Fixture, command: &str, input: Value) {
    let before = f.cli("status", json!({})).unwrap();
    assert!(
        f.cli(command, input).is_err(),
        "{command} unexpectedly accepted"
    );
    assert_eq!(f.cli("status", json!({})).unwrap(), before);
}
fn held_merge(missing: bool) -> Fixture {
    let f = Fixture::new();
    fs::create_dir_all(f.repo.join("scripts")).unwrap();
    fs::write(
        f.repo.join("scripts/worktree-add.sh"),
        "#!/bin/sh\nset -eu\ngit worktree add --detach \".worktrees/$1\" origin/main\n",
    )
    .unwrap();
    git(&f.repo, &["add", "."]).unwrap();
    git(&f.repo, &["commit", "-m", "fixture helper"]).unwrap();
    git(&f.repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]).unwrap();
    f.bound(false);
    full_requirements(&f);
    let jira=f.cli("jira-prepare",json!({"key":"TASK-1","issue":"TASK-1","current":"1","resolved":false,"target":"2","transitions":[{"id":"progress","to":"2"}]})).unwrap()["id"].as_u64().unwrap();
    f.cli("dispatch", json!({"operation":jira})).unwrap();
    f.cli("jira-observe",json!({"operation":jira,"current":"2","resolved":false,"evidence":"fresh fixture connector observation"})).unwrap();
    let review = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(review, "reviewer", "cancelled", None).unwrap();
    let implementation = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(implementation, "implementer", "completed", None)
        .unwrap();
    f.changed();
    historical_reviews(&f);
    let head = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .head
        .clone();
    git(&f.repo, &["merge", "--ff-only", &head]).unwrap();
    f.add_bare_origin();
    f.fake_tools();
    fs::write(f.root.join("created"), "exists").unwrap();
    fs::copy(f.root.join("merged.json"), f.root.join("pr.json")).unwrap();
    let body = f.root.join("body.md");
    fs::write(&body, "unfinished diagnostic work").unwrap();
    // An external actor merged the PR after create was prepared. Observation
    // settles that uncertain identity, but cannot satisfy task acceptance.
    f.cli(
        "gh-prepare",
        json!({"key":"TASK-1","action":"create-pr","title":"diagnostic","body":body}),
    )
    .unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    // Checks may still be pending after an external merge. Record the old
    // per-head wait deadline through the actual polling command.
    let gh_path = f.root.join("fake-bin/gh");
    let gh_script = fs::read_to_string(&gh_path).unwrap();
    fs::write(
        &gh_path,
        gh_script.replace("\"bucket\":\"pass\"", "\"bucket\":\"pending\""),
    )
    .unwrap();
    f.cli("poll-checks", json!({"key":"TASK-1"})).unwrap();
    fs::write(&gh_path, gh_script).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].deadlines.len(), 1);
    f.cli("hold", json!({"key":"TASK-1","outcome":"needs-human","reason":"operational proof and cleanup remain"})).unwrap();
    if missing {
        git(
            &f.repo,
            &[
                "worktree",
                "remove",
                f.repo.join(".worktrees/worker1").to_str().unwrap(),
            ],
        )
        .unwrap();
    }
    fs::write(f.root.join("followup-approval.txt"), "Current user: one further diagnostic correction and review; full task criteria remain binding.").unwrap();
    refresh(&f);
    f
}
fn prepare(f: &Fixture) {
    let before = f.state();
    let result = f.cli("prepare-followup", request(f)).unwrap();
    assert_eq!(result["additional_launches"], 0);
    let after = f.state();
    assert_eq!(after.version, 7);
    assert_eq!(
        serde_json::to_value(&before.launches).unwrap(),
        serde_json::to_value(&after.launches).unwrap()
    );
    let t = &after.tasks["TASK-1"];
    assert_eq!((t.reviews, t.repairs), (3, 1));
    assert_eq!(t.delivery_history[0].task.pr, Some(12));
    assert!(t.delivery_history[0].task.main_verified);
    assert_eq!(t.pr, None);
    assert!(!t.main_verified && !t.final_verified);
    assert_eq!(t.deadlines, before.tasks["TASK-1"].deadlines);
    assert_eq!(
        serde_json::to_value(&t.delivery_history[0].task).unwrap(),
        serde_json::to_value(&before.tasks["TASK-1"]).unwrap()
    );
    assert!(t.evidence.is_empty() && t.loop_state.measurements.is_empty() && t.review.is_none());
}
fn bind(f: &Fixture) {
    refresh(f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    assert_eq!(
        f.cli("next", json!({})).unwrap()["action"],
        "reserve-and-bind-slot"
    );
    f.cli("reserve-slot", json!({"slot":"worker2"})).unwrap();
    f.cli(
        "provision-slot",
        json!({"slot":"worker2","branch":"agent-test-linux/worker2/TASK-1-followup"}),
    )
    .unwrap();
    let s = f.state();
    let plan = &s.tasks["TASK-1"].delivery_history[0]
        .task
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .input;
    assert_ne!(
        plan.baseline_commit,
        s.tasks["TASK-1"].snapshot.as_ref().unwrap().base.as_ref()
    );
    f.cli("loop-plan", json!({"key":"TASK-1","plan":plan}))
        .unwrap();
}
fn change_followup(f: &Fixture) {
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let worker = f.repo.join(".worktrees/worker2");
    fs::write(
        worker.join("source.txt"),
        "correct diagnostic; operational proof and cleanup measured separately\n",
    )
    .unwrap();
    git(&worker, &["add", "."]).unwrap();
    git(&worker, &["commit", "-m", "fix(test): diagnostic"]).unwrap();
    f.finish(id, "followup-implementer", "completed", None)
        .unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    unchanged_rejection(
        f,
        "agent-begin",
        json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium"}),
    );
}
fn followup_github(f: &Fixture) {
    use std::os::unix::fs::PermissionsExt;
    fs::copy(f.root.join("pr.json"), f.root.join("historical.json")).unwrap();
    f.fake_tools();
    fs::remove_file(f.root.join("created")).unwrap();
    for name in ["pr.json", "ready.json", "merged.json"] {
        let path = f.root.join(name);
        let mut pr: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        pr["number"] = json!(13);
        fs::write(path, serde_json::to_vec(&pr).unwrap()).unwrap();
    }
    let path = f.root.join("fake-bin/gh");
    let script = fs::read_to_string(&path).unwrap();
    let script = script.replace("case \"$1 $2\" in", "if test \"$1 $2 $3\" = 'pr view 12'; then cat \"$FAKE_ROOT/historical.json\"; exit 0; fi\ncase \"$1 $2\" in");
    fs::write(&path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn followup_missing_worker_runs_one_pair_and_publishes_without_reusing_history() {
    let f = held_merge(true);
    prepare(&f);
    unchanged_rejection(&f, "prepare-followup", request(&f));
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    unchanged_rejection(&f, "authorize-budget-adjustment", budget(&f));
    bind(&f);
    unchanged_rejection(&f, "gh-observe", json!({"key":"TASK-1"}));
    change_followup(&f);
    full_evidence(&f, &[]);
    review_pass(&f);
    followup_github(&f);
    let op = f.cli("gh-prepare",json!({"key":"TASK-1","action":"create-pr","title":"fix(test): diagnostic","body":f.root.join("body.md")})).unwrap()["id"].as_u64().unwrap();
    let before = f.cli("status", json!({})).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1","pr":12}))
        .unwrap();
    assert_eq!(f.cli("status", json!({})).unwrap(), before);
    let old = f.state().tasks["TASK-1"].delivery_history[0].operations[0];
    let old_gh = f
        .state()
        .operations
        .iter()
        .find(|o| o.kind == "github")
        .unwrap()
        .id;
    unchanged_rejection(&f, "gh-run", json!({"operation":old_gh}));
    unchanged_rejection(&f, "dispatch", json!({"operation":old}));
    unchanged_rejection(
        &f,
        "jira-observe",
        json!({"operation":old,"current":"2","resolved":false,"evidence":"old transition replay"}),
    );

    f.cli("gh-run", json!({"operation":op})).unwrap();
    let s = f.state();
    assert_eq!(s.tasks["TASK-1"].pr, Some(13));
    assert_eq!(s.tasks["TASK-1"].delivery_history[0].task.pr, Some(12));
    assert_eq!(
        (s.tasks["TASK-1"].reviews, s.tasks["TASK-1"].repairs),
        (4, 2)
    );
    assert_eq!(s.launches.len(), 6);
    land_followup(&f);
    unchanged_rejection(&f, "complete", json!({"key":"TASK-1"}));
    unchanged_rejection(
        &f,
        "agent-begin",
        json!({"key":"TASK-1","role":"reviewer","model":"gpt-5.6-sol","effort":"medium"}),
    );
}

#[test]
fn followup_preparation_rejects_missing_receipt_requirements_live_turn_and_unknown_operation() {
    let f = held_merge(true);
    let mut bad = request(&f);
    bad["artifact"] = json!(f.root.join("absent"));
    unchanged_rejection(&f, "prepare-followup", bad);
    let mut bad = request(&f);
    bad["requirements"] = json!("weaker criteria");
    unchanged_rejection(&f, "prepare-followup", bad);
    // Fault injection represents a crashed controller with an unresolved launch
    // or operation. Normal commands cannot manufacture those after a hold.
    for field in ["launch", "operation", "completed", "terminal"] {
        let original = serde_json::to_value(f.state()).unwrap();
        let mut state = original.clone();
        match field {
            "launch" => state["launches"][0]["ended"] = Value::Null,
            "operation" => state["operations"][0]["status"] = json!("unknown"),
            "completed" => state["tasks"]["TASK-1"]["final_verified"] = json!(true),
            _ => state["tasks"]["TASK-1"]["was_terminal"] = json!(true),
        }
        write_state(&f, &state);
        unchanged_rejection(&f, "prepare-followup", request(&f));
        write_state(&f, &original);
    }
    prepare(&f);
}
fn write_state(f: &Fixture, value: &Value) {
    rusqlite::Connection::open(f.root.join("state.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE state SET data=?1 WHERE id=1",
            [serde_json::to_string(value).unwrap()],
        )
        .unwrap();
}

#[test]
fn followup_preparation_does_not_grant_launches_and_changed_receipt_blocks_launch() {
    let f = held_merge(true);
    prepare(&f);
    bind(&f);
    unchanged_rejection(
        &f,
        "agent-begin",
        json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium"}),
    );
    f.cli(
        "hold",
        json!({"key":"TASK-1","outcome":"needs-human","reason":"record actual pair authority"}),
    )
    .unwrap();
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    refresh(&f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    fs::write(f.root.join("followup-approval.txt"), "changed approval").unwrap();
    unchanged_rejection(
        &f,
        "agent-begin",
        json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium"}),
    );
}

#[test]
fn followup_preserves_dirty_old_worker_and_refuses_historical_or_foreign_replacement() {
    let f = held_merge(false);
    let old = f.repo.join(".worktrees/worker1/source.txt");
    fs::write(&old, "unmerged local work must survive").unwrap();
    prepare(&f);
    refresh(&f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker1"}));
    let reservations = f.repo.join(".local/epic-control/slots");
    fs::write(reservations.join("worker2"), "another-run").unwrap();
    unchanged_rejection(&f, "reserve-slot", json!({"slot":"worker2"}));
    assert_eq!(
        fs::read_to_string(reservations.join("worker2")).unwrap(),
        "another-run"
    );
    assert_eq!(
        fs::read_to_string(old).unwrap(),
        "unmerged local work must survive"
    );
}

fn prepared_historical_slot_reuse() -> (Fixture, String, String) {
    let f = held_merge(true);
    prepare(&f);
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    let old_prepared_base = f.state().tasks["TASK-1"]
        .followup
        .as_ref()
        .unwrap()
        .base
        .clone();
    fs::write(
        f.repo.join("source.txt"),
        "main advanced after follow-up preparation\n",
    )
    .unwrap();
    git(&f.repo, &["add", "source.txt"]).unwrap();
    git(&f.repo, &["commit", "-m", "advance main after preparation"]).unwrap();
    let new_base = sha(&f.repo, "HEAD").unwrap();
    git(
        &f.repo,
        &["update-ref", "refs/remotes/origin/main", &new_base],
    )
    .unwrap();
    refresh(&f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    f.cli("reserve-slot", json!({"slot":"worker2"})).unwrap();
    let path = f.repo.join(".worktrees/worker2");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "agent-test-linux/worker2/TASK-1-followup",
            path.to_str().unwrap(),
            "origin/main",
        ],
    )
    .unwrap();
    // This fixture models a completed delivery record retained from an earlier
    // normal controller cycle. Replaying that whole cycle would obscure the
    // public binding behavior under test.
    let mut state = f.state();
    let owner = state.tasks.get_mut("TASK-2").unwrap();
    owner.delivery = Delivery::Merged;
    owner.sync = Sync::Confirmed;
    owner.started_by_run = true;
    owner.slot = Some("worker2".into());
    owner.main_verified = true;
    owner.merged_commit = Some(new_base.parse().unwrap());
    write_state(&f, &serde_json::to_value(state).unwrap());
    fs::write(
        f.root.join("slot-reuse-approval.txt"),
        "Current user authorizes TASK-1 to reuse TASK-2's completed worker2 slot",
    )
    .unwrap();
    (f, old_prepared_base.to_string(), new_base.to_string())
}

fn slot_reuse_request(f: &Fixture) -> Value {
    json!({
        "key":"TASK-1",
        "slot":"worker2",
        "historical_slot_reuse":{
            "historical_key":"TASK-2",
            "source":"actual current-user worker-slot exception",
            "artifact":f.root.join("slot-reuse-approval.txt")
        }
    })
}

#[test]
fn explicit_historical_slot_reuse_refreshes_only_unstarted_delivery_state() {
    let (f, old_prepared_base, new_base) = prepared_historical_slot_reuse();
    let before = f.state();
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker2"}));

    f.cli("bind", slot_reuse_request(&f)).unwrap();
    let after = f.state();
    let task = &after.tasks["TASK-1"];
    let receipt = task.historical_slot_reuse.as_ref().unwrap();
    assert_eq!(task.slot.as_deref(), Some("worker2"));
    assert_eq!(receipt.key.as_ref(), "TASK-1");
    assert_eq!(receipt.slot.as_ref(), "worker2");
    assert_eq!(receipt.historical_key.as_ref(), "TASK-2");
    assert_eq!(receipt.previous_prepared_base, old_prepared_base);
    assert_eq!(receipt.refreshed_base, new_base);
    assert_eq!(task.followup.as_ref().unwrap().base, new_base);
    assert_eq!(task.snapshot.as_ref().unwrap().base, new_base);
    assert_eq!(
        (task.reviews, task.repairs),
        (
            before.tasks["TASK-1"].reviews,
            before.tasks["TASK-1"].repairs
        )
    );
    assert_eq!(
        epic_control::followup::original_base(task).unwrap(),
        epic_control::followup::original_base(&before.tasks["TASK-1"]).unwrap()
    );

    let plan = task.delivery_history[0]
        .task
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .input
        .clone();
    f.cli("loop-plan", json!({"key":"TASK-1","plan":plan}))
        .unwrap();
    fs::write(f.root.join("slot-reuse-approval.txt"), "changed receipt").unwrap();
    unchanged_rejection(
        &f,
        "agent-begin",
        json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium"}),
    );
}

#[test]
fn historical_slot_exception_rejects_unfinished_owner_or_started_prepared_work() {
    let (f, _, _) = prepared_historical_slot_reuse();
    let pristine = f.cli("status", json!({})).unwrap();
    for condition in ["unfinished-owner", "evidence", "launch"] {
        let mut state = pristine.clone();
        match condition {
            "unfinished-owner" => {
                state["tasks"]["TASK-2"]["delivery"] = json!("active");
                state["tasks"]["TASK-2"]["main_verified"] = json!(false);
            }
            "evidence" => {
                state["tasks"]["TASK-1"]["evidence"] =
                    state["tasks"]["TASK-1"]["delivery_history"][0]["task"]["evidence"].clone();
            }
            _ => {
                let mut launch = state["launches"][0].clone();
                launch["id"] = json!(state["serial"].as_u64().unwrap() + 1);
                launch["key"] = json!("TASK-1");
                launch["ended"] = json!(1);
                state["launches"].as_array_mut().unwrap().push(launch);
            }
        }
        write_state(&f, &state);
        unchanged_rejection(&f, "bind", slot_reuse_request(&f));
    }
    write_state(&f, &pristine);
    assert!(f.state().tasks["TASK-1"].slot.is_none());
}

#[test]
fn followup_old_schema_read_does_not_migrate_and_new_schema_rejects_future_versions() {
    let f = held_merge(true);
    let mut old = serde_json::to_value(f.state()).unwrap();
    old["version"] = json!(6);
    for t in old["tasks"].as_object_mut().unwrap().values_mut() {
        t.as_object_mut().unwrap().remove("followup");
        t.as_object_mut().unwrap().remove("delivery_history");
    }
    write_state(&f, &old);
    assert_eq!(f.state().version, 6);
    let stored: String = rusqlite::Connection::open(f.root.join("state.sqlite3"))
        .unwrap()
        .query_row("SELECT data FROM state WHERE id=1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(), old);
    prepare(&f);
    let mut future = serde_json::to_value(f.state()).unwrap();
    future["version"] = json!(11);
    write_state(&f, &future);
    assert!(f.cli("status", json!({})).is_err());
    assert!(f.cli("resume", json!({"key":"TASK-1"})).is_err());
}

fn full_requirements(f: &Fixture) {
    let criteria: Vec<_> = (1..=7).map(|i| json!({"id":format!("AC{i}"),
        "text":match i {3 => "Native operational recovery proof",6 => "Temporary exercise code removed",_ => "Preserve cumulative task behavior"},
        "human_only":false})).collect();
    f.cli("brief",json!({"key":"TASK-1","criteria":criteria,"tier":"routine","reason":"isolated fixture behavior"})).unwrap();
    let state = f.state();
    let mut plan = serde_json::to_value(
        &state.tasks["TASK-1"]
            .loop_state
            .plan
            .as_ref()
            .unwrap()
            .input,
    )
    .unwrap();
    let baseline = plan["baselines"][0].clone();
    plan["baselines"] = json!(
        (1..=7)
            .map(|i| {
                let mut b = baseline.clone();
                b["criterion"] = json!(format!("AC{i}"));
                b
            })
            .collect::<Vec<_>>()
    );
    f.cli("loop-plan", json!({"key":"TASK-1","plan":plan}))
        .unwrap();
}
fn full_evidence(f: &Fixture, omit: &[&str]) {
    let state = f.state();
    let proof = f.root.join("current-proof.txt");
    fs::write(&proof,"Disposable fixture current operational and cleanup proof, measured against original task base").unwrap();
    let results: Vec<_> = state.tasks["TASK-1"].criteria.iter().filter(|c| !omit.contains(&c.id.as_str())).map(|c| json!({
        "observed":"Current behavior checked against unchanged original task baseline; no automatic historical evidence reuse",
        "satisfied":true,"evidence":{"criterion":c.id,"status":"passed","kind":"inspection","artifact":proof,
        "implementation":["source.txt"],"description":c.text,"human_source":null}})).collect();
    f.cli(
        "evidence-batch",
        json!({"key":"TASK-1","snapshot":state.tasks["TASK-1"].snapshot,"results":results}),
    )
    .unwrap();
}
fn review_pass(f: &Fixture) {
    let snapshot = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "followup-reviewer",
        "completed",
        Some(json!({"snapshot":snapshot,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
    )
    .unwrap();
}

#[test]
fn followup_merge_does_not_complete_operational_or_cleanup_acceptance() {
    let f = held_merge(true);
    prepare(&f);
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    bind(&f);
    change_followup(&f);
    full_evidence(&f, &["AC3", "AC6"]);
    let snapshot = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    unchanged_rejection(
        &f,
        "agent-finish",
        json!({"launch":id,"session":"reviewer","outcome":"completed",
        "review":{"snapshot":snapshot,"verdict":"PASS","findings":[],"evidence_gaps":[]},"usage":null}),
    );
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(
            json!({"snapshot":snapshot,"verdict":"BLOCKED","findings":[],
        "evidence_gaps":["AC3 operational proof and AC6 temporary-code removal remain"]}),
        ),
    )
    .unwrap();
    followup_github(&f);
    fs::write(f.root.join("created"), "external PR").unwrap();
    git(&f.repo, &["merge", "--ff-only", &snapshot.head]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();
    fs::copy(f.root.join("merged.json"), f.root.join("pr.json")).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].delivery, Delivery::NeedsHuman);
    assert_eq!(f.state().tasks["TASK-1"].pr, Some(13));
    unchanged_rejection(
        &f,
        "final-verify",
        json!({"key":"TASK-1","main_commit":snapshot.head,"evidence":f.root.join("current-proof.txt")}),
    );
    unchanged_rejection(
        &f,
        "jira-prepare",
        json!({"key":"TASK-1","issue":"TASK-1","current":"2","resolved":false,"target":"4","transitions":[{"id":"done","to":"4"}]}),
    );
    unchanged_rejection(&f, "complete", json!({"key":"TASK-1"}));
    let state = f.cli("status", json!({})).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1","pr":12}))
        .unwrap();
    assert_eq!(f.cli("status", json!({})).unwrap(), state);
}

#[test]
fn followup_final_verify_uses_current_merge_and_full_cumulative_criteria() {
    let f = held_merge(true);
    prepare(&f);
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    bind(&f);
    change_followup(&f);
    full_evidence(&f, &[]);
    review_pass(&f);
    followup_github(&f);
    let state = f.state();
    let task = &state.tasks["TASK-1"];
    let head = task.snapshot.as_ref().unwrap().head.clone();
    let old = task.delivery_history[0].task.merged_commit.clone().unwrap();
    git(&f.repo, &["merge", "--ff-only", &head]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();
    fs::write(f.root.join("created"), "external PR").unwrap();
    fs::copy(f.root.join("merged.json"), f.root.join("pr.json")).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    unchanged_rejection(
        &f,
        "final-verify",
        json!({"key":"TASK-1","main_commit":old,"evidence":f.root.join("current-proof.txt")}),
    );
    f.cli(
        "final-verify",
        json!({"key":"TASK-1","main_commit":head,"evidence":f.root.join("current-proof.txt")}),
    )
    .unwrap();
    f.cli("jira-prepare",json!({"key":"TASK-1","issue":"TASK-1","current":"4","resolved":true,"target":"4","transitions":[]})).unwrap();
    f.cli("complete", json!({"key":"TASK-1"})).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].delivery, Delivery::Merged);
    assert!(f.state().tasks["TASK-1"].final_verified);
    unchanged_rejection(&f, "prepare-followup", request(&f));
}

fn land_followup(f: &Fixture) {
    f.cli("jira-prepare",json!({"key":"TASK-1","issue":"TASK-1","current":"3","resolved":false,"target":"3","transitions":[]})).unwrap();
    let ready = f
        .cli("gh-prepare", json!({"key":"TASK-1","action":"ready"}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    f.cli("gh-run", json!({"operation":ready})).unwrap();
    f.cli("poll-checks", json!({"key":"TASK-1"})).unwrap();
    let head = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .head
        .clone();
    git(&f.repo, &["merge", "--ff-only", &head]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();
    let merge = f
        .cli(
            "gh-prepare",
            json!({"key":"TASK-1","action":"merge","method":"merge"}),
        )
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    f.cli("gh-run", json!({"operation":merge})).unwrap();
    let task = f.state().tasks["TASK-1"].clone();
    assert!(task.main_verified && !task.final_verified);
    assert_eq!(task.merged_commit, Some(head));
    let before = f.cli("status", json!({})).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1","pr":12}))
        .unwrap();
    assert_eq!(f.cli("status", json!({})).unwrap(), before);
}

#[test]
fn followup_rejects_symlink_foreign_dirty_and_other_task_slot_without_effects() {
    let f = held_merge(true);
    prepare(&f);
    refresh(&f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    f.cli("reserve-slot", json!({"slot":"worker2"})).unwrap();
    let path = f.repo.join(".worktrees/worker2");
    std::os::unix::fs::symlink(&f.repo, &path).unwrap();
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker2"}));
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    git(&path, &["init"]).unwrap();
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker2"}));
    fs::remove_dir_all(&path).unwrap();
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "agent-test-linux/worker2/TASK-1-followup",
            path.to_str().unwrap(),
            "origin/main",
        ],
    )
    .unwrap();
    fs::write(path.join("source.txt"), "dirty replacement").unwrap();
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker2"}));
    assert_eq!(
        fs::read_to_string(path.join("source.txt")).unwrap(),
        "dirty replacement"
    );
    git(&path, &["restore", "source.txt"]).unwrap();
    // Represents another task's durable reservation while recovering a crash.
    let original = f.cli("status", json!({})).unwrap();
    let mut state = original.clone();
    state["tasks"]["TASK-2"]["slot"] = json!("worker2");
    write_state(&f, &state);
    unchanged_rejection(&f, "bind", json!({"key":"TASK-1","slot":"worker2"}));
    write_state(&f, &original);
    f.cli("bind", json!({"key":"TASK-1","slot":"worker2"}))
        .unwrap();
}

#[test]
fn followup_failed_pair_consumes_authority_and_next_pair_requires_fresh_receipt() {
    let f = held_merge(true);
    prepare(&f);
    f.cli("authorize-budget-adjustment", budget(&f)).unwrap();
    bind(&f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "implementer", "failed", None).unwrap();
    f.checkpoint();
    let review = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(review, "reviewer", "cancelled", None).unwrap();
    unchanged_rejection(
        &f,
        "agent-begin",
        json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium"}),
    );
    f.cli("hold",json!({"key":"TASK-1","outcome":"needs-human","reason":"pair consumed; current user must authorize further work"})).unwrap();
    unchanged_rejection(&f, "authorize-budget-adjustment", budget(&f));
    let receipt = f.root.join("next-user-approval.txt");
    fs::write(
        &receipt,
        "Another actual current-user instruction for one pair",
    )
    .unwrap();
    let mut request = budget(&f);
    request["artifact"] = json!(receipt);
    f.cli("authorize-budget-adjustment", request).unwrap();
    refresh(&f);
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
    f.begin("implementer").unwrap();
    let t = f.state().tasks["TASK-1"].clone();
    assert_eq!(t.reviews, 4);
}

#[test]
fn followup_requires_unchanged_historical_pr_and_rejects_reused_authorization() {
    let f = held_merge(true);
    let original = fs::read(f.root.join("pr.json")).unwrap();
    for field in ["headRefOid", "mergeCommit", "number", "state"] {
        let mut pr: Value = serde_json::from_slice(&original).unwrap();
        pr[field] = match field {
            "number" => json!(99),
            "mergeCommit" => json!({"oid":"missing"}),
            _ => json!("drift"),
        };
        fs::write(f.root.join("pr.json"), serde_json::to_vec(&pr).unwrap()).unwrap();
        unchanged_rejection(&f, "prepare-followup", request(&f));
    }
    fs::write(f.root.join("pr.json"), original).unwrap();
    prepare(&f);
    let mut brief = f.state().tasks["TASK-1"].criteria.clone();
    brief.retain(|c| c.id != "AC3" && c.id != "AC6");
    bind(&f);
    unchanged_rejection(
        &f,
        "brief",
        json!({"key":"TASK-1","criteria":brief,"tier":"routine","reason":"staged merge must not weaken task acceptance"}),
    );
}

fn historical_reviews(f: &Fixture) {
    full_evidence(f, &["AC3", "AC6"]);
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    let snapshot = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(
        id,
        "reviewer",
        "completed",
        Some(
            json!({"snapshot":snapshot,"verdict":"BLOCKED", "findings":[],
        "evidence_gaps":["AC3 native operational proof and AC6 temporary-code removal remain"]}),
        ),
    )
    .unwrap();
}
