use super::*;

fn hold_exhausted(f: &Fixture) {
    f.bound(false);
    for _ in 0..3 {
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "reviewer", "cancelled", None).unwrap();
    }
    f.cli(
        "hold",
        json!({"key":"TASK-1","outcome":"needs-human","reason":"budget exhausted"}),
    )
    .unwrap();
}

fn grant(f: &Fixture, mode: &str, paths: Value) -> anyhow::Result<Value> {
    let receipt = f.root.join(format!("{mode}-approval.txt"));
    fs::write(&receipt, format!("User grants one {mode} pair")).unwrap();
    f.cli(
        "authorize-budget-adjustment",
        json!({
            "key":"TASK-1",
            "mode":mode,
            "source":"fixture user instruction",
            "artifact":receipt,
            "scope":"one bounded implementation and review pair",
            "paths":paths
        }),
    )
}

fn resume(f: &Fixture) {
    let mut task = Fixture::spec("TASK-1", 1);
    task["jira_status"] = json!("2");
    f.cli("refresh", json!({"tasks":[task,Fixture::spec("TASK-2",2)]}))
        .unwrap();
    f.cli("resume", json!({"key":"TASK-1"})).unwrap();
}

fn scoped_fixture() -> Fixture {
    let f = Fixture::new();
    let worker = f.repo.join(".worktrees/worker1");
    f.bound(false);
    fs::write(worker.join("README.md"), "Documentation baseline\n").unwrap();
    git(&worker, &["add", "."]).unwrap();
    git(&worker, &["commit", "-m", "documentation baseline"]).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    for _ in 0..3 {
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "reviewer", "cancelled", None).unwrap();
    }
    f.cli(
        "hold",
        json!({"key":"TASK-1","outcome":"needs-human","reason":"budget exhausted"}),
    )
    .unwrap();
    grant(&f, "documentation-cycle", json!(["README.md"])).unwrap();
    f
}

fn spend_scoped(f: &Fixture) {
    resume(f);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    let worker = f.repo.join(".worktrees/worker1");
    fs::write(worker.join("README.md"), "Correct documentation\n").unwrap();
    git(&worker, &["commit", "-am", "documentation repair"]).unwrap();
    f.finish(id, "implementer", "completed", None).unwrap();
    f.cli("snapshot", json!({"key":"TASK-1"})).unwrap();
    f.checkpoint();
    let review = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(review, "reviewer", "cancelled", None).unwrap();
}

#[test]
fn pair_grants_one_of_each() {
    let f = Fixture::new();
    hold_exhausted(&f);
    grant(&f, "additional-cycle", json!([])).unwrap();
    resume(&f);
    let implementation = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    assert!(f.begin("implementer").is_err());
    f.finish(implementation, "implementer", "cancelled", None)
        .unwrap();
    f.checkpoint();
    let review = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    assert!(f.begin("reviewer").is_err());
    f.finish(review, "reviewer", "cancelled", None).unwrap();
}

#[test]
fn failed_pair_is_spent() {
    let f = Fixture::new();
    hold_exhausted(&f);
    grant(&f, "additional-cycle", json!([])).unwrap();
    resume(&f);
    let implementation = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(implementation, "implementer", "cancelled", None)
        .unwrap();
    f.checkpoint();
    assert!(f.begin("implementer").is_err());
    let review = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(review, "reviewer", "cancelled", None).unwrap();
    assert!(f.begin("reviewer").is_err());
}

#[test]
fn pair_requires_hold() {
    let f = Fixture::new();
    f.bound(false);
    assert!(grant(&f, "additional-cycle", json!([])).is_err());
}

#[test]
fn pair_is_task_scoped() {
    let f = Fixture::new();
    hold_exhausted(&f);
    let receipt = f.root.join("other-task-approval.txt");
    fs::write(&receipt, "User grants TASK-2 one pair").unwrap();
    assert!(
        f.cli(
            "authorize-budget-adjustment",
            json!({"key":"TASK-2","mode":"additional-cycle","source":"fixture user",
                "artifact":receipt,"scope":"TASK-2 only","paths":[]}),
        )
        .is_err()
    );
    grant(&f, "additional-cycle", json!([])).unwrap();
}

#[test]
fn scoped_pair_is_uncounted() {
    let f = scoped_fixture();
    spend_scoped(&f);
    assert_eq!(f.state().tasks["TASK-1"].reviews, 3);
}

#[test]
fn scoped_pair_keeps_exhaustion() {
    let f = scoped_fixture();
    spend_scoped(&f);
    assert!(f.begin("implementer").is_err());
    assert!(f.begin("reviewer").is_err());
}

#[test]
fn scoped_pair_needs_acceptance() {
    let f = scoped_fixture();
    spend_scoped(&f);
    let task = &f.state().tasks["TASK-1"];
    assert!(!task.final_verified);
    assert!(f.cli("complete", json!({"key":"TASK-1"})).is_err());
}
