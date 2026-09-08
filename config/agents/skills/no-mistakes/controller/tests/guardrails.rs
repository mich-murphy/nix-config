mod improvements;
use epic_control::{engine, model::*, store::*};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};
use tempfile::TempDir;

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
    repo: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]).unwrap();
        git(&repo, &["config", "user.name", "Test"]).unwrap();
        git(&repo, &["config", "user.email", "test@example.invalid"]).unwrap();
        fs::write(repo.join(".gitignore"), ".local/\n.worktrees/\n").unwrap();
        fs::write(repo.join("source.txt"), "base\n").unwrap();
        git(&repo, &["add", "."]).unwrap();
        git(&repo, &["commit", "-m", "test base"]).unwrap();
        let head = sha(&repo, "HEAD").unwrap();
        git(&repo, &["update-ref", "refs/remotes/origin/main", &head]).unwrap();
        let root = repo.join(".local/runs/test");
        let fixture = Self {
            _tmp: tmp,
            root,
            repo,
        };
        fixture.cli("init", json!({"repo":fixture.repo,"github_repo":"test/repo","epic":"EPIC-1","statuses":{"todo":"1","progress":"2","review":"3","done":"4"},"coordinator_model":"gpt-5.6-sol","coordinator_effort":"medium","preflight":"isolated fixture prerequisites"})).unwrap();
        fixture
    }
    fn call(&self, mut value: Value) -> anyhow::Result<Value> {
        let command = value
            .as_object_mut()
            .unwrap()
            .remove("command")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        self.cli(&command, value)
    }
    fn state(&self) -> State {
        serde_json::from_value(self.cli("status", json!({})).unwrap()).unwrap()
    }
    fn spec(key: &str, priority: u32) -> Value {
        json!({"key":key,"parent":"EPIC-1","subtasks":[],"priority":priority,"due":null,"rank":"a","not_before":null,"dependencies":[],"member":true,"ownership_clear":true,"ownership_evidence":"no other branch or PR","jira_status":"1","resolved":false,"blocker":null})
    }
    fn claimed(&self) {
        self.call(
            json!({"command":"discover","tasks":[Self::spec("TASK-1",1),Self::spec("TASK-2",2)]}),
        )
        .unwrap();
        self.call(json!({"command":"claim","key":"TASK-1"}))
            .unwrap();
    }
    fn bound(&self, human: bool) {
        self.claimed();
        self.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"2","resolved":false,"target":"2","transitions":[]})).unwrap();
        self.call(json!({"command":"brief","key":"TASK-1","criteria":[{"id":"AC1","text":"observable behavior","human_only":human}],"tier":"routine","reason":"isolated behavior"})).unwrap();
        self.call(json!({"command":"reserve-slot","slot":"worker1"}))
            .unwrap();
        let path = self.repo.join(".worktrees/worker1");
        git(
            &self.repo,
            &[
                "worktree",
                "add",
                "-b",
                "agent-test-linux/worker1/TASK-1-change",
                path.to_str().unwrap(),
                "origin/main",
            ],
        )
        .unwrap();
        self.call(json!({"command":"bind","key":"TASK-1","slot":"worker1"}))
            .unwrap();
        self.loop_plan();
    }
    fn loop_plan(&self) {
        let baseline = self.root.join("baseline.txt");
        fs::write(&baseline, "Initial source.txt contains base; inspect source contract at the recorded base commit.").unwrap();
        self.call(json!({"command":"loop-plan","key":"TASK-1","plan":{"baseline_commit":self.state().tasks["TASK-1"].snapshot.as_ref().unwrap().base,"family":"fixture","deliverable":"observable behavior","components":["source.txt"],"examples":["source.txt at base"],"baselines":[{"criterion":"AC1","starting_condition":"base behavior","target":"observable behavior","method":"inspect source contract","artifact":baseline}]}})).unwrap();
    }
    fn checkpoint(&self) {
        let count = self.state().launches.len();
        let log = self.root.join(format!("progress-{count}.txt"));
        fs::write(
            &log,
            format!("Fixture resolved contract question in turn {count}"),
        )
        .unwrap();
        self.call(json!({"command":"checkpoint","key":"TASK-1","advanced":true,"finding":"resolved fixture contract question","next_step":"validate observable contract","scope_reason":"One source behavior remains within the planned deliverable","artifact":log})).unwrap();
    }
    fn evidence(&self) {
        let log = self.root.join("proof.txt");
        fs::write(&log, "checked observable contract\n").unwrap();
        self.call(json!({"command":"measure","key":"TASK-1","criterion":"AC1","observed":"observable contract satisfied relative to recorded base","satisfied":true,"artifact":log})).unwrap();
        self.call(json!({"command":"evidence","key":"TASK-1","evidence":{"criterion":"AC1","status":"passed","kind":"inspection","artifact":log,"implementation":["source.txt:1"],"description":"source contract inspection","human_source":null}})).unwrap();
    }
    fn begin(&self, role: &str) -> anyhow::Result<Value> {
        self.call(json!({"command":"agent-begin","key":"TASK-1","role":role,"allow_evidence_gaps":role=="reviewer","model":if role=="implementer"{"gpt-5.6-terra"}else{"gpt-5.6-sol"},"effort":"medium","reason":"new evidence or focused verification question"}))
    }
    fn finish(
        &self,
        id: u64,
        session: &str,
        outcome: &str,
        review: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.call(json!({"command":"agent-finish","launch":id,"session":session,"outcome":outcome,"review":review,"usage":null}))
    }
}

#[test]
fn freezes_membership_and_respects_priority() {
    let f = Fixture::new();
    f.call(
        json!({"command":"discover","tasks":[Fixture::spec("TASK-1",2),Fixture::spec("TASK-2",1)]}),
    )
    .unwrap();
    assert!(f.call(json!({"command":"claim","key":"TASK-1"})).is_err());
    f.call(json!({"command":"claim","key":"TASK-2"})).unwrap();
    assert!(f.call(json!({"command":"discover","tasks":[]})).is_err());
}
#[test]
fn does_not_overlap_tasks() {
    let f = Fixture::new();
    f.claimed();
    assert!(f.call(json!({"command":"claim","key":"TASK-2"})).is_err());
}
#[test]
fn blocks_code_dependency_without_delivery_on_main() {
    let f = Fixture::new();
    let mut task = Fixture::spec("TASK-1", 1);
    task["dependencies"] = json!([{"key":"EXT-1","code":true,"verified":true,"evidence":"Jira Done","main_commit":null}]);
    f.call(json!({"command":"discover","tasks":[task]}))
        .unwrap();
    assert!(f.call(json!({"command":"claim","key":"TASK-1"})).is_err());
}
#[test]
fn concurrent_launch_claims_have_one_winner() {
    let f = Fixture::new();
    f.bound(false);
    let barrier = Arc::new(Barrier::new(2));
    let jobs: Vec<_> = (0..2)
        .map(|_| {
            let root = f.root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                invoke(&root, "agent-begin", json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium","reason":"","ci_repair":false})).is_ok()
            })
        })
        .collect();
    assert_eq!(
        jobs.into_iter()
            .filter_map(|j| j.join().ok())
            .filter(|ok| *ok)
            .count(),
        1
    );
    assert_eq!(f.state().launches.len(), 1);
}
#[test]
fn cancellations_consume_all_three_review_attempts_across_reopen() {
    let f = Fixture::new();
    f.bound(false);
    for _ in 0..3 {
        let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "review-session", "cancelled", None).unwrap();
    }
    assert!(f.begin("reviewer").is_err());
    assert_eq!(f.state().tasks["TASK-1"].reviews, 3);
}
#[test]
fn rejects_stale_review_after_head_changes() {
    let f = Fixture::new();
    f.bound(false);
    f.evidence();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    let path = f.repo.join(".worktrees/worker1");
    fs::write(path.join("source.txt"), "changed\n").unwrap();
    git(&path, &["add", "."]).unwrap();
    git(&path, &["commit", "-m", "change during review"]).unwrap();
    assert!(
        f.finish(
            id,
            "reviewer",
            "completed",
            Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]}))
        )
        .is_err()
    );
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    f.call(json!({"command":"snapshot","key":"TASK-1"}))
        .unwrap();
    assert!(f.state().tasks["TASK-1"].evidence.is_empty());
}
#[test]
fn pass_requires_every_criterion_and_immutable_artifacts() {
    let f = Fixture::new();
    f.bound(false);
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    assert!(
        f.finish(
            id,
            "reviewer",
            "completed",
            Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]}))
        )
        .is_err()
    );
    f.finish(id, "reviewer", "failed", None).unwrap();
    f.evidence();
    fs::write(f.root.join("proof.txt"), "replaced").unwrap();
    assert!(engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).is_err());
}
#[test]
fn human_acceptance_cannot_be_relabelled_as_test() {
    let f = Fixture::new();
    f.bound(true);
    let log = f.root.join("test.log");
    fs::write(&log, "passed").unwrap();
    assert!(f.call(json!({"command":"evidence","key":"TASK-1","evidence":{"criterion":"AC1","status":"passed","kind":"command","artifact":log,"implementation":["source.txt"],"description":"test passed","human_source":null}})).is_err());
}
#[test]
fn reviewer_cannot_reuse_implementer_session() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "same", "completed", None).unwrap();
    f.checkpoint();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    assert!(f.finish(id, "same", "cancelled", None).is_err());
}
#[test]
fn unknown_jira_outcome_blocks_replay_and_reconciles_success() {
    let f = Fixture::new();
    f.claimed();
    let op=f.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"1","resolved":false,"target":"2","transitions":[{"id":"10","to":"2"}]})).unwrap()["id"].as_u64().unwrap();
    f.call(json!({"command":"dispatch","operation":op}))
        .unwrap();
    assert!(
        f.call(json!({"command":"dispatch","operation":op}))
            .is_err()
    );
    assert!(f.call(json!({"command":"hold","key":"TASK-1","outcome":"needs-human","reason":"network timeout"})).is_err());
    f.call(json!({"command":"jira-observe","operation":op,"current":"2","resolved":false,"evidence":"reread issue after crash"})).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].sync, Sync::Confirmed);
    assert_eq!(f.state().operations[0].attempts, 1);
}
#[test]
fn jira_retry_is_bounded_and_does_not_reopen_external_done() {
    let f = Fixture::new();
    f.claimed();
    let op=f.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"1","resolved":false,"target":"2","transitions":[{"id":"10","to":"2"}]})).unwrap()["id"].as_u64().unwrap();
    for attempt in 0..2 {
        f.call(json!({"command":"dispatch","operation":op}))
            .unwrap();
        f.call(json!({"command":"jira-observe","operation":op,"current":"1","resolved":false,"evidence":"fresh issue"})).unwrap();
        if attempt == 0 {
            f.call(json!({"command":"jira-retry","operation":op,"current":"1","resolved":false,"transitions":[{"id":"10","to":"2"}]})).unwrap();
        }
    }
    assert!(f.call(json!({"command":"jira-retry","operation":op,"current":"1","resolved":false,"transitions":[{"id":"10","to":"2"}]})).is_err());
}
#[test]
fn cannot_clean_unfinished_or_unknown_slot() {
    let f = Fixture::new();
    f.bound(false);
    assert!(
        f.call(json!({"command":"cleanup-check","key":"TASK-1"}))
            .is_err()
    );
    fs::create_dir(f.repo.join(".worktrees/worker2")).unwrap();
    assert!(
        f.call(json!({"command":"reserve-slot","slot":"worker2"}))
            .is_err()
    );
}
#[test]
fn cannot_defer_ci_before_deadline() {
    let f = Fixture::new();
    f.bound(false);
    assert!(
        f.call(
            json!({"command":"hold","key":"TASK-1","outcome":"deferred","reason":"checks pending"})
        )
        .is_err()
    );
}
#[test]
fn requirements_change_invalidates_evidence_without_resetting_budget() {
    let f = Fixture::new();
    f.bound(false);
    f.evidence();
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    f.call(json!({"command":"brief","key":"TASK-1","criteria":[{"id":"AC1","text":"different behavior","human_only":false}],"tier":"routine","reason":"same scope, corrected requirement"})).unwrap();
    let t = &f.state().tasks["TASK-1"];
    assert!(t.evidence.is_empty());
    assert_eq!(t.reviews, 1);
}
#[test]
fn malformed_commands_fail_closed() {
    let f = Fixture::new();
    assert!(
        f.cli("dispatch", json!({"operation":1,"force":true}))
            .is_err()
    );
    assert!(
        f.cli(
            "agent-begin",
            json!({"key":"TASK-1","role":"fixer","model":"x","effort":"max"})
        )
        .is_err()
    );
    assert!(f.state().launches.is_empty());
    assert!(f.state().operations.is_empty());
}

#[test]
fn cross_run_issue_claims_are_exclusive() {
    let f = Fixture::new();
    f.claimed();
    let root = f.repo.join(".local/runs/other");
    invoke(&root, "init", json!({"repo":f.repo,"github_repo":"test/repo","epic":"EPIC-1","statuses":{"todo":"1","progress":"2","review":"3","done":"4"},"coordinator_model":"gpt-5.6-sol","coordinator_effort":"medium","preflight":"second isolated coordinator"})).unwrap();
    invoke(
        &root,
        "discover",
        json!({"tasks":[Fixture::spec("TASK-1",1)]}),
    )
    .unwrap();
    assert!(invoke(&root, "claim", json!({"key":"TASK-1"})).is_err());
}
#[test]
fn old_jira_receipt_cannot_overwrite_confirmed_state() {
    let f = Fixture::new();
    f.claimed();
    let op=f.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"1","resolved":false,"target":"2","transitions":[{"id":"10","to":"2"}]})).unwrap()["id"].as_u64().unwrap();
    f.call(json!({"command":"dispatch","operation":op}))
        .unwrap();
    f.call(json!({"command":"jira-observe","operation":op,"current":"2","resolved":false,"evidence":"fresh"})).unwrap();
    assert!(f.call(json!({"command":"jira-observe","operation":op,"current":"1","resolved":false,"evidence":"old replay"})).is_err());
    assert_eq!(f.state().tasks["TASK-1"].spec.jira_status, "2");
}
#[test]
fn cannot_mark_unrecorded_subtask_done() {
    let f = Fixture::new();
    f.claimed();
    // Fault injection models an external or interrupted state unavailable through normal commands.
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("fixture-subtask", |s| {
            let t = s.tasks.get_mut("TASK-1").unwrap();
            t.spec.subtasks.push("SUB-1".into());
            t.final_verified = true;
            Ok(json!({}))
        })
        .unwrap();
    assert!(f.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"SUB-1","current":"2","resolved":false,"target":"4","transitions":[{"id":"40","to":"4"}]})).is_err());
}
#[test]
fn continued_repair_turns_use_one_cycle() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    for _ in 0..3 {
        let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "implementer", "completed", None).unwrap();
        f.checkpoint();
    }
    assert_eq!(f.state().tasks["TASK-1"].repairs, 1);
}
#[test]
fn astra_review_consumes_both_budgets() {
    let f = Fixture::new();
    f.bound(false);
    let id = f.begin("reviewer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    let request = json!({"command":"agent-begin","key":"TASK-1","role":"reviewer","allow_evidence_gaps":true,"model":"gpt-6-astra","effort":"high","reason":"specific consequential question unresolved by Sol; inspect source contract"});
    let id = f.call(request.clone()).unwrap()["id"].as_u64().unwrap();
    f.finish(id, "reviewer", "cancelled", None).unwrap();
    assert!(f.call(request).is_err());
    let t = &f.state().tasks["TASK-1"];
    assert_eq!((t.reviews, t.escalations), (2, 1));
}
#[test]
fn deadline_survives_hold_and_resume() {
    let f = Fixture::new();
    f.bound(false);
    // Fault injection models an external or interrupted state unavailable through normal commands.
    let mut store = Store::open(&f.root).unwrap();
    let deadline = now() - 1;
    store
        .change("fixture-ci", |s| {
            let t = s.tasks.get_mut("TASK-1").unwrap();
            t.deadlines
                .insert(t.snapshot.as_ref().unwrap().head.to_string(), deadline);
            Ok(json!({}))
        })
        .unwrap();
    f.call(json!({"command":"hold","key":"TASK-1","outcome":"deferred","reason":"CI deadline expired"})).unwrap();
    let specs: Vec<_> = f.state().tasks.values().map(|t| t.spec.clone()).collect();
    f.call(json!({"command":"refresh","tasks":specs})).unwrap();
    f.call(json!({"command":"resume","key":"TASK-1","final_revisit":true}))
        .unwrap();
    assert_eq!(
        f.state().tasks["TASK-1"].deadlines.values().next(),
        Some(&deadline)
    );
}
#[test]
fn rollback_does_not_consume_attempt_on_rejected_model() {
    let f = Fixture::new();
    f.bound(false);
    assert!(f.call(json!({"command":"agent-begin","key":"TASK-1","role":"reviewer","model":"gpt-5.6-terra","effort":"medium"})).is_err());
    assert_eq!(f.state().tasks["TASK-1"].reviews, 0);
    assert!(f.state().launches.is_empty());
}

#[cfg(target_os = "linux")]
impl Fixture {
    fn fake_tools(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = self.root.join("fake-bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("gh"),r##"#!/bin/sh
case "$1 $2" in
  'pr list') if test -f "$FAKE_ROOT/created"; then printf '['; cat "$FAKE_ROOT/pr.json"; printf ']'; else printf '[]'; fi ;;
  'pr view') case "$*" in *autoMergeRequest*) printf '{"autoMergeRequest":null,"mergeStateStatus":"CLEAN"}' ;; *) cat "$FAKE_ROOT/pr.json" ;; esac ;;
  'pr create') if test -f "$FAKE_ROOT/fail-create"; then exit 1; fi; touch "$FAKE_ROOT/created"; printf 'created' ;;
  'pr ready') cp "$FAKE_ROOT/ready.json" "$FAKE_ROOT/pr.json" ;;
  'pr checks') printf '[{"name":"required","bucket":"pass","link":"test://check"}]' ;;
  'pr merge') cp "$FAKE_ROOT/merged.json" "$FAKE_ROOT/pr.json" ;;
  *) exit 2 ;;
esac
"##).unwrap();
        fs::write(
            bin.join("codex"),
            r##"#!/bin/sh
if test "$1 $2" = 'login status'; then printf 'Logged in using ChatGPT\n'; exit 0; fi
printf '%s\n' "$@" > "$FAKE_ROOT/codex-args.txt"
while test "$#" -gt 0; do
  if test "$1" = '--output-last-message'; then shift; result=$1; fi
  shift
done
cat > "$FAKE_ROOT/received-prompt.txt"
printf 'done' > "$result"
printf '{"type":"thread.started","thread_id":"controlled-session"}\n'
printf '{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}\n'
"##,
        )
        .unwrap();
        for name in ["gh", "codex"] {
            fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        let t = &self.state().tasks["TASK-1"];
        let pr = json!({"number":12,"state":"OPEN","isDraft":true,"headRefOid":t.snapshot.as_ref().unwrap().head,"headRefName":t.branch,"baseRefName":"main","mergeCommit":null,"url":"test://pr/12"});
        fs::write(self.root.join("pr.json"), serde_json::to_vec(&pr).unwrap()).unwrap();
        let mut ready = pr;
        ready["isDraft"] = json!(false);
        fs::write(
            self.root.join("ready.json"),
            serde_json::to_vec(&ready).unwrap(),
        )
        .unwrap();
        let mut merged = ready;
        merged["state"] = json!("MERGED");
        merged["mergeCommit"] = json!({"oid":t.snapshot.as_ref().unwrap().head});
        fs::write(
            self.root.join("merged.json"),
            serde_json::to_vec(&merged).unwrap(),
        )
        .unwrap();
        bin
    }
}

impl Fixture {
    fn cli(&self, command: &str, input: Value) -> anyhow::Result<Value> {
        invoke(&self.root, command, input)
    }
}

fn invoke(root: &Path, command: &str, input: Value) -> anyhow::Result<Value> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let bin = root.join("fake-bin");
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(paths)?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_epic-control"))
        .args(["--state", root.to_str().unwrap(), command])
        .env("PATH", path)
        .env("FAKE_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if !["status", "next", "drive", "usage-report", "review-schema"].contains(&command) {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(&input)?.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice::<Value>(&output.stdout)?["result"].clone())
}

#[test]
#[cfg(target_os = "linux")]
fn gh_wrapper_reconciles_create_and_definite_failure_without_duplicate() {
    let f = Fixture::new();
    f.bound(false);
    f.changed();
    f.fake_tools();
    let body = f.root.join("pr-body.md");
    fs::write(&body, "Task evidence").unwrap();
    let op=f.call(json!({"command":"gh-prepare","key":"TASK-1","action":"create-pr","title":"feat(test): behavior (TASK-1)","body":body,"method":null})).unwrap()["id"].as_u64().unwrap();
    fs::write(f.root.join("fail-create"), "fail").unwrap();
    f.cli("gh-run", json!({"operation":op})).unwrap();
    assert_eq!(f.state().operations[0].status, OpStatus::Unknown);
    f.cli(
        "gh-resolve-failure",
        json!({"operation":op,"evidence":"failed caller exited; fresh GitHub list proves no PR"}),
    )
    .unwrap();
    assert_eq!(f.state().operations[0].status, OpStatus::Failed);
    fs::remove_file(f.root.join("fail-create")).unwrap();
    let op=f.call(json!({"command":"gh-prepare","key":"TASK-1","action":"create-pr","title":"feat(test): behavior (TASK-1)","body":body,"method":null})).unwrap()["id"].as_u64().unwrap();
    f.cli("gh-run", json!({"operation":op})).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].pr, Some(12));
    f.cli("gh-run", json!({"operation":op})).unwrap();
    assert_eq!(f.state().operations[1].attempts, 1);
}
#[test]
#[cfg(target_os = "linux")]
fn runtime_check_executes_and_records_real_exit_status() {
    let f = Fixture::new();
    f.bound(false);
    f.cli("run-check",json!({"key":"TASK-1","criteria":["AC1"],"argv":["sh","-c","printf contract-checked"],"cwd":f.repo.join(".worktrees/worker1"),"timeout_seconds":10,"implementation":["source.txt:1"]})).unwrap();
    assert_eq!(
        f.state().tasks["TASK-1"].evidence["AC1"].input.status,
        EvidenceStatus::Passed
    );
    assert_eq!(f.state().operations[0].status, OpStatus::Confirmed);
}
#[test]
#[cfg(target_os = "linux")]
fn codex_runner_controls_model_effort_session_and_counts_usage() {
    let f = Fixture::new();
    f.bound(false);
    f.fake_tools();
    let prompt = f.root.join("brief.txt");
    fs::write(&prompt, "Inspect the existing source. Do not change it.").unwrap();
    f.cli("run-agent",json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium","reason":"","ci_repair":false,"prompt":prompt,"timeout_seconds":10})).unwrap();
    let s = f.state();
    let l = &s.launches[0];
    assert!(l.ended.is_some() && l.process.as_ref().unwrap().terminal);
    assert_eq!(l.session.as_deref(), Some("controlled-session"));
    assert_eq!(l.usage.as_ref().unwrap()["input_tokens"], 10);
    let args = fs::read_to_string(f.root.join("codex-args.txt")).unwrap();
    assert!(args.contains("gpt-5.6-terra") && args.contains("model_reasoning_effort=\"medium\""));
}

impl Fixture {
    fn changed(&self) {
        let path = self.repo.join(".worktrees/worker1");
        fs::write(path.join("source.txt"), "implemented behavior\n").unwrap();
        git(&path, &["add", "."]).unwrap();
        git(&path, &["commit", "-m", "feat(test): behavior"]).unwrap();
        self.call(json!({"command":"snapshot","key":"TASK-1"}))
            .unwrap();
        self.checkpoint();
    }
    fn pass_review(&self) {
        self.evidence();
        let snap = self.state().tasks["TASK-1"].snapshot.clone().unwrap();
        let id = self.begin("reviewer").unwrap()["id"].as_u64().unwrap();
        self.finish(
            id,
            "reviewer",
            "completed",
            Some(json!({"snapshot":snap,"verdict":"PASS","findings":[],"evidence_gaps":[]})),
        )
        .unwrap();
    }

    #[cfg(target_os = "linux")]
    fn add_bare_origin(&self) {
        let remote = self._tmp.path().join("remote.git");
        git(
            &self.repo,
            &[
                "clone",
                "--bare",
                self.repo.to_str().unwrap(),
                remote.to_str().unwrap(),
            ],
        )
        .unwrap();
        git(
            &self.repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        )
        .unwrap();
    }

    #[cfg(target_os = "linux")]
    fn complete_delivery(&self) -> String {
        self.bound(false);
        self.changed();
        self.pass_review();
        self.fake_tools();
        let head = self.state().tasks["TASK-1"]
            .snapshot
            .as_ref()
            .unwrap()
            .head
            .clone();
        git(&self.repo, &["merge", "--ff-only", &head]).unwrap();
        self.add_bare_origin();
        let body = self.root.join("body.md");
        fs::write(&body, "Acceptance and validation").unwrap();
        let id=self.call(json!({"command":"gh-prepare","key":"TASK-1","action":"create-pr","title":"feat(test): behavior (TASK-1)","body":body,"method":null})).unwrap()["id"].as_u64().unwrap();
        self.cli("gh-run", json!({"operation":id})).unwrap();
        self.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"3","resolved":false,"target":"3","transitions":[]})).unwrap();
        let id=self.call(json!({"command":"gh-prepare","key":"TASK-1","action":"ready","title":null,"body":null,"method":null})).unwrap()["id"].as_u64().unwrap();
        self.cli("gh-run", json!({"operation":id})).unwrap();
        self.cli("poll-checks", json!({"key":"TASK-1"})).unwrap();
        let id=self.call(json!({"command":"gh-prepare","key":"TASK-1","action":"merge","title":null,"body":null,"method":"merge"})).unwrap()["id"].as_u64().unwrap();
        self.cli("gh-run", json!({"operation":id})).unwrap();
        assert!(self.state().tasks["TASK-1"].main_verified);
        assert!(
            self.call(json!({"command":"complete","key":"TASK-1"}))
                .is_err()
        );
        let final_log = self.root.join("final.log");
        fs::write(&final_log, "final main acceptance").unwrap();
        self.call(
            json!({"command":"final-verify","key":"TASK-1","main_commit":head,"evidence":final_log}),
        )
        .unwrap();
        self.call(json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"4","resolved":true,"target":"4","transitions":[]})).unwrap();
        self.call(json!({"command":"complete","key":"TASK-1"}))
            .unwrap();
        head.to_string()
    }

    #[cfg(target_os = "linux")]
    fn assert_completed_delivery(&self, head: &str, accounting: (u32, u32, u32, u32, usize, u64)) {
        let state = self.state();
        let task = &state.tasks["TASK-1"];
        assert!(state.active.is_none());
        assert_eq!(task.delivery, Delivery::Merged);
        assert!(task.main_verified && task.final_verified);
        assert!(!task.pr_open && !task.draft);
        assert_eq!(task.merged_commit.as_deref(), Some(head));
        assert_eq!(task.sync, Sync::Confirmed);
        assert_eq!(task.spec.jira_status, "4");
        assert!(task.spec.resolved);
        assert_eq!(
            task.next_action,
            "Run cleanup-check, then repository cleanup helper"
        );
        assert_eq!(
            (
                task.reviews,
                task.repairs,
                task.ci_repairs,
                task.escalations,
                state.launches.len(),
                task.review.as_ref().unwrap().launch,
            ),
            accounting
        );
    }
}
#[test]
fn parent_done_revalidates_evidence_at_prepare_and_dispatch() {
    let f = Fixture::new();
    f.bound(false);
    f.pass_review();
    let commit = f.state().tasks["TASK-1"]
        .snapshot
        .as_ref()
        .unwrap()
        .head
        .clone();
    let final_log = f.root.join("final.txt");
    fs::write(&final_log, "verified on main").unwrap();
    f.call(
        json!({"command":"final-verify","key":"TASK-1","main_commit":commit,"evidence":final_log}),
    )
    .unwrap();
    let prepare = json!({"command":"jira-prepare","key":"TASK-1","issue":"TASK-1","current":"2","resolved":false,"target":"4","transitions":[{"id":"40","to":"4"}]});
    let original = fs::read(f.root.join("proof.txt")).unwrap();
    fs::write(f.root.join("proof.txt"), "changed evidence").unwrap();
    assert!(f.call(prepare.clone()).is_err());
    fs::write(f.root.join("proof.txt"), &original).unwrap();
    let id = f.call(prepare).unwrap()["id"].as_u64().unwrap();
    fs::write(f.root.join("proof.txt"), "changed evidence").unwrap();
    assert!(
        f.call(json!({"command":"dispatch","operation":id}))
            .is_err()
    );
    assert_eq!(f.state().operations[0].status, OpStatus::Prepared);
}
#[test]
#[cfg(target_os = "linux")]
fn missing_check_executable_is_failed_not_permanently_unknown() {
    let f = Fixture::new();
    f.bound(false);
    let result=f.cli("run-check",json!({"key":"TASK-1","criteria":["AC1"],"argv":["/nonexistent/epic-test-command"],"cwd":f.repo.join(".worktrees/worker1"),"timeout_seconds":10,"implementation":["source.txt:1"]})).unwrap();
    assert_eq!(result["passed"], false);
    assert_eq!(f.state().operations[0].status, OpStatus::Failed);
    assert!(engine::idle(&f.state()).is_ok());
}
#[test]
#[cfg(target_os = "linux")]
fn uncertain_provision_confirms_the_same_already_bound_slot() {
    let f = Fixture::new();
    f.bound(false);
    // Fault injection models an external or interrupted state unavailable through normal commands.
    let mut store = Store::open(&f.root).unwrap();
    let value=store.change("fixture-helper",|s|{let id=s.id();s.operations.push(Operation{id,key:"TASK-1".into(),kind:"provision".into(),status:OpStatus::Unknown,created:now(),snapshot:None,attempts:1,details:json!({"slot":"worker1","branch":"agent-test-linux/worker1/TASK-1-change"}),observation:None,process:Some(ProcessIdentity{pid:2_000_000_000,start:"old".into(),group:2_000_000_000,terminal:true,marker:"fixture".into(),exit_success:Some(true)})});Ok(json!({"id":id}))}).unwrap();
    let result = f
        .cli("reconcile-helper", json!({"operation":value["id"]}))
        .unwrap();
    assert_eq!(result["confirmed"], true);
    assert_eq!(f.state().operations[0].status, OpStatus::Confirmed);
    assert_eq!(f.state().tasks["TASK-1"].slot.as_deref(), Some("worker1"));
}

#[test]
#[cfg(target_os = "linux")]
fn same_slot_replay_cannot_bypass_a_second_unfinished_owner() {
    let f = Fixture::new();
    f.bound(false);
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("fixture-duplicate-slot-owner", |state| {
            state.tasks.get_mut("TASK-2").unwrap().slot = Some("worker1".into());
            Ok(json!({"changed":true}))
        })
        .unwrap();
    let operation = inject_provision(
        &f,
        "TASK-1",
        "worker1",
        "agent-test-linux/worker1/TASK-1-change",
    );
    let before = serde_json::to_value(f.state()).unwrap();

    assert!(
        f.cli("reconcile-helper", json!({"operation":operation}))
            .is_err()
    );
    assert_eq!(serde_json::to_value(f.state()).unwrap(), before);
}

#[test]
#[cfg(target_os = "linux")]
fn provision_rejects_missing_checkout_owned_by_unfinished_task_before_helper_runs() {
    let f = Fixture::new();
    prepare_second_task_after_first_slot_disappears(&f);
    install_worktree_helper(&f);
    f.cli("reserve-slot", json!({"slot":"worker1"})).unwrap();
    let before = f.state();

    assert!(
        f.cli(
            "provision-slot",
            json!({"slot":"worker1","branch":"agent-test-linux/worker1/TASK-2-change"})
        )
        .is_err()
    );

    let after = f.state();
    assert!(!f.root.join("worktree-helper-ran").exists());
    assert!(!f.repo.join(".worktrees/worker1").exists());
    assert_eq!(after.operations.len(), before.operations.len());
    assert_eq!(after.serial, before.serial);
    assert_eq!(after.tasks["TASK-1"].slot.as_deref(), Some("worker1"));
    assert!(after.tasks["TASK-2"].slot.is_none());
}

#[test]
#[cfg(target_os = "linux")]
fn legacy_provision_collision_settles_failed_and_allows_a_different_slot() {
    let f = Fixture::new();
    prepare_second_task_after_first_slot_disappears(&f);
    let first_counters = task_counters(&f.state().tasks["TASK-1"]);
    let second_counters = task_counters(&f.state().tasks["TASK-2"]);
    let old_path = f.repo.join(".worktrees/worker1");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "agent-test-linux/worker1/TASK-2-change",
            old_path.to_str().unwrap(),
            "origin/main",
        ],
    )
    .unwrap();
    let preserved = fs::read(old_path.join("source.txt")).unwrap();
    let legacy_operation = inject_legacy_provision(&f);

    let result = f
        .cli("reconcile-helper", json!({"operation":legacy_operation}))
        .unwrap();

    assert_eq!(result["confirmed"], false);
    assert_eq!(result["binding_rejected"], true);
    let recovered = f.state();
    let operation = recovered
        .operations
        .iter()
        .find(|operation| operation.id == legacy_operation)
        .unwrap();
    assert_eq!(operation.status, OpStatus::Failed);
    assert_eq!(
        operation.observation.as_ref().unwrap()["checkout"]["clean"],
        true
    );
    assert_eq!(
        operation.observation.as_ref().unwrap()["unfinished_owner"],
        "TASK-1"
    );
    assert_eq!(recovered.tasks["TASK-1"].slot.as_deref(), Some("worker1"));
    assert!(recovered.tasks["TASK-2"].slot.is_none());
    assert_eq!(task_counters(&recovered.tasks["TASK-1"]), first_counters);
    assert_eq!(task_counters(&recovered.tasks["TASK-2"]), second_counters);
    assert_eq!(fs::read(old_path.join("source.txt")).unwrap(), preserved);
    let settled = serde_json::to_value(&recovered).unwrap();
    assert!(
        f.cli("reconcile-helper", json!({"operation":legacy_operation}),)
            .is_err()
    );
    assert_eq!(serde_json::to_value(f.state()).unwrap(), settled);

    install_worktree_helper(&f);
    f.cli("reserve-slot", json!({"slot":"worker2"})).unwrap();
    let allocation = f
        .cli(
            "provision-slot",
            json!({"slot":"worker2","branch":"agent-test-linux/worker2/TASK-2-change"}),
        )
        .unwrap();
    assert_eq!(allocation["action"], "prepare-loop-plan");
    let allocated = f.state();
    assert_eq!(allocated.tasks["TASK-1"].slot.as_deref(), Some("worker1"));
    assert_eq!(allocated.tasks["TASK-2"].slot.as_deref(), Some("worker2"));
    assert_eq!(fs::read(old_path.join("source.txt")).unwrap(), preserved);
}

#[test]
#[cfg(target_os = "linux")]
fn uncertain_provision_cannot_confirm_a_different_already_bound_slot() {
    let f = Fixture::new();
    f.bound(false);
    let path = f.repo.join(".worktrees/worker2");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "agent-test-linux/worker2/TASK-1-change",
            path.to_str().unwrap(),
            "origin/main",
        ],
    )
    .unwrap();
    let operation = inject_provision(
        &f,
        "TASK-1",
        "worker2",
        "agent-test-linux/worker2/TASK-1-change",
    );
    let before = serde_json::to_value(f.state()).unwrap();

    assert!(
        f.cli("reconcile-helper", json!({"operation":operation}))
            .is_err()
    );
    assert_eq!(serde_json::to_value(f.state()).unwrap(), before);
}

#[test]
#[cfg(target_os = "linux")]
fn legacy_collision_does_not_bypass_active_task_guard() {
    let f = Fixture::new();
    prepare_second_task_after_first_slot_disappears(&f);
    let path = f.repo.join(".worktrees/worker1");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "agent-test-linux/worker1/TASK-2-change",
            path.to_str().unwrap(),
            "origin/main",
        ],
    )
    .unwrap();
    let operation = inject_legacy_provision(&f);
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("fixture-active-task-drift", |state| {
            state.active = Some("TASK-1".into());
            Ok(json!({"changed":true}))
        })
        .unwrap();
    let before = serde_json::to_value(f.state()).unwrap();

    assert!(
        f.cli("reconcile-helper", json!({"operation":operation}))
            .is_err()
    );
    assert_eq!(serde_json::to_value(f.state()).unwrap(), before);
}

#[cfg(target_os = "linux")]
fn prepare_second_task_after_first_slot_disappears(f: &Fixture) {
    f.bound(false);
    f.cli(
        "hold",
        json!({"key":"TASK-1","outcome":"needs-human","reason":"preserve unfinished fixture work"}),
    )
    .unwrap();
    let first_path = f.repo.join(".worktrees/worker1");
    git(
        &f.repo,
        &["worktree", "remove", first_path.to_str().unwrap()],
    )
    .unwrap();
    let mut first = Fixture::spec("TASK-1", 1);
    first["jira_status"] = json!("2");
    f.cli(
        "refresh",
        json!({"tasks":[first,Fixture::spec("TASK-2",2)]}),
    )
    .unwrap();
    f.cli("claim", json!({"key":"TASK-2"})).unwrap();
    f.cli(
        "jira-prepare",
        json!({"key":"TASK-2","issue":"TASK-2","current":"2","resolved":false,"target":"2","transitions":[]}),
    )
    .unwrap();
    f.cli(
        "brief",
        json!({"key":"TASK-2","criteria":[{"id":"AC2","text":"second observable behavior","human_only":false}],"tier":"routine","reason":"isolated second behavior"}),
    )
    .unwrap();
}

#[cfg(target_os = "linux")]
fn install_worktree_helper(f: &Fixture) {
    fs::create_dir_all(f.repo.join("scripts")).unwrap();
    fs::write(
        f.repo.join("scripts/worktree-add.sh"),
        "#!/bin/sh\nset -eu\ntouch \"$FAKE_ROOT/worktree-helper-ran\"\ngit worktree add --detach \".worktrees/$1\" origin/main\n",
    )
    .unwrap();
}

#[cfg(target_os = "linux")]
fn inject_legacy_provision(f: &Fixture) -> u64 {
    // Fault injection represents the retained operation from a controller version
    // that ran the helper before checking unfinished ledger ownership.
    inject_provision(
        f,
        "TASK-2",
        "worker1",
        "agent-test-linux/worker1/TASK-2-change",
    )
}

#[cfg(target_os = "linux")]
fn inject_provision(f: &Fixture, key: &str, slot: &str, branch: &str) -> u64 {
    // Fault injection represents retained recovery state that normal commands
    // cannot create after the provisioning preflight repair.
    let mut store = Store::open(&f.root).unwrap();
    store
        .change("fixture-legacy-provision", |state| {
            let id = state.id();
            state.operations.push(Operation {
                id,
                key: key.into(),
                kind: "provision".into(),
                status: OpStatus::Unknown,
                created: now(),
                snapshot: None,
                attempts: 1,
                details: json!({"slot":slot,"branch":branch}),
                observation: None,
                process: Some(ProcessIdentity {
                    pid: 2_000_000_000,
                    start: "old".into(),
                    group: 2_000_000_000,
                    terminal: true,
                    marker: "fixture".into(),
                    exit_success: Some(true),
                }),
            });
            Ok(json!({"id":id}))
        })
        .unwrap()["id"]
        .as_u64()
        .unwrap()
}

fn task_counters(task: &Task) -> (u32, u32, u32, u32) {
    (
        task.reviews,
        task.repairs,
        task.ci_repairs,
        task.escalations,
    )
}
#[test]
#[cfg(target_os = "linux")]
fn startup_crash_before_gate_is_recoverable_without_claiming_execution() {
    let f = Fixture::new();
    f.bound(false);
    // Fault injection models an external or interrupted state unavailable through normal commands.
    let mut store = Store::open(&f.root).unwrap();
    let id = store
        .change("fixture-unreleased", |s| {
            let id = s.id();
            s.operations.push(Operation {
                id,
                key: "TASK-1".into(),
                kind: "check".into(),
                status: OpStatus::Unknown,
                created: now(),
                snapshot: None,
                attempts: 1,
                details: json!({}),
                observation: None,
                process: None,
            });
            Ok(json!({"id":id}))
        })
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let result = f
        .cli(
            "recover-operation",
            json!({"operation":id,"terminate":false}),
        )
        .unwrap();
    assert_eq!(result["command_executed"], false);
    assert_eq!(f.state().operations[0].status, OpStatus::Failed);
}
#[test]
#[cfg(target_os = "linux")]
fn surviving_owned_descendant_can_be_stopped_after_leader_exits() {
    let f = Fixture::new();
    f.bound(false);
    assert!(f.cli("run-check",json!({"key":"TASK-1","criteria":["AC1"],"argv":["sh","-c","sleep 30 & exit 0"],"cwd":f.repo.join(".worktrees/worker1"),"timeout_seconds":10,"implementation":["source.txt:1"]})).is_err());
    let id = f.state().operations[0].id;
    f.cli(
        "recover-operation",
        json!({"operation":id,"terminate":true}),
    )
    .unwrap();
    assert_eq!(f.state().operations[0].status, OpStatus::Failed);
}
#[test]
#[cfg(target_os = "linux")]
fn complete_delivery_requires_merge_review_jira_and_allows_cleanup_check() {
    let f = Fixture::new();
    f.complete_delivery();
    assert!(f.state().active.is_none());
    assert_eq!(f.state().tasks["TASK-1"].delivery, Delivery::Merged);
    f.call(json!({"command":"cleanup-check","key":"TASK-1"}))
        .unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn completed_delivery_stays_stable_after_worker_reset_reuse_and_removal() {
    let f = Fixture::new();
    let head = f.complete_delivery();
    let completed = f.state();
    let task = &completed.tasks["TASK-1"];
    let accounting = (
        task.reviews,
        task.repairs,
        task.ci_repairs,
        task.escalations,
        completed.launches.len(),
        task.review.as_ref().unwrap().launch,
    );
    fs::write(f.repo.join("later.txt"), "later main work\n").unwrap();
    git(&f.repo, &["add", "."]).unwrap();
    git(&f.repo, &["commit", "-m", "test: advance main"]).unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();

    let worker = f.repo.join(".worktrees/worker1");
    git(&worker, &["fetch", "origin", "main"]).unwrap();
    git(&worker, &["reset", "--hard", "origin/main"]).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    f.assert_completed_delivery(&head, accounting);

    git(&worker, &["checkout", "-B", "worker1", "origin/main"]).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    f.assert_completed_delivery(&head, accounting);

    git(&f.repo, &["worktree", "remove", worker.to_str().unwrap()]).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    f.assert_completed_delivery(&head, accounting);
}

#[test]
#[cfg(target_os = "linux")]
fn completed_delivery_rejects_open_and_closed_remote_pr_without_mutation() {
    let f = Fixture::new();
    let head = f.complete_delivery();
    let completed = f.state();
    let task = &completed.tasks["TASK-1"];
    let accounting = (
        task.reviews,
        task.repairs,
        task.ci_repairs,
        task.escalations,
        completed.launches.len(),
        task.review.as_ref().unwrap().launch,
    );
    let mut pr: Value = serde_json::from_slice(&fs::read(f.root.join("pr.json")).unwrap()).unwrap();

    for state in ["OPEN", "CLOSED"] {
        pr["state"] = json!(state);
        pr["isDraft"] = json!(state == "OPEN");
        pr["mergeCommit"] = Value::Null;
        fs::write(f.root.join("pr.json"), serde_json::to_vec(&pr).unwrap()).unwrap();

        assert!(f.cli("gh-observe", json!({"key":"TASK-1"})).is_err());
        f.assert_completed_delivery(&head, accounting);
    }
}

#[test]
#[cfg(target_os = "linux")]
fn completed_delivery_rejects_remote_head_and_merge_identity_drift() {
    let f = Fixture::new();
    let head = f.complete_delivery();
    let completed = f.state();
    let task = &completed.tasks["TASK-1"];
    let accounting = (
        task.reviews,
        task.repairs,
        task.ci_repairs,
        task.escalations,
        completed.launches.len(),
        task.review.as_ref().unwrap().launch,
    );
    fs::write(f.repo.join("later.txt"), "later main work\n").unwrap();
    git(&f.repo, &["add", "."]).unwrap();
    git(&f.repo, &["commit", "-m", "test: advance main"]).unwrap();
    let later = sha(&f.repo, "HEAD").unwrap();
    git(&f.repo, &["push", "origin", "main"]).unwrap();

    let mut pr: Value = serde_json::from_slice(&fs::read(f.root.join("pr.json")).unwrap()).unwrap();
    pr["headRefOid"] = json!(later);
    fs::write(f.root.join("pr.json"), serde_json::to_vec(&pr).unwrap()).unwrap();
    assert!(f.cli("gh-observe", json!({"key":"TASK-1"})).is_err());
    f.assert_completed_delivery(&head, accounting);

    pr["headRefOid"] = json!(head);
    pr["mergeCommit"] = json!({"oid":later});
    fs::write(f.root.join("pr.json"), serde_json::to_vec(&pr).unwrap()).unwrap();
    assert!(f.cli("gh-observe", json!({"key":"TASK-1"})).is_err());
    f.assert_completed_delivery(&head, accounting);
}

#[test]
#[cfg(target_os = "linux")]
fn premature_external_merge_without_acceptance_is_held() {
    let f = Fixture::new();
    f.bound(false);
    f.changed();
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

    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    let state = f.state();
    let task = &state.tasks["TASK-1"];
    assert_eq!(task.delivery, Delivery::NeedsHuman);
    assert!(task.main_verified);
    assert!(!task.final_verified);
    assert!(engine::reviewed(&state, task).is_err());
    assert_eq!((task.reviews, task.repairs), (0, 0));
}

#[test]
fn already_satisfied_work_routes_to_final_verification_without_empty_pr() {
    let f = Fixture::new();
    f.bound(false);
    f.pass_review();
    assert_eq!(
        engine::next(&f.state(), now())["action"],
        "final-acceptance-verification"
    );
}

#[test]
fn legacy_state_requires_configuration_without_resetting_budgets() {
    let f = Fixture::new();
    f.bound(false);
    let mut old = serde_json::to_value(f.state()).unwrap();
    old["version"] = json!(1);
    old.as_object_mut().unwrap().remove("loop_policy");
    old["tasks"]["TASK-1"]
        .as_object_mut()
        .unwrap()
        .remove("loop_state");
    old["tasks"]["TASK-1"]["reviews"] = json!(2);
    rusqlite::Connection::open(f.root.join("state.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE state SET data=?1 WHERE id=1",
            [serde_json::to_string(&old).unwrap()],
        )
        .unwrap();
    let mut legacy = f.state();
    assert_eq!(engine::next(&legacy, now())["action"], "configure-loop");
    assert!(engine::acceptance(&legacy, &legacy.tasks["TASK-1"]).is_err());
    engine::apply(
        &mut legacy,
        Input::ConfigureLoop {
            policy: Default::default(),
        },
        now(),
    )
    .unwrap();
    assert_eq!(legacy.tasks["TASK-1"].reviews, 2);
    f.call(json!({"command":"configure-loop","policy":{}}))
        .unwrap();
    assert_eq!(f.state().version, 2);
    assert_eq!(f.state().tasks["TASK-1"].reviews, 2);
    assert_eq!(engine::next(&legacy, now())["action"], "prepare-loop-plan");
    assert!(
        f.call(json!({"command":"configure-loop","policy":{"max_open_prs":99}}))
            .is_err()
    );
}

#[test]
fn measurements_are_required_even_when_a_command_passed() {
    let f = Fixture::new();
    f.bound(false);
    let log = f.root.join("check.txt");
    fs::write(&log, "executable exited zero").unwrap();
    f.call(json!({"command":"evidence","key":"TASK-1","evidence":{"criterion":"AC1","status":"passed","kind":"command","artifact":log,"implementation":["source.txt"],"description":"command result","human_source":null}})).unwrap();
    assert!(engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).is_err());
    f.call(json!({"command":"measure","key":"TASK-1","criterion":"AC1","observed":"expected behavior still absent","satisfied":false,"artifact":log})).unwrap();
    assert!(engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).is_err());
    f.evidence();
    engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).unwrap();
    fs::write(f.root.join("baseline.txt"), "weakened baseline").unwrap();
    assert!(engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).is_err());
}

#[test]
fn replanning_cannot_rewrite_the_starting_condition() {
    let f = Fixture::new();
    f.bound(false);
    let mut plan = serde_json::to_value(
        &f.state().tasks["TASK-1"]
            .loop_state
            .plan
            .as_ref()
            .unwrap()
            .input,
    )
    .unwrap();
    plan["baselines"][0]["target"] = json!("weaker target");
    assert!(
        f.call(json!({"command":"loop-plan","key":"TASK-1","plan":plan}))
            .is_err()
    );
    assert_eq!(
        f.state().tasks["TASK-1"]
            .loop_state
            .plan
            .as_ref()
            .unwrap()
            .input
            .baselines[0]
            .target,
        "observable behavior"
    );
}

#[test]
fn stalled_implementation_is_bounded_across_reopen_and_replan() {
    let f = Fixture::new();
    f.bound(false);
    for turn in 0..2 {
        let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "implementer", "completed", None).unwrap();
        assert!(f.begin("implementer").is_err());
        let log = f.root.join(format!("stalled-{turn}.txt"));
        fs::write(&log, "same failing observation, no uncertainty resolved").unwrap();
        f.call(json!({"command":"checkpoint","key":"TASK-1","advanced":false,"finding":"same failure persists","next_step":"diagnose missing runtime prerequisite","artifact":log})).unwrap();
    }
    assert_eq!(f.state().tasks["TASK-1"].loop_state.stalled, 2);
    assert!(f.begin("implementer").is_err());
    f.loop_plan();
    assert!(f.begin("implementer").is_err());
    assert_eq!(f.state().launches.len(), 2);
    f.call(json!({"command":"hold","key":"TASK-1","outcome":"needs-human","reason":"Provide the missing runtime prerequisite"})).unwrap();
    f.call(
        json!({"command":"refresh","tasks":[Fixture::spec("TASK-1",1),Fixture::spec("TASK-2",2)]}),
    )
    .unwrap();
    f.call(json!({"command":"resume","key":"TASK-1"})).unwrap();
    assert!(f.begin("implementer").is_err());
}

#[test]
fn implementation_limit_counts_cancelled_attempts_and_still_allows_validation() {
    let f = Fixture::new();
    f.call(json!({"command":"configure-loop","policy":{"max_implementation_turns":1}}))
        .unwrap();
    f.bound(false);
    let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
    f.finish(id, "implementer", "cancelled", None).unwrap();
    f.checkpoint();
    assert!(f.begin("implementer").is_err());
    f.pass_review();
    assert_eq!(f.state().launches.len(), 2);
}

#[test]
fn new_evidence_resets_stall_streak_but_duplicate_evidence_does_not() {
    let f = Fixture::new();
    f.bound(false);
    let log = f.root.join("diagnosis.txt");
    fs::write(&log, "a measured failed contract").unwrap();
    for advanced in [false, true] {
        let id = f.begin("implementer").unwrap()["id"].as_u64().unwrap();
        f.finish(id, "implementer", "completed", None).unwrap();
        let request = json!({"command":"checkpoint","key":"TASK-1","advanced":advanced,"finding":"isolated the failing boundary","next_step":"correct this boundary","artifact":log});
        if advanced {
            assert!(f.call(request.clone()).is_err());
            fs::write(&log, "new observation isolates the faulty caller").unwrap();
        }
        f.call(request).unwrap();
    }
    assert_eq!(f.state().tasks["TASK-1"].loop_state.stalled, 0);
}

#[test]
fn scope_checkpoint_reports_outside_paths_and_requires_reassessment() {
    let f = Fixture::new();
    f.bound(false);
    let path = f.repo.join(".worktrees/worker1");
    fs::write(path.join("helper.txt"), "necessary helper\n").unwrap();
    git(&path, &["add", "."]).unwrap();
    git(&path, &["commit", "-m", "add helper"]).unwrap();
    f.call(json!({"command":"snapshot","key":"TASK-1"}))
        .unwrap();
    let log = f.root.join("scope.txt");
    fs::write(&log, "helper implements the same accepted contract").unwrap();
    let mut request = json!({"command":"checkpoint","key":"TASK-1","advanced":true,"finding":"helper supplies missing contract","next_step":"validate contract","artifact":log});
    assert!(f.call(request.clone()).is_err());
    request["scope_reason"] =
        json!("One small helper is necessary for the original contract; no business scope added");
    let response = f.call(request).unwrap();
    assert_eq!(
        response["checkpoint"]["outside_plan"],
        json!(["helper.txt"])
    );
    assert_eq!(response["checkpoint"]["size"]["added"], 1);
    f.pass_review();
    fs::write(path.join("helper.txt"), "changed contract\n").unwrap();
    git(&path, &["add", "."]).unwrap();
    git(&path, &["commit", "-m", "integration correction"]).unwrap();
    f.call(json!({"command":"snapshot","key":"TASK-1"}))
        .unwrap();
    f.evidence();
    assert!(engine::acceptance(&f.state(), &f.state().tasks["TASK-1"]).is_err());
    // A new integration snapshot can be assessed without pretending another agent turn ran.
    fs::write(&log, "new integration still satisfies the original scope").unwrap();
    f.call(json!({"command":"checkpoint","key":"TASK-1","advanced":true,"finding":"integration assessed","next_step":"review integration","scope_reason":"One helper remains the same concern","artifact":log})).unwrap();
    assert_eq!(f.state().tasks["TASK-1"].loop_state.stalled, 0);
}

#[test]
fn unfinished_pr_limit_stops_claims_but_allows_existing_work_to_resume() {
    let f = Fixture::new();
    f.call(json!({"command":"configure-loop","policy":{"max_open_prs":1}}))
        .unwrap();
    f.claimed();
    Store::open(&f.root)
        .unwrap()
        .change("fixture-open-pr", |s| {
            let t = s.tasks.get_mut("TASK-1").unwrap();
            t.pr = Some(12);
            t.pr_open = true;
            Ok(json!({}))
        })
        .unwrap();
    f.call(json!({"command":"hold","key":"TASK-1","outcome":"needs-human","reason":"Await acceptance"})).unwrap();
    f.call(
        json!({"command":"refresh","tasks":[Fixture::spec("TASK-1",1),Fixture::spec("TASK-2",2)]}),
    )
    .unwrap();
    assert_eq!(
        engine::next(&f.state(), now())["action"],
        "recover-unfinished-prs"
    );
    assert!(f.call(json!({"command":"claim","key":"TASK-2"})).is_err());
    f.call(json!({"command":"resume","key":"TASK-1"})).unwrap();
    assert_eq!(f.state().active.as_deref(), Some("TASK-1"));
}

#[test]
fn human_review_mode_requires_a_current_receipt_and_invalidates_it_on_changes() {
    let f = Fixture::new();
    f.call(json!({"command":"configure-loop","policy":{"review_mode":"human-review"}}))
        .unwrap();
    f.bound(false);
    f.pass_review();
    assert_eq!(
        engine::next(&f.state(), now())["action"],
        "await-human-review"
    );
    let receipt = f.root.join("human-review.txt");
    fs::write(
        &receipt,
        "Fixture human approved the recorded exact snapshot",
    )
    .unwrap();
    let snap = f.state().tasks["TASK-1"].snapshot.clone().unwrap();
    let final_request =
        json!({"command":"final-verify","key":"TASK-1","main_commit":snap.head,"evidence":receipt});
    assert!(f.call(final_request.clone()).is_err());
    f.call(json!({"command":"human-review","key":"TASK-1","snapshot":snap,"source":"fixture user message 1","artifact":receipt})).unwrap();
    epic_control::loops::human_review(&f.state(), &f.state().tasks["TASK-1"]).unwrap();
    fs::write(&receipt, "changed receipt").unwrap();
    assert!(f.call(final_request).is_err());
    f.changed();
    assert!(f.state().tasks["TASK-1"].loop_state.human_review.is_none());
}

#[test]
#[cfg(target_os = "linux")]
fn scoped_feedback_is_pinned_and_injected_into_native_and_managed_launches() {
    let f = Fixture::new();
    let lesson = |id: &str, family: &str, status: &str| json!({"id":id,"families":[family],"source":"fixture review","instruction":format!("Instruction {id}"),"example":"source.txt","status":status,"acceptance_source":if status=="accepted" {Some("fixture human decision")}else{None}});
    fs::write(
        f.repo.join("feedback.json"),
        serde_json::to_vec(&json!([
            lesson("local", "fixture", "accepted"),
            lesson("other", "unrelated", "accepted"),
            lesson("draft", "fixture", "proposed")
        ]))
        .unwrap(),
    )
    .unwrap();
    git(&f.repo, &["add", "feedback.json"]).unwrap();
    git(&f.repo, &["commit", "-m", "approved feedback"]).unwrap();
    git(
        &f.repo,
        &[
            "update-ref",
            "refs/remotes/origin/main",
            &sha(&f.repo, "HEAD").unwrap(),
        ],
    )
    .unwrap();
    f.call(json!({"command":"configure-loop","policy":{"feedback_file":"feedback.json"}}))
        .unwrap();
    f.bound(false);
    let native = f.begin("implementer").unwrap();
    assert_eq!(
        native["context"]["plan"]["feedback"]["lessons"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        native["context"]["plan"]["feedback"]["lessons"][0]["id"],
        "local"
    );
    let id = native["id"].as_u64().unwrap();
    f.finish(id, "controlled-session", "completed", None)
        .unwrap();
    f.checkpoint();
    // Even edits to main's working file cannot change the already committed, pinned task context.
    fs::write(f.repo.join("feedback.json"), "not approved or committed").unwrap();
    f.fake_tools();
    let prompt = f.root.join("brief.txt");
    fs::write(&prompt, "Inspect the source").unwrap();
    f.cli("run-agent",json!({"key":"TASK-1","role":"implementer","model":"gpt-5.6-terra","effort":"medium","reason":"","ci_repair":false,"prompt":prompt,"timeout_seconds":10})).unwrap();
    let received = fs::read_to_string(f.root.join("received-prompt.txt")).unwrap();
    assert!(received.contains("Instruction local"));
    assert!(!received.contains("Instruction other") && !received.contains("Instruction draft"));
    assert_eq!(
        f.state().launches[0].context["plan"]["feedback"],
        f.state().launches[1].context["plan"]["feedback"]
    );
}

#[test]
fn proposed_lessons_require_verified_delivery_before_reuse() {
    let f = Fixture::new();
    f.bound(false);
    let mut lesson = json!({"id":"shared-contract","families":["fixture"],"source":"binding fixture contract and review finding","instruction":"Follow the existing source contract","example":"source.txt","status":"proposed","acceptance_source":null});
    f.call(json!({"command":"lesson-record","key":"TASK-1","lesson":lesson}))
        .unwrap();
    f.loop_plan();
    assert!(
        f.state().tasks["TASK-1"]
            .loop_state
            .plan
            .as_ref()
            .unwrap()
            .feedback
            .run_lessons
            .is_empty()
    );
    lesson["status"] = json!("accepted");
    lesson["acceptance_source"] =
        json!("Coordinator verified this repeats the binding source rule after independent PASS");
    assert!(
        f.call(json!({"command":"lesson-record","key":"TASK-1","lesson":lesson}))
            .is_err()
    );
    f.pass_review();
    let proof = f.root.join("final-proof.txt");
    fs::write(&proof, "final source contract checked at main").unwrap();
    f.call(json!({"command":"final-verify","key":"TASK-1","main_commit":f.state().tasks["TASK-1"].snapshot.as_ref().unwrap().head,"evidence":proof})).unwrap();
    f.call(json!({"command":"lesson-record","key":"TASK-1","lesson":lesson}))
        .unwrap();
    assert!(
        f.call(json!({"command":"lesson-record","key":"TASK-1","lesson":lesson}))
            .is_err()
    );
    f.loop_plan();
    let state = f.state();
    let packet = &state.tasks["TASK-1"]
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .feedback;
    assert_eq!(packet.run_lessons.len(), 1);
    assert_eq!(packet.run_lessons[0].key, "TASK-1");
    assert!(packet.run_lessons[0].main_commit.is_some());
    assert!(packet.file.is_none());
}

#[test]
#[cfg(target_os = "linux")]
fn human_mode_guards_merge_preparation_and_execution() {
    let f = Fixture::new();
    f.call(json!({"command":"configure-loop","policy":{"review_mode":"human-review"}}))
        .unwrap();
    f.bound(false);
    f.changed();
    f.pass_review();
    f.fake_tools();
    fs::write(f.root.join("created"), "exists").unwrap();
    fs::copy(f.root.join("ready.json"), f.root.join("pr.json")).unwrap();
    f.cli("gh-observe", json!({"key":"TASK-1"})).unwrap();
    f.cli("poll-checks", json!({"key":"TASK-1"})).unwrap();
    let request = json!({"command":"gh-prepare","key":"TASK-1","action":"merge","title":null,"body":null,"method":"merge"});
    assert!(f.call(request.clone()).is_err());
    let receipt = f.root.join("review-receipt.txt");
    fs::write(&receipt, "Human approved this precise snapshot").unwrap();
    f.call(json!({"command":"human-review","key":"TASK-1","snapshot":f.state().tasks["TASK-1"].snapshot,"source":"fixture human decision","artifact":receipt})).unwrap();
    let id = f.call(request).unwrap()["id"].as_u64().unwrap();
    fs::write(&receipt, "tampered receipt").unwrap();
    let err = f.cli("gh-run", json!({"operation":id})).unwrap_err();
    assert!(err.to_string().contains("receipt changed"));
    assert_eq!(
        f.state().operations.last().unwrap().status,
        OpStatus::Prepared
    );
    assert!(!f.state().tasks["TASK-1"].main_verified);
}

mod documentation_budget;
