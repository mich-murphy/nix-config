use crate::{engine, model::*, store::*};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy)]
enum ProcessOwner {
    Launch(u64),
    Operation(u64),
}
fn set_process(s: &mut State, owner: ProcessOwner, process: ProcessIdentity) -> Result<()> {
    match owner {
        ProcessOwner::Launch(id) => {
            s.launches
                .iter_mut()
                .find(|l| l.id == id)
                .context("launch")?
                .process = Some(process)
        }
        ProcessOwner::Operation(id) => {
            s.operations
                .iter_mut()
                .find(|o| o.id == id)
                .context("operation")?
                .process = Some(process)
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_start(pid: u32) -> Result<String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    Ok(stat
        .rsplit_once(") ")
        .context("invalid process stat")?
        .1
        .split_whitespace()
        .nth(19)
        .context("missing process start time")?
        .to_owned())
}
#[cfg(not(target_os = "linux"))]
fn process_start(_pid: u32) -> Result<String> {
    bail!(
        "owned process supervision currently requires Linux; use the native agent bridge on other hosts"
    )
}
#[cfg(unix)]
fn group_alive(group: u32) -> Result<bool> {
    // kill(signal=0) checks only this recorded process group; it sends no signal.
    // SAFETY: kill with signal 0 only probes the checked numeric process-group ID.
    let result = unsafe { libc::kill(-(i32::try_from(group)?), 0) };
    if result == 0 {
        let members = Command::new("pgrep")
            .args(["-g", &group.to_string()])
            .output()?;
        if members.status.code() == Some(1) {
            return Ok(false);
        }
        ensure!(
            members.status.success(),
            "cannot inspect the owned process group"
        );
        for pid in String::from_utf8(members.stdout)?.split_whitespace() {
            let pid: u32 = pid.parse()?;
            match fs::read_to_string(format!("/proc/{pid}/stat")) {
                Ok(stat) => {
                    if stat
                        .rsplit_once(") ")
                        .context("process stat")?
                        .1
                        .split_whitespace()
                        .next()
                        != Some("Z")
                    {
                        return Ok(true);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        return Ok(false);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(false)
    } else {
        Err(error.into())
    }
}
#[cfg(not(unix))]
fn group_alive(_group: u32) -> Result<bool> {
    bail!("process group checks require Unix")
}
#[cfg(unix)]
fn stop_group(process: &ProcessIdentity) -> Result<()> {
    if let Ok(start) = process_start(process.pid) {
        ensure!(
            start == process.start,
            "process identity changed; refusing to signal it"
        );
    } else {
        required_text(&process.marker, "owned descendant marker")?;
        // Query only the recorded group, never enumerate unrelated worker processes.
        let members = Command::new("pgrep")
            .args(["-g", &process.group.to_string()])
            .output()?;
        ensure!(
            members.status.success(),
            "cannot establish surviving owned descendants"
        );
        let marker = format!("EPIC_CONTROL_OWNER={}", process.marker);
        let pids = String::from_utf8(members.stdout)?;
        for pid in pids.split_whitespace() {
            let pid: u32 = pid.parse()?;
            let environment = fs::read(format!("/proc/{pid}/environ"))?;
            ensure!(
                environment
                    .split(|b| *b == 0)
                    .any(|entry| entry == marker.as_bytes()),
                "process group contains an unverified descendant; refusing to signal it"
            );
        }
    }
    // SAFETY: ownership and start identity were verified above; kill takes no pointers.
    let result = unsafe { libc::kill(-(i32::try_from(process.group)?), libc::SIGKILL) };
    ensure!(result == 0, "cannot stop owned process group");
    Ok(())
}
#[cfg(not(unix))]
fn stop_group(_process: &ProcessIdentity) -> Result<()> {
    bail!("process groups require Unix")
}

// Child output goes to files, so a quiet parent or a full pipe cannot deadlock a job.
// Callers yield their outer exec tool while this foreground process supervises its child.
fn capture(
    command: &mut Command,
    root: &Path,
    label: &str,
    seconds: u64,
) -> Result<(bool, String)> {
    capture_owned(command, root, label, seconds, None)
}
fn capture_owned(
    command: &mut Command,
    root: &Path,
    label: &str,
    seconds: u64,
    mut store: Option<(&mut Store, ProcessOwner)>,
) -> Result<(bool, String)> {
    ensure!(
        cfg!(target_os = "linux"),
        "built-in process supervision currently requires Linux; use native tool receipts on other hosts"
    );
    let unique = format!(
        "{}-{}-{}-{}",
        label,
        std::process::id(),
        now(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    );
    let out_path = root.join(format!("{unique}.stdout"));
    let err_path = root.join(format!("{unique}.stderr"));
    let out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out_path)?;
    let err = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&err_path)?;
    let gate = root.join(format!("{unique}.gate"));
    let marker = hash(unique.as_bytes());
    *command = owned_wrapper(command, &gate, &marker)?;
    let spawned = command
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            if let Some((db, owner)) = &mut store {
                db.change("definite-spawn-failure", |s| {
                    match owner {
                        ProcessOwner::Launch(id) => {
                            let l = s.launches.iter_mut().find(|l| l.id == *id).unwrap();
                            l.ended = Some(now());
                            l.outcome = Some("failed-before-start".into());
                        }
                        ProcessOwner::Operation(id) => {
                            let op = s.operations.iter_mut().find(|o| o.id == *id).unwrap();
                            op.status = OpStatus::Failed;
                            op.observation = Some(json!({"spawn_failed":true}));
                        }
                    }
                    Ok(json!({"spawn_failed":true}))
                })?;
            }
            return Err(error.into());
        }
    };
    let process = ProcessIdentity {
        pid: child.id(),
        start: process_start(child.id())?,
        group: child.id(),
        terminal: false,
        marker,
        exit_success: None,
    };
    fs::write(
        root.join(format!("{unique}.process.json")),
        serde_json::to_vec(&process)?,
    )?;
    if let Some((db, owner)) = &mut store {
        db.change("process-start", |s| {
            set_process(s, *owner, process.clone())?;
            Ok(json!(process))
        })?;
    }
    // The wrapper cannot execute the requested command until its identity is durable.
    let mut gate_file = OpenOptions::new().write(true).create_new(true).open(gate)?;
    gate_file.write_all(process.marker.as_bytes())?;
    gate_file.sync_all()?;
    wait_owned(&mut child, &process, &out_path, root, label, seconds, store)
}

fn gh(root: &Path, repo: &str, args: &[String]) -> Result<(bool, String)> {
    let mut cmd = Command::new("gh");
    cmd.args(args).args(["--repo", repo]);
    capture(&mut cmd, root, "gh", 120)
}
fn gh_json(root: &Path, repo: &str, args: &[String]) -> Result<Value> {
    let (ok, text) = gh(root, repo, args)?;
    ensure!(ok, "GitHub read failed; inspect retained logs");
    Ok(serde_json::from_str(&text)?)
}
fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}
fn locate(store: &Store, key: &str) -> Result<Option<Value>> {
    let s = store.read()?;
    let t = s.tasks.get(key).context("unknown task")?;
    let fields = "number,state,isDraft,headRefOid,headRefName,baseRefName,mergeCommit,url";
    if let Some(pr) = t.pr {
        return Ok(Some(gh_json(
            &store.root,
            &s.github_repo,
            &args(&["pr", "view", &pr.to_string(), "--json", fields]),
        )?));
    }
    let branch = t.branch.as_ref().context("no branch")?;
    let list = gh_json(
        &store.root,
        &s.github_repo,
        &args(&[
            "pr", "list", "--head", branch, "--base", "main", "--state", "all", "--limit", "100",
            "--json", fields,
        ]),
    )?;
    let list = list.as_array().context("invalid GitHub PR list")?;
    ensure!(
        list.len() <= 1,
        "multiple PRs match this run's branch; reconcile ownership"
    );
    Ok(list.first().cloned())
}
pub(crate) fn verify_pr(s: &State, key: &str, pr: &Value) -> Result<()> {
    let t = s.tasks.get(key).context("task")?;
    let number = pr["number"].as_u64().context("invalid PR number")?;
    ensure!(
        !t.delivery_history.iter().any(|d| d.task.pr == Some(number)),
        "historical PR cannot become the current delivery"
    );
    if let Some(expected) = t.pr {
        ensure!(number == expected, "unexpected PR number");
    }
    ensure!(
        pr["baseRefName"] == "main" && pr["headRefName"].as_str() == t.branch.as_deref(),
        "unexpected PR identity"
    );
    ensure!(
        pr["headRefOid"].as_str() == t.snapshot.as_ref().map(|x| x.head.as_ref()),
        "GitHub head drift; do not silently accept it"
    );
    if pr["state"] == "MERGED" {
        let merge = pr["mergeCommit"]["oid"]
            .as_str()
            .context("missing merge commit")?;
        if let Some(expected) = &t.merged_commit {
            ensure!(merge == expected.as_ref(), "GitHub merge commit drift");
        }
    }
    Ok(())
}

fn completed_pr_delivery(s: &State, key: &str) -> bool {
    let Some(t) = s.tasks.get(key) else {
        return false;
    };
    crate::reconciliation::completed(s, t)
}

pub fn observe_pr(store: &mut Store, key: &str) -> Result<Value> {
    let s = store.read()?;
    let pr = locate(store, key)?;
    let Some(pr) = pr else {
        return Ok(json!({"found":false}));
    };
    verify_pr(&s, key, &pr)?;
    let merged = pr["state"] == "MERGED";
    ensure!(
        !completed_pr_delivery(&s, key) || merged,
        "completed delivery requires a merged PR observation"
    );
    if merged {
        git(
            &s.repo,
            &[
                "fetch",
                "origin",
                "refs/heads/main:refs/remotes/origin/main",
            ],
        )?;
        on_main(
            &s.repo,
            pr["mergeCommit"]["oid"]
                .as_str()
                .context("missing merge commit")?,
        )?;
    }
    store.change("observe-github",|s|{
        verify_pr(s,key,&pr)?;
        let completed = completed_pr_delivery(s, key);
        ensure!(!completed || merged, "completed delivery requires a merged PR observation");
        let t=s.tasks.get_mut(key).context("task")?;
        t.pr=pr["number"].as_u64();t.pr_open=pr["state"]=="OPEN";t.draft=pr["isDraft"].as_bool().context("missing draft status")?;
        if merged {
            t.merged_commit=pr["mergeCommit"]["oid"].as_str().map(str::parse).transpose()?;t.main_verified=true;
            if !completed {t.next_action="Final acceptance verification, then Jira Done".into();}
        }
        // Reconciliation closes only operations whose postcondition is actually observed.
        for op in &mut s.operations {
            if op.key!=key||op.id<=crate::followup::identity(t)||op.kind!="github"||matches!(op.status,OpStatus::Confirmed|OpStatus::Failed){continue;}
            let satisfied=match op.details["action"].as_str(){Some("create-pr")=>true,Some("ready")=>!t.draft,Some("merge")=>merged,_=>false};
            if satisfied {op.status=OpStatus::Confirmed;op.observation=Some(pr.clone());}
        }
        if merged && !completed && (engine::reviewed(s,&s.tasks[key]).and_then(|()| crate::loops::human_review(s,&s.tasks[key]))).is_err() {
            let t=s.tasks.get_mut(key).unwrap();t.delivery=Delivery::NeedsHuman;t.final_verified=false;
            t.next_action="External merge has unmet review/acceptance; preserve merge evidence and synchronize Jira In Progress, then hold needs-human".into();
        }
        Ok(json!({"found":true,"pr":pr,"main_verified":merged}))
    })
}
pub fn run_gh(store: &mut Store, id: u64) -> Result<Value> {
    let (_, op) = current_delivery_operation(store, id)?;
    ensure!(op.kind == "github", "not a GitHub operation");
    let before = observe_pr(store, &op.key)?;
    let s = store.read()?;
    let op = s.operations.iter().find(|o| o.id == id).unwrap().clone();
    if op.status == OpStatus::Confirmed {
        return Ok(json!({"reconciled":true,"operation":id}));
    }
    ensure!(
        op.status == OpStatus::Prepared,
        "operation outcome is uncertain; observe it, do not replay automatically"
    );
    let t = s.tasks.get(&op.key).unwrap();
    validate_gh_snapshot(&s, t, &op)?;
    let action = op.details["action"].as_str().context("action")?;
    let command = match action {
        "create-pr" => {
            ensure!(
                before["found"] == false,
                "an existing PR must be reconciled"
            );
            create_pr_command(&op, t)?
        }
        "ready" => args(&["pr", "ready", &t.pr.context("PR missing")?.to_string()]),
        "merge" => merge_pr_command(store, &op, &before)?,
        _ => bail!("unknown GitHub action"),
    };
    store.change("dispatch-github", |s| {
        engine::apply(s, Input::Dispatch { operation: id }, now())
    })?;
    // A transport error intentionally leaves UNKNOWN. A later observation decides what happened.
    let mut cmd = Command::new("gh");
    cmd.args(&command).args(["--repo", &s.github_repo]);
    let root = store.root.clone();
    let (success, _) = capture_owned(
        &mut cmd,
        &root,
        &format!("github-operation-{id}"),
        120,
        Some((store, ProcessOwner::Operation(id))),
    )?;
    crate::followup::current_operation(&store.read()?, &op)?;
    let observed = observe_pr(store, &op.key)?;
    Ok(
        json!({"command_success":success,"observation":observed,"next":engine::next(&store.read()?,now())}),
    )
}
pub fn poll_checks(store: &mut Store, key: &str) -> Result<Value> {
    let s = store.read()?;
    let t = s.tasks.get(key).context("task")?;
    let pr = locate(store, key)?.context("PR missing")?;
    verify_pr(&s, key, &pr)?;
    ensure!(
        pr["isDraft"] == false,
        "mark ready before starting the CI deadline"
    );
    let observed_snapshot = t.snapshot.clone().context("snapshot")?;
    let (success, text) = gh(
        &store.root,
        &s.github_repo,
        &args(&[
            "pr",
            "checks",
            &t.pr.context("PR missing")?.to_string(),
            "--required",
            "--json",
            "name,bucket,link",
        ]),
    )?;
    let after = locate(store, key)?.context("PR disappeared while checking CI")?;
    verify_pr(&s, key, &after)?;
    let checks: Vec<Value> =
        serde_json::from_str(&text).context("required checks unavailable; do not infer success")?;
    ensure!(
        !checks.is_empty(),
        "no required checks returned; investigate branch protection instead of assuming green"
    );
    ensure!(
        checks.iter().all(|v| matches!(
            v["bucket"].as_str(),
            Some("pass" | "skipping" | "pending" | "fail" | "cancel")
        )),
        "unrecognized check state"
    );
    let pending = checks.iter().any(|v| v["bucket"] == "pending");
    let failed = checks
        .iter()
        .any(|v| matches!(v["bucket"].as_str(), Some("fail" | "cancel")));
    ensure!(
        success || pending || failed,
        "GitHub command failed without classified checks"
    );
    store.change("observe-checks",|s|{
        let t=s.tasks.get_mut(key).context("task")?;
        ensure!(t.snapshot.as_ref()==Some(&observed_snapshot),"snapshot changed while observing checks");
        let head=observed_snapshot.head.clone();
        t.checks=checks.clone();t.checks_head=Some(head.to_string());
        if pending {t.deadlines.entry(head.to_string()).or_insert(now()+1800);}
        Ok(json!({"pending":pending,"failed":failed,"deadline":t.deadlines.get(head.as_ref()),"checks":checks}))
    })
}
pub fn drive(store: &mut Store) -> Result<Value> {
    loop {
        let state = store.read()?;
        let step = engine::next(&state, now());
        if step["action"] != "poll-checks-and-merge" {
            return Ok(step);
        }
        let key = state.active.as_ref().context("active task")?;
        let checks = poll_checks(store, key)?;
        if checks["pending"] != true || checks["failed"] == true {
            return Ok(
                json!({"action":if checks["failed"]==true{"diagnose-ci"}else{"prepare-merge"},"key":key,"checks":checks}),
            );
        }
        if checks["deadline"].as_u64().is_some_and(|d| now() >= d) {
            return store.change("defer-ci",|s|engine::apply(s,Input::Hold{key:key.parse()?,outcome:Delivery::Deferred,reason:"Required checks still pending at deadline; revisit this exact PR/head once before finishing".into()},now()));
        }
        eprintln!(
            "Required checks pending for {key}; deadline {}",
            checks["deadline"]
        );
        thread::sleep(Duration::from_secs(15));
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRequest {
    #[serde(default)]
    pub allow_evidence_gaps: bool,
    pub key: String,
    pub role: Role,
    pub model: String,
    pub effort: String,
    pub reason: String,
    pub ci_repair: bool,
    pub prompt: std::path::PathBuf,
    pub timeout_seconds: u64,
}
pub fn run_agent(store: &mut Store, r: AgentRequest) -> Result<Value> {
    ensure!(
        (1..=14400).contains(&r.timeout_seconds),
        "agent timeout must be 1..14400 seconds"
    );
    let prompt = fs::read_to_string(&r.prompt)?;
    required_text(&prompt, "agent brief")?;
    require_saved_authentication()?;
    let s = store.read()?;
    let t = s.tasks.get(&r.key).context("task")?;
    let cwd = worktree(&s, t)?;
    let launch = store.change("agent-begin", |s| {
        let result = engine::apply(
            s,
            Input::AgentBegin {
                allow_evidence_gaps: r.allow_evidence_gaps,
                key: r.key.parse()?,
                role: r.role.clone(),
                model: r.model.clone(),
                effort: r.effort.clone(),
                reason: r.reason.clone(),
                ci_repair: r.ci_repair,
            },
            now(),
        )?;
        let id = result["id"].as_u64().context("launch id")?;
        s.launches.iter_mut().find(|l| l.id == id).unwrap().managed = true;
        Ok(result)
    })?;
    let id = launch["id"].as_u64().context("launch id")?;
    let output = store.root.join(format!("agent-{id}.result.json"));
    let schema = store.root.join(format!("agent-{id}.schema.json"));
    let review_schema = crate::review_format::schema();
    fs::write(&schema, serde_json::to_vec(&review_schema)?)?;
    let mut cmd = codex_command(&r, &cwd, &launch, &schema, &output);
    let prompt_path = store.root.join(format!("agent-{id}.prompt.txt"));
    let instructions = format!(
        "You are the {:?} for {}. Read required repository instructions. Do not delegate. Do not call epic-control, edit its state, perform GitHub/Jira mutations, or clean worktrees. Stay within this role and task. Use existing authenticated tools only; no API billing changes. Exact snapshot: {}.\n\n{}",
        r.role, r.key, launch["snapshot"], prompt
    );
    let instructions = format!(
        "{}\n\nController-supplied task plan and scoped feedback:\n{}",
        instructions,
        serde_json::to_string_pretty(&launch["context"])?
    );
    fs::write(&prompt_path, &instructions)?;
    // File stdin avoids command interpolation and keeps potentially long briefs off argv.
    cmd.env("EPIC_CONTROL_INPUT", &prompt_path);
    let root = store.root.clone();
    let result = capture_owned(
        &mut cmd,
        &root,
        &format!("agent-{id}"),
        r.timeout_seconds,
        Some((store, ProcessOwner::Launch(id))),
    );
    let (ok, events) = match result {
        Ok(x) => x,
        Err(e) => return Err(e.context(
            "launch remains claimed; reconcile the recorded process/session before agent-finish",
        )),
    };
    let (session, usage) = agent_events(&events, launch["session"].as_str())?;
    let review = if ok && r.role == Role::Reviewer {
        Some(serde_json::from_slice::<ReviewOutput>(&fs::read(output)?)?)
    } else {
        None
    };
    store.change("agent-finish", |s| {
        engine::apply(
            s,
            Input::AgentFinish {
                launch: id,
                session,
                outcome: if ok { "completed" } else { "failed" }.into(),
                review,
                usage,
            },
            now(),
        )
    })
}

pub fn recover_agent(store: &mut Store, id: u64, terminate: bool) -> Result<Value> {
    let s = store.read()?;
    let launch = s.launches.iter().find(|l| l.id == id).context("launch")?;
    ensure!(launch.ended.is_none(), "launch already terminal");
    let Some(process) = launch.process.as_ref() else {
        ensure!(
            launch.managed,
            "native bridge launch: verify its actual session terminal state before agent-finish"
        );
        return recover_startup(store, ProcessOwner::Launch(id));
    };
    if group_alive(process.group)? {
        ensure!(
            terminate,
            "owned process group remains active; monitor it or request termination"
        );
        stop_group(process)?;
        for _ in 0..50 {
            if !group_alive(process.group)? {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        ensure!(
            !group_alive(process.group)?,
            "owned group has not settled; keep the claim"
        );
    }
    store.change("agent-process-recovered",|s|{s.launches.iter_mut().find(|l|l.id==id).unwrap().process.as_mut().unwrap().terminal=true;Ok(json!({"launch":id,"process_terminal":true,"next":"agent-finish with the actual session and failed/cancelled outcome"}))})
}

pub fn resolve_gh_failure(store: &mut Store, id: u64, evidence: &str) -> Result<Value> {
    required_text(
        evidence,
        "proof that the prior caller terminated and the operation did not execute",
    )?;
    let s = store.read()?;
    let op = s
        .operations
        .iter()
        .find(|o| o.id == id)
        .context("operation")?
        .clone();
    ensure!(
        op.kind == "github" && op.status == OpStatus::Unknown,
        "only uncertain GitHub operations need reconciliation"
    );
    if let Some(process) = &op.process {
        ensure!(
            process.terminal && !group_alive(process.group)?,
            "prior GitHub process has not terminated; recover it first"
        );
    }
    crate::followup::current_operation(&store.read()?, &op)?;
    let observed = observe_pr(store, &op.key)?;
    if store
        .read()?
        .operations
        .iter()
        .find(|o| o.id == id)
        .unwrap()
        .status
        == OpStatus::Confirmed
    {
        return Ok(json!({"confirmed":true}));
    }
    let no_effect = match op.details["action"].as_str() {
        Some("create-pr") => observed["found"] == false,
        Some("ready") => observed["pr"]["isDraft"] == true,
        Some("merge") => {
            // Do not classify a queued or auto-merge request as failed merely because it is not merged yet.
            let pr = op.details["pr"].as_u64().context("PR")?;
            let extra = gh_json(
                &store.root,
                &s.github_repo,
                &args(&[
                    "pr",
                    "view",
                    &pr.to_string(),
                    "--json",
                    "autoMergeRequest,mergeStateStatus",
                ]),
            )?;
            observed["pr"]["state"] == "OPEN"
                && extra["autoMergeRequest"].is_null()
                && extra["mergeStateStatus"] != "QUEUED"
        }
        _ => false,
    };
    ensure!(no_effect, "remote outcome is not established; keep unknown");
    store.change("github-failed-after-reconciliation",|s|{let op=s.operations.iter_mut().find(|o|o.id==id).unwrap();ensure!(op.status==OpStatus::Unknown,"operation changed");op.status=OpStatus::Failed;op.observation=Some(json!({"evidence":evidence,"remote":observed}));Ok(json!({"failed":true,"next":"diagnose, then prepare a new operation only if still authorized"}))})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckRequest {
    pub key: String,
    pub criteria: Vec<String>,
    pub argv: Vec<String>,
    pub cwd: std::path::PathBuf,
    pub timeout_seconds: u64,
    pub implementation: Vec<String>,
}
pub fn run_check(store: &mut Store, r: CheckRequest) -> Result<Value> {
    ensure!(
        !r.argv.is_empty() && !r.argv[0].is_empty(),
        "check argv required"
    );
    ensure!(
        (1..=14400).contains(&r.timeout_seconds),
        "check timeout must be 1..14400 seconds"
    );
    ensure!(
        !r.criteria.is_empty() && !r.implementation.is_empty(),
        "criterion and implementation references required"
    );
    let s = store.read()?;
    engine::idle(&s)?;
    ensure!(
        s.active.as_ref() == Some(&r.key),
        "check must belong to active task"
    );
    let t = &s.tasks[&r.key];
    let snapshot = fresh(&s, t)?;
    let slot = worktree(&s, t)?;
    let cwd = r.cwd.canonicalize()?;
    ensure!(
        cwd.starts_with(slot),
        "check directory must be inside the owned worker"
    );
    ensure!(
        r.criteria.iter().all(|id| crate::stages::criteria(t)
            .iter()
            .any(|c| &c.id == id && !c.human_only)),
        "checks cannot supply human acceptance or unknown criteria"
    );
    let op = store.change("check-begin", |s| {
        engine::idle(s)?;
        ensure!(
            s.tasks[&r.key].snapshot.as_ref() == Some(&snapshot),
            "snapshot changed before check"
        );
        let op = Operation {
            id: s.id(),
            key: r.key.clone(),
            kind: "check".into(),
            status: OpStatus::Unknown,
            created: now(),
            snapshot: Some(snapshot.clone()),
            attempts: 1,
            details: json!({"argv":r.argv,"cwd":cwd,"criteria":r.criteria}),
            observation: None,
            process: None,
        };
        s.operations.push(op.clone());
        Ok(json!(op))
    })?;
    let id = op["id"].as_u64().unwrap();
    let mut cmd = Command::new(&r.argv[0]);
    cmd.args(&r.argv[1..]).current_dir(&cwd);
    let root = store.root.clone();
    let started = std::time::Instant::now();
    let result = capture_owned(
        &mut cmd,
        &root,
        &format!("check-{id}"),
        r.timeout_seconds,
        Some((store, ProcessOwner::Operation(id))),
    );
    let (passed, stdout) =
        result.context("check outcome uncertain; recover-operation before repeating")?;
    let path = root.join(format!("check-{id}.json"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(
            &json!({"argv":r.argv,"cwd":cwd,"snapshot":snapshot,"passed":passed,"stdout":stdout,"operation":id,"elapsed_ms":started.elapsed().as_millis()}),
        )?,
    )?;
    store.change("check-complete",|s|{
        let t=&s.tasks[&r.key];let unchanged=fresh(s,t).is_ok_and(|current|current==snapshot);
        let op=s.operations.iter_mut().find(|o|o.id==id).unwrap();op.status=if passed&&unchanged{OpStatus::Confirmed}else{OpStatus::Failed};op.observation=Some(json!({"passed":passed,"snapshot_unchanged":unchanged,"artifact":path}));
        if unchanged {
            for criterion in &r.criteria {
                engine::apply(s,Input::Evidence{key:r.key.parse()?,evidence:EvidenceInput{criterion:criterion.clone(),status:if passed{EvidenceStatus::Passed}else{EvidenceStatus::Failed},kind:EvidenceKind::Command,artifact:path.clone(),implementation:r.implementation.clone(),description:format!("Executed {:?} in {}",r.argv,cwd.display()),human_source:None}},now())?;
            }
        }
        Ok(json!({"passed":passed,"snapshot_unchanged":unchanged,"artifact":path,"next":if unchanged{"inspect evidence"}else{"preserve changed work, record snapshot and rerun relevant checks"}}))
    })
}
pub fn recover_operation(store: &mut Store, id: u64, terminate: bool) -> Result<Value> {
    let s = store.read()?;
    let op = s
        .operations
        .iter()
        .find(|o| o.id == id)
        .context("operation")?;
    ensure!(op.status == OpStatus::Unknown, "operation is not uncertain");
    let Some(mut process) = op.process.clone() else {
        return recover_startup(store, ProcessOwner::Operation(id));
    };
    if group_alive(process.group)? {
        ensure!(terminate, "owned operation remains active");
        stop_group(&process)?;
        for _ in 0..50 {
            if !group_alive(process.group)? {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        ensure!(
            !group_alive(process.group)?,
            "owned process group has not settled"
        );
    }
    process.terminal = true;
    store.change("operation-process-recovered",|s|{let op=s.operations.iter_mut().find(|o|o.id==id).unwrap();op.process=Some(process);if op.kind=="check"{op.status=OpStatus::Failed;op.observation=Some(json!({"result":"interrupted check; no passing evidence recorded"}));}Ok(json!({"process_terminal":true,"operation":id,"next":"reconcile GitHub/Jira outcome before retrying any external mutation"}))})
}

pub fn provision_slot(store: &mut Store, slot: &str, branch: &str) -> Result<Value> {
    let s = store.read()?;
    engine::idle(&s)?;
    let key = s.active.as_ref().context("claim a task first")?.clone();
    slot.parse::<crate::ids::SlotId>()?;
    ensure!(
        s.slots.get(slot) == Some(&s.run_id),
        "reserve this numbered slot first"
    );
    ensure!(
        branch.starts_with("agent-")
            && branch.contains(&format!("/{slot}/"))
            && branch.contains(&key),
        "branch must identify agent, slot and task"
    );
    ensure!(
        engine::unfinished_slot_owner(&s, &key, slot)?.is_none(),
        "slot contains unfinished work"
    );
    ensure!(
        !s.repo.join(".worktrees").join(slot).exists(),
        "slot already exists; reconcile its existing checkout before binding, never overwrite it"
    );
    crate::followup::binding(&s, &s.tasks[&key], slot)?;
    if let Some(f) = &s.tasks[&key].followup {
        ensure!(
            sha(&s.repo, "origin/main")? == f.base,
            "prepared origin/main base changed"
        );
    }
    let op = store.change("provision-begin", |s| {
        engine::idle(s)?;
        let id = s.id();
        s.operations.push(Operation {
            id,
            key: key.clone(),
            kind: "provision".into(),
            status: OpStatus::Unknown,
            created: now(),
            snapshot: None,
            attempts: 1,
            details: json!({"slot":slot,"branch":branch}),
            observation: None,
            process: None,
        });
        Ok(json!({"id":id}))
    })?;
    let id = op["id"].as_u64().unwrap();
    let root = store.root.clone();
    let mut cmd = Command::new("bash");
    cmd.current_dir(&s.repo)
        .arg(s.repo.join("scripts/worktree-add.sh"))
        .arg(slot);
    let (ok, _) = capture_owned(
        &mut cmd,
        &root,
        &format!("provision-{id}"),
        14400,
        Some((store, ProcessOwner::Operation(id))),
    )?;
    if ok {
        git(
            &s.repo.join(".worktrees").join(slot),
            &["switch", "-c", branch, "origin/main"],
        )?;
    }
    store.change("provision-finish",|s|{
        let op=s.operations.iter_mut().find(|o|o.id==id).unwrap();op.status=if ok{OpStatus::Confirmed}else{OpStatus::Failed};
        op.observation=Some(json!({"helper_success":ok}));
        if ok{engine::apply(s,Input::Bind{key:key.parse()?,slot:slot.parse()?,historical_slot_reuse:None},now())}else{Ok(json!({"provisioned":false,"next":"preserve and inspect partial slot; do not reset unknown work"}))}
    })
}
pub fn cleanup(store: &mut Store, key: &str, delete: bool) -> Result<Value> {
    let checked = store.change("cleanup-check", |s| {
        engine::apply(s, Input::CleanupCheck { key: key.parse()? }, now())
    })?;
    let slot = checked["slot"].as_str().context("slot")?.to_owned();
    let s = store.read()?;
    let id = store.change("cleanup-begin", |s| {
        engine::apply(s, Input::CleanupCheck { key: key.parse()? }, now())?;
        let id = s.id();
        s.operations.push(Operation {
            id,
            key: key.into(),
            kind: "cleanup".into(),
            status: OpStatus::Unknown,
            created: now(),
            snapshot: s.tasks[key].snapshot.clone(),
            attempts: 1,
            details: json!({"slot":slot,"delete":delete}),
            observation: None,
            process: None,
        });
        Ok(json!({"id":id}))
    })?["id"]
        .as_u64()
        .unwrap();
    let script = s.repo.join(if delete {
        "scripts/worktree-delete.sh"
    } else {
        "scripts/worktree-reset.sh"
    });
    let root = store.root.clone();
    if delete {
        let mut preview = Command::new("bash");
        preview
            .current_dir(&s.repo)
            .arg(&script)
            .arg(&slot)
            .arg("--what-if");
        let (ok, text) = capture(&mut preview, &root, &format!("cleanup-preview-{id}"), 120)?;
        ensure!(
            ok && text.contains(&slot),
            "cleanup preview did not establish the exact selected slot"
        );
    }
    let mut cmd = Command::new("bash");
    cmd.current_dir(&s.repo).arg(&script).arg(&slot);
    let (ok, _) = capture_owned(
        &mut cmd,
        &root,
        &format!("cleanup-{id}"),
        14400,
        Some((store, ProcessOwner::Operation(id))),
    )?;
    if ok {
        if delete {
            ensure!(
                !s.repo.join(".worktrees").join(&slot).exists(),
                "cleanup did not remove the selected worktree"
            );
        } else {
            let path = s.repo.join(".worktrees").join(&slot);
            clean(&path)?;
            ensure!(
                sha(&path, "HEAD")? == sha(&path, "origin/main")?,
                "reset checkout is not aligned with origin/main"
            );
        }
    }
    store.change("cleanup-finish", |s| {
        let op = s.operations.iter_mut().find(|o| o.id == id).unwrap();
        op.status = if ok {
            OpStatus::Confirmed
        } else {
            OpStatus::Failed
        };
        op.observation = Some(json!({"helper_success":ok,"delete":delete,"slot":slot}));
        s.tasks.get_mut(key).unwrap().next_action = if ok {
            "Repository cleanup verified"
        } else {
            "Cleanup helper failed; preserve slot and inspect logs"
        }
        .into();
        if ok && delete {
            let reservation = s.repo.join(".local/epic-control/slots").join(&slot);
            ensure!(
                fs::read_to_string(&reservation)? == s.run_id,
                "reservation ownership changed"
            );
            fs::remove_file(reservation)?;
            s.slots.remove(&slot);
        }
        Ok(json!({"cleanup_verified":ok,"slot":slot,"deleted":ok&&delete}))
    })
}

pub fn owned_child(arguments: Vec<std::ffi::OsString>) -> Result<()> {
    ensure!(arguments.len() >= 3, "invalid owned child invocation");
    let gate = Path::new(&arguments[0]);
    let marker = arguments[1].to_str().context("marker")?;
    let start = Instant::now();
    while !gate.exists() {
        ensure!(
            start.elapsed() < Duration::from_secs(15),
            "supervisor did not release startup gate; no command executed"
        );
        thread::sleep(Duration::from_millis(10));
    }
    ensure!(
        fs::read_to_string(gate)? == marker,
        "startup gate identity mismatch"
    );
    let mut cmd = Command::new(&arguments[2]);
    cmd.args(&arguments[3..]);
    if let Some(path) = std::env::var_os("EPIC_CONTROL_INPUT") {
        cmd.stdin(Stdio::from(fs::File::open(path)?));
        cmd.env_remove("EPIC_CONTROL_INPUT");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(cmd.exec().into())
    }
    #[cfg(not(unix))]
    {
        bail!("owned child execution requires Unix")
    }
}

// A missing database receipt cannot mean an untracked command was released:
// capture_owned persists that receipt before publishing the command's startup gate.
fn recover_startup(store: &mut Store, owner: ProcessOwner) -> Result<Value> {
    let s = store.read()?;
    let prefix = match owner {
        ProcessOwner::Launch(id) => format!("agent-{id}-"),
        ProcessOwner::Operation(id) => {
            let op = s
                .operations
                .iter()
                .find(|o| o.id == id)
                .context("operation")?;
            format!(
                "{}-{id}-",
                match op.kind.as_str() {
                    "check" => "check",
                    "github" => "github-operation",
                    "provision" => "provision",
                    "cleanup" => "cleanup",
                    _ => bail!("native operation has no supervised startup"),
                }
            )
        }
    };
    let mut receipt = None;
    for entry in fs::read_dir(&store.root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) {
            continue;
        }
        ensure!(
            !name.ends_with(".gate"),
            "released gate without database process receipt; preserve state for manual reconciliation"
        );
        if name.ends_with(".process.json") {
            ensure!(receipt.is_none(), "multiple startup receipts");
            receipt = Some(serde_json::from_slice::<ProcessIdentity>(&fs::read(
                entry.path(),
            )?)?);
        }
    }
    if let Some(process) = &receipt
        && group_alive(process.group)?
    {
        stop_group(process)?;
    }
    store.change("startup-not-released", |s| {
        match owner {
            ProcessOwner::Launch(id) => {
                let l = s.launches.iter_mut().find(|l| l.id == id).unwrap();
                ensure!(l.process.is_none() && l.ended.is_none(), "launch changed");
                l.ended = Some(now());
                l.outcome = Some("failed-before-start".into());
            }
            ProcessOwner::Operation(id) => {
                let op = s.operations.iter_mut().find(|o| o.id == id).unwrap();
                ensure!(
                    op.process.is_none() && op.status == OpStatus::Unknown,
                    "operation changed"
                );
                op.status = OpStatus::Failed;
                op.observation = Some(json!({"startup_gate_never_released":true}));
            }
        }
        Ok(json!({"failed_before_start":true,"command_executed":false}))
    })
}

pub fn reconcile_helper(store: &mut Store, id: u64) -> Result<Value> {
    let (s, op) = current_delivery_operation(store, id)?;
    ensure!(
        matches!(op.kind.as_str(), "provision" | "cleanup") && op.status == OpStatus::Unknown,
        "not an uncertain helper operation"
    );
    if op.process.is_none() {
        return recover_startup(store, ProcessOwner::Operation(id));
    }
    let process = op.process.as_ref().unwrap();
    ensure!(
        process.terminal && !group_alive(process.group)?,
        "recover the helper process before checking its postconditions"
    );
    let slot = op.details["slot"].as_str().context("slot")?;
    slot.parse::<crate::ids::SlotId>()?;
    let path = s.repo.join(".worktrees").join(slot);
    let (complete, checkout) = helper_postconditions(&s, &op, process, &path)?;
    store.change("helper-reconciled", |state| {
        record_helper_reconciliation(state, id, slot, &path, complete, checkout)
    })
}

fn helper_postconditions(
    state: &State,
    operation: &Operation,
    process: &ProcessIdentity,
    path: &Path,
) -> Result<(bool, Option<Value>)> {
    if operation.kind == "provision" {
        return provision_postconditions(operation, path);
    }
    if operation.details["delete"] == true {
        let registered = git(&state.repo, &["worktree", "list", "--porcelain"])?
            .lines()
            .any(|line| line == format!("worktree {}", path.display()));
        return Ok((!path.exists() && !registered, None));
    }
    let complete = if path.is_dir() && process.exit_success == Some(true) {
        git(path, &["status", "--porcelain"])?.is_empty()
            && sha(path, "HEAD")? == sha(path, "origin/main")?
    } else {
        false
    };
    Ok((complete, None))
}

fn provision_postconditions(operation: &Operation, path: &Path) -> Result<(bool, Option<Value>)> {
    if !path.is_dir() {
        return Ok((false, None));
    }
    let clean = git(path, &["status", "--porcelain"])?.is_empty();
    let branch = git(path, &["branch", "--show-current"])?;
    let head = sha(path, "HEAD")?;
    let origin_main = sha(path, "origin/main")?;
    let expected_branch = operation.details["branch"].as_str().context("branch")?;
    let complete = clean && branch == expected_branch && head == origin_main;
    Ok((
        complete,
        Some(json!({
            "clean": clean,
            "branch": branch,
            "head": head,
            "origin_main": origin_main,
        })),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProvisionBinding {
    NeedsBind,
    AlreadyBound,
    Conflict(String),
}

fn provision_binding_state(
    state: &State,
    operation: &Operation,
    slot: &str,
) -> Result<ProvisionBinding> {
    ensure!(
        state.active.as_deref() == Some(operation.key.as_str()),
        "operation task is not the active task"
    );
    ensure!(
        !state.launches.iter().any(|launch| launch.ended.is_none())
            && !state.operations.iter().any(|candidate| {
                candidate.id != operation.id
                    && matches!(candidate.status, OpStatus::Prepared | OpStatus::Unknown)
            }),
        "other work is pending or uncertain"
    );
    let task = state.tasks.get(&operation.key).context("operation task")?;
    crate::followup::binding(state, task, slot)?;
    let binding = match task.slot.as_deref() {
        Some(bound) => {
            ensure!(
                bound == slot,
                "provisioned slot differs from the task's existing binding"
            );
            ensure!(
                engine::unfinished_slot_owner(state, &operation.key, slot)?.is_none(),
                "slot contains unfinished work"
            );
            ProvisionBinding::AlreadyBound
        }
        None => match engine::unfinished_slot_owner(state, &operation.key, slot)? {
            Some(owner) => ProvisionBinding::Conflict(owner.to_owned()),
            None => ProvisionBinding::NeedsBind,
        },
    };
    let mut candidate = task.clone();
    candidate.slot = Some(slot.to_owned());
    worktree(state, &candidate)?;
    Ok(binding)
}

fn current_provision_operation(state: &State, id: u64, key: &str, slot: &str) -> Result<Operation> {
    let operation = state
        .operations
        .iter()
        .find(|operation| operation.id == id)
        .context("operation")?;
    ensure!(
        operation.status == OpStatus::Unknown
            && operation.kind == "provision"
            && operation.key == key
            && operation.details["slot"] == slot,
        "provision operation changed during reconciliation"
    );
    Ok(operation.clone())
}

fn settle_provision_conflict(
    state: &mut State,
    id: u64,
    key: &str,
    owner: String,
    path: &Path,
    checkout: Option<Value>,
) -> Result<Value> {
    let operation = state
        .operations
        .iter_mut()
        .find(|operation| operation.id == id)
        .unwrap();
    operation.status = OpStatus::Failed;
    operation.observation = Some(json!({
        "postconditions_verified": true,
        "path": path,
        "checkout": checkout,
        "binding": "rejected",
        "binding_error": "slot contains unfinished work",
        "unfinished_owner": owner,
    }));
    state.tasks.get_mut(key).unwrap().next_action =
        "Preserve the conflicting checkout and reserve a different available slot".into();
    Ok(json!({
        "confirmed": false,
        "binding_rejected": true,
        "next": "preserve the checkout and reserve a different available slot",
    }))
}

fn complete_provision_binding(
    state: &mut State,
    binding: &ProvisionBinding,
    key: String,
    slot: &str,
) -> Result<()> {
    if binding == &ProvisionBinding::NeedsBind {
        engine::apply(
            state,
            Input::Bind {
                key: key.parse()?,
                slot: slot.parse()?,
                historical_slot_reuse: None,
            },
            now(),
        )?;
    }
    Ok(())
}

fn record_helper_reconciliation(
    state: &mut State,
    id: u64,
    slot: &str,
    path: &Path,
    complete: bool,
    checkout: Option<Value>,
) -> Result<Value> {
    let operation = state
        .operations
        .iter()
        .find(|operation| operation.id == id)
        .context("operation")?;
    let key = operation.key.clone();
    let provision = operation.kind == "provision";
    let delete = operation.kind == "cleanup" && operation.details["delete"] == true;
    let binding = if complete && provision {
        let current = current_provision_operation(state, id, &key, slot)?;
        Some(provision_binding_state(state, &current, slot)?)
    } else {
        None
    };
    if let Some(ProvisionBinding::Conflict(owner)) = &binding {
        return settle_provision_conflict(state, id, &key, owner.clone(), path, checkout);
    }
    let operation = state
        .operations
        .iter_mut()
        .find(|operation| operation.id == id)
        .unwrap();
    operation.status = if complete {
        OpStatus::Confirmed
    } else {
        OpStatus::Failed
    };
    operation.observation = Some(json!({
        "postconditions_verified": complete,
        "path": path,
        "checkout": checkout,
    }));
    if complete && delete {
        remove_slot_reservation(state, slot)?;
    }
    state.tasks.get_mut(&key).unwrap().next_action = if complete {
        "Helper postconditions verified"
    } else {
        "Helper incomplete; preserve partial work and reconcile before any retry"
    }
    .into();
    if let Some(binding) = binding {
        complete_provision_binding(state, &binding, key, slot)?;
    }
    Ok(json!({
        "confirmed": complete,
        "binding_rejected": false,
        "next": if complete {
            "resume workflow"
        } else {
            "inspect partial slot; no mutation was repeated"
        },
    }))
}

fn remove_slot_reservation(state: &mut State, slot: &str) -> Result<()> {
    let reservation = state.repo.join(".local/epic-control/slots").join(slot);
    if reservation.exists() {
        ensure!(
            fs::read_to_string(&reservation)? == state.run_id,
            "reservation ownership changed"
        );
        fs::remove_file(reservation)?;
    }
    state.slots.remove(slot);
    Ok(())
}

fn owned_wrapper(command: &Command, gate: &Path, marker: &str) -> Result<Command> {
    let mut wrapper = Command::new(std::env::current_exe()?);
    wrapper
        .arg("__owned-child")
        .arg(gate)
        .arg(marker)
        .arg(command.get_program())
        .args(command.get_args());
    if let Some(cwd) = command.get_current_dir() {
        wrapper.current_dir(cwd);
    }
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            wrapper.env(key, value);
        } else {
            wrapper.env_remove(key);
        }
    }
    wrapper
        .env("EPIC_CONTROL_OWNER", marker)
        .stdin(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        wrapper.process_group(0);
    }

    Ok(wrapper)
}
fn wait_owned(
    child: &mut std::process::Child,
    process: &ProcessIdentity,
    out_path: &Path,
    root: &Path,
    label: &str,
    seconds: u64,
    mut store: Option<(&mut Store, ProcessOwner)>,
) -> Result<(bool, String)> {
    let start = Instant::now();
    let mut progress = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if let Some((db, owner)) = &mut store {
                ensure!(
                    !group_alive(process.group)?,
                    "agent exited but owned descendants remain; claim is retained until recover-agent verifies termination"
                );
                let mut terminal = process.clone();
                terminal.terminal = true;
                terminal.exit_success = Some(status.success());
                db.change("process-terminal", |s| {
                    set_process(s, *owner, terminal.clone())?;
                    Ok(json!({"process_terminal":true,"pid":terminal.pid}))
                })?;
            }
            return Ok((status.success(), fs::read_to_string(out_path)?));
        }
        if start.elapsed() >= Duration::from_secs(seconds) {
            stop_group(process)?;
            child.wait()?;
            bail!(
                "owned command timed out; reconcile external state before retrying; logs: {}",
                root.display()
            );
        }
        if progress.elapsed() >= Duration::from_secs(30) {
            eprintln!("{label} remains active; logs in {}", root.display());
            progress = Instant::now();
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn require_saved_authentication() -> Result<()> {
    let auth = Command::new("codex")
        .args(["login", "status"])
        .env_remove("CODEX_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .output()
        .context("check saved Codex authentication")?;
    let auth_text = format!(
        "{}{}",
        String::from_utf8_lossy(&auth.stdout),
        String::from_utf8_lossy(&auth.stderr)
    );
    ensure!(
        auth.status.success() && auth_text.contains("Logged in using ChatGPT"),
        "saved ChatGPT authentication is required; no API billing fallback is permitted"
    );

    Ok(())
}
fn agent_events(events: &str, previous: Option<&str>) -> Result<(String, Option<Value>)> {
    let mut session = previous.map(str::to_owned);
    let mut usage = None;
    for line in events.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if v["type"] == "thread.started" {
                session = v["thread_id"].as_str().map(str::to_owned);
            }
            if v["type"] == "turn.completed" {
                usage = v.get("usage").cloned();
            }
        }
    }
    let session =
        session.context("no session id received; launch remains claimed for reconciliation")?;

    Ok((session, usage))
}

fn create_pr_command(op: &Operation, task: &Task) -> Result<Vec<String>> {
    let body = Path::new(op.details["body"].as_str().context("body path")?);
    ensure!(
        Some(artifact(body)?.as_ref()) == op.details["body_digest"].as_str(),
        "PR body changed after preparation"
    );
    Ok(args(&[
        "pr",
        "create",
        "--draft",
        "--base",
        "main",
        "--head",
        task.branch.as_ref().unwrap(),
        "--title",
        op.details["title"].as_str().unwrap(),
        "--body-file",
        body.to_str().context("UTF-8 body path")?,
    ]))
}

fn validate_gh_snapshot(s: &State, t: &Task, op: &Operation) -> Result<Snapshot> {
    let snap = fresh(s, t)?;
    ensure!(
        op.snapshot.as_ref() == Some(&snap),
        "operation snapshot became stale"
    );

    Ok(snap)
}

fn codex_command(
    r: &AgentRequest,
    cwd: &Path,
    launch: &Value,
    schema: &Path,
    output: &Path,
) -> Command {
    let mut cmd = Command::new("codex");
    cmd.current_dir(cwd).arg("exec");
    if let Some(session) = launch["session"].as_str() {
        cmd.args(["resume", session]);
    }
    cmd.args([
        "--json",
        "--model",
        &r.model,
        "-c",
        &format!("model_reasoning_effort=\"{}\"", r.effort),
        "-c",
        "features.multi_agent=false",
        "-c",
        "service_tier=\"default\"",
        "-c",
        if r.role == Role::Implementer {
            "sandbox_mode=\"workspace-write\""
        } else {
            "sandbox_mode=\"read-only\""
        },
    ]);
    cmd.env_remove("CODEX_API_KEY").env_remove("OPENAI_API_KEY");
    if r.role == Role::Reviewer {
        cmd.arg("--output-schema").arg(schema);
    }
    cmd.arg("--output-last-message")
        .arg(output)
        .arg("-")
        .stdin(Stdio::piped());

    cmd
}

fn merge_pr_command(store: &mut Store, op: &Operation, observed: &Value) -> Result<Vec<String>> {
    let s = store.read()?;
    let t = s.tasks.get(&op.key).context("task")?;
    let snap = validate_gh_snapshot(&s, t, op)?;

    engine::reviewed(&s, t)?;
    crate::loops::human_review(&s, t)?;
    ensure!(
        observed["pr"]["state"] == "OPEN" && !t.draft,
        "PR is not ready"
    );
    poll_checks(store, &op.key)?;
    let latest = store.read()?;
    let t = &latest.tasks[&op.key];
    ensure!(
        !t.checks.is_empty()
            && t.checks
                .iter()
                .all(|c| matches!(c["bucket"].as_str(), Some("pass" | "skipping"))),
        "required checks not green"
    );
    Ok(args(&[
        "pr",
        "merge",
        &t.pr.unwrap().to_string(),
        &format!("--{}", op.details["method"].as_str().unwrap()),
        "--match-head-commit",
        &snap.head,
    ]))
}

/// Explicit historical observation never changes current delivery or operations.
pub fn observe_selected_pr(store: &mut Store, key: &str, number: Option<u64>) -> Result<Value> {
    let s = store.read()?;
    let t = s.tasks.get(key).context("task")?;
    let Some(number) = number else {
        return observe_pr(store, key);
    };
    if t.pr == Some(number) {
        return observe_pr(store, key);
    }
    if let Some(external) = t
        .delivery_history
        .iter()
        .filter_map(|d| d.superseded_by.as_ref())
        .find(|r| r.pr == number)
    {
        let pr = read_pr_number(store, &s.github_repo, number)?;
        git(
            &s.repo,
            &[
                "fetch",
                "origin",
                "refs/heads/main:refs/remotes/origin/main",
            ],
        )?;
        crate::superseded::replacement(&s, external, &pr)?;
        return Ok(
            json!({"found":true,"historical":true,"external_replacement":true,"pr":pr,"current_delivery":crate::followup::identity(t)}),
        );
    }
    let archived = t
        .delivery_history
        .iter()
        .find(|d| d.task.pr == Some(number))
        .context("PR is neither current nor historical for this task")?;
    let mut historical = s.clone();
    historical.tasks.insert(key.into(), *archived.task.clone());
    let pr = read_pr_number(store, &s.github_repo, number)?;
    if archived.superseded_by.is_some() {
        crate::superseded::closed(&historical, key, &pr)?;
    } else {
        verify_merged_observation(&historical, key, &pr)?;
    }
    Ok(
        json!({"found":true,"historical":true,"pr":pr,"current_delivery":crate::followup::identity(t)}),
    )
}
pub(crate) fn read_pr_number(store: &Store, repo: &str, number: u64) -> Result<Value> {
    gh_json(
        &store.root,
        repo,
        &args(&[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "number,state,isDraft,headRefOid,headRefName,baseRefName,mergeCommit,url",
        ]),
    )
}
pub(crate) fn verify_merged_observation(s: &State, key: &str, pr: &Value) -> Result<()> {
    verify_pr(s, key, pr)?;
    ensure!(pr["state"] == "MERGED", "historical PR must remain merged");
    git(
        &s.repo,
        &[
            "fetch",
            "origin",
            "refs/heads/main:refs/remotes/origin/main",
        ],
    )?;
    on_main(
        &s.repo,
        pr["mergeCommit"]["oid"].as_str().context("merge commit")?,
    )
}
pub fn prepare_followup(store: &mut Store, request: crate::followup::Request) -> Result<Value> {
    let s = store.read()?;
    crate::followup::validate(&s, &request, now())?;
    let pr = read_pr_number(
        store,
        &s.github_repo,
        s.tasks[request.key.as_ref()].pr.context("historical PR")?,
    )?;
    verify_merged_observation(&s, &request.key, &pr)?;
    store.change("prepare-followup", |current| {
        verify_pr(current, &request.key, &pr)?;
        crate::followup::prepare(current, request, pr, now())
    })
}

fn current_delivery_operation(store: &Store, id: u64) -> Result<(State, Operation)> {
    let state = store.read()?;
    let operation = state
        .operations
        .iter()
        .find(|o| o.id == id)
        .context("operation")?
        .clone();
    crate::followup::current_operation(&state, &operation)?;
    Ok((state, operation))
}
