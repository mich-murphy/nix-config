mod agents;
mod delivery;
mod jira;
mod progress;
mod queue;
mod tasks;
use crate::loops;
use crate::{model::*, store::*};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
};

pub fn idle(s: &State) -> Result<()> {
    ensure!(
        !s.launches.iter().any(|l| l.ended.is_none()),
        "an agent turn is claimed or running; reconcile it first"
    );
    ensure!(
        !s.operations
            .iter()
            .any(|o| matches!(o.status, OpStatus::Prepared | OpStatus::Unknown)),
        "an external operation is pending or uncertain; reconcile it first"
    );
    Ok(())
}
pub(crate) fn unfinished_slot_owner<'a>(
    s: &'a State,
    key: &str,
    slot: &str,
) -> Result<Option<&'a str>> {
    let mut owners = s
        .tasks
        .values()
        .filter(|task| {
            task.spec.key != key
                && task.slot.as_deref() == Some(slot)
                && !(matches!(task.delivery, Delivery::Merged | Delivery::AlreadySatisfied)
                    && task.sync == Sync::Confirmed)
        })
        .map(|task| task.spec.key.as_str());
    let owner = owners.next();
    ensure!(
        owners.next().is_none(),
        "slot is assigned to multiple unfinished tasks"
    );
    Ok(owner)
}
fn task(s: &State, key: &str) -> Result<Task> {
    s.tasks.get(key).cloned().context("unknown task")
}
fn active(s: &State, key: &str) -> Result<Task> {
    ensure!(
        s.active.as_deref() == Some(key),
        "task is not the active task"
    );
    task(s, key)
}
fn save(s: &mut State, key: &str, t: Task) {
    s.tasks.insert(key.to_owned(), t);
}
fn reserve_issue(s: &State, key: &str) -> Result<()> {
    let directory = s.repo.join(".local/epic-control/issues");
    fs::create_dir_all(&directory)?;
    ensure!(
        directory.canonicalize()? == directory,
        "issue reservation path is redirected"
    );
    let path = directory.join(key);
    if path.exists() {
        ensure!(
            fs::read_to_string(path)? == s.run_id,
            "issue belongs to another run; resume that run or record an ownership conflict"
        );
    } else {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .context("issue claim collision")?;
        f.write_all(s.run_id.as_bytes())?;
        f.sync_all()?;
    }
    Ok(())
}
fn spec_valid(t: &TaskSpec) -> Result<()> {
    for sub in &t.subtasks {
        ensure!(sub != &t.key, "subtask cannot equal task");
    }
    required_text(&t.rank, "resolved Jira rank")?;
    if t.ownership_clear {
        required_text(&t.ownership_evidence, "ownership evidence")?;
    }
    for dep in &t.dependencies {
        if dep.verified {
            required_text(&dep.evidence, "dependency evidence")?;
        }
    }
    Ok(())
}
fn blocked(s: &State, t: &Task, at: u64) -> Result<()> {
    let reservation = s.repo.join(".local/epic-control/issues").join(&t.spec.key);
    if reservation.exists() {
        ensure!(
            fs::read_to_string(reservation)? == s.run_id,
            "task belongs to another run"
        );
    }
    fn cycle(
        s: &State,
        key: &str,
        path: &mut Vec<String>,
        done: &mut BTreeSet<String>,
    ) -> Option<Vec<String>> {
        if let Some(i) = path.iter().position(|k| k == key) {
            let mut result = path[i..].to_vec();
            result.push(key.into());
            return Some(result);
        }
        if done.contains(key) {
            return None;
        }
        path.push(key.into());
        if let Some(t) = s.tasks.get(key) {
            for dep in &t.spec.dependencies {
                if let Some(result) = cycle(s, &dep.key, path, done) {
                    return Some(result);
                }
            }
        }
        path.pop();
        done.insert(key.into());
        None
    }
    if let Some(path) = cycle(s, &t.spec.key, &mut vec![], &mut BTreeSet::new()) {
        bail!("dependency cycle: {}", path.join(" -> "));
    }
    ensure!(t.spec.member, "task was removed from epic");
    ensure!(t.spec.ownership_clear, "ownership is unresolved");
    ensure!(
        !t.was_terminal,
        "pre-existing terminal task is preserved; record contradictions without reopening"
    );
    ensure!(t.spec.blocker.is_none(), "task has a recorded blocker");
    ensure!(
        t.spec.not_before.is_none_or(|n| n <= at),
        "not-before date is in the future"
    );
    for dep in &t.spec.dependencies {
        ensure!(dep.verified, "unverified prerequisite {}", dep.key);
        if let Some(parent) = s.tasks.get(&dep.key) {
            ensure!(
                parent.main_verified,
                "prerequisite {} is not verified on main",
                dep.key
            );
        }
        if dep.code {
            on_main(
                &s.repo,
                dep.main_commit
                    .as_ref()
                    .context("code prerequisite needs a commit")?,
            )?;
        }
    }
    Ok(())
}
pub fn candidates(s: &State, at: u64) -> Vec<String> {
    let mut items: Vec<_> = s
        .tasks
        .values()
        .filter(|t| t.delivery == Delivery::Queued && blocked(s, t, at).is_ok())
        .collect();
    items.sort_by_key(|t| {
        let planning = s
            .order
            .iter()
            .position(|key| key == &t.spec.key)
            .unwrap_or(usize::MAX);
        (
            planning,
            t.spec.priority,
            t.spec.due.unwrap_or(u64::MAX),
            t.spec.rank.clone(),
            t.spec.key.clone(),
        )
    });
    items.iter().map(|t| t.spec.key.clone()).collect()
}
fn refreshed(s: &State, at: u64) -> Result<()> {
    ensure!(
        s.refresh_generation > s.selection_generation && at.saturating_sub(s.refreshed) <= 300,
        "refresh the frozen queue before selecting a task"
    );
    Ok(())
}
pub fn acceptance(s: &State, t: &Task) -> Result<()> {
    crate::followup::acceptance(t)?;
    crate::stages::validate(t)?;
    loops::acceptance(s, t)?;
    let snap = fresh(s, t)?;
    crate::superseded::scope(s, t, &snap)?;
    ensure!(!t.criteria.is_empty(), "no acceptance criteria");
    for c in crate::stages::criteria(t) {
        let e = t
            .evidence
            .get(&c.id)
            .with_context(|| format!("missing evidence for {}", c.id))?;
        ensure!(
            e.snapshot == snap && e.input.status == EvidenceStatus::Passed,
            "criterion {} is not passed for this snapshot",
            c.id
        );
        ensure!(
            artifact(&e.input.artifact)? == e.digest,
            "evidence artifact changed"
        );
        if c.human_only {
            ensure!(
                e.input.kind == EvidenceKind::Human
                    && e.input
                        .human_source
                        .as_ref()
                        .is_some_and(|v| !v.trim().is_empty()),
                "human acceptance is missing"
            );
        }
    }
    Ok(())
}
pub fn reviewed(s: &State, t: &Task) -> Result<()> {
    acceptance(s, t)?;
    let review = t
        .review
        .as_ref()
        .context("independent review has not passed")?;
    ensure!(
        review.output.verdict == Verdict::Pass
            && Some(&review.output.snapshot) == t.snapshot.as_ref(),
        "review does not cover this snapshot"
    );
    ensure!(
        review.output.evidence_gaps.is_empty(),
        "review has evidence gaps"
    );
    for id in t.findings.keys() {
        ensure!(
            t.dispositions.contains_key(id),
            "finding {id} is not dispositioned"
        );
    }
    Ok(())
}
pub fn next(s: &State, at: u64) -> Value {
    if let Some(o) = s
        .operations
        .iter()
        .find(|o| matches!(o.status, OpStatus::Prepared | OpStatus::Unknown))
    {
        return json!({"action":if o.status==OpStatus::Prepared {"dispatch-operation"}else{"reconcile-operation"},"operation":o});
    }
    if let Some(l) = s.launches.iter().find(|l| l.ended.is_none()) {
        return json!({"action":"monitor-agent","launch":l});
    }
    if s.loop_policy.is_none() {
        return json!({"action":"configure-loop","reason":"Existing state and budgets are preserved; configure progress policy before continuing."});
    }
    if let Some(k) = &s.active {
        let t = &s.tasks[k];
        let action = task_action(s, t);
        return json!({"action":action,"key":k,"next_action":t.next_action});
    }
    if refreshed(s, at).is_err() {
        return json!({"action":"refresh-queue"});
    }
    if loops::capacity(s).is_err() {
        return json!({"action":"recover-unfinished-prs","keys":loops::unfinished(s),"reason":"Unfinished PR limit reached; observe existing PRs, resume eligible work, or report the hold."});
    }
    if let Some(k) = candidates(s, at).first() {
        return json!({"action":"claim","key":k});
    }
    json!({"action":"revisit-or-report","remaining":s.tasks.values().filter(|t|!matches!(t.delivery,Delivery::Merged|Delivery::AlreadySatisfied|Delivery::Excluded)).map(|t|json!({"key":t.spec.key,"state":t.delivery,"reason":blocked(s,t,at).err().map(|e|e.to_string()),"next":t.next_action})).collect::<Vec<_>>()})
}

pub fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    crate::followup::guard_input(s, &input)?;
    match input {
        input @ (Input::ConfigureLoop { .. }
        | Input::LessonRecord { .. }
        | Input::LoopPlan { .. }
        | Input::Measure { .. }
        | Input::Checkpoint { .. }
        | Input::HumanReview { .. }) => progress::apply(s, input, at),
        input @ (Input::Discover { .. }
        | Input::Refresh { .. }
        | Input::Claim { .. }
        | Input::Hold { .. }
        | Input::Resume { .. }) => queue::apply(s, input, at),
        input @ (Input::Brief { .. }
        | Input::ReserveSlot { .. }
        | Input::Bind { .. }
        | Input::Snapshot { .. }
        | Input::Evidence { .. }
        | Input::SubtaskRecord { .. }) => tasks::apply(s, input, at),
        input @ (Input::AgentBegin { .. }
        | Input::AgentFinish { .. }
        | Input::Disposition { .. }) => agents::apply(s, input, at),
        input @ (Input::JiraPrepare { .. }
        | Input::Dispatch { .. }
        | Input::JiraObserve { .. }
        | Input::JiraRetry { .. }
        | Input::SyncFailed { .. }) => jira::apply(s, input, at),
        input @ (Input::GhPrepare { .. }
        | Input::FinalVerify { .. }
        | Input::Complete { .. }
        | Input::CleanupCheck { .. }) => delivery::apply(s, input, at),
        input => apply_support(s, input, at),
    }
}
fn apply_support(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::AuthorizeBudgetAdjustment { .. } => crate::budget::apply(s, input, at),
        Input::EvidenceBatch {
            key,
            snapshot,
            results,
        } => crate::batch::record(s, key, snapshot, results, at),
        Input::UsageImport { request } => crate::telemetry::import(s, request),
        Input::UsageReport => Ok(crate::telemetry::report(s)),
        Input::UsageBind { launch, source } => crate::usage_capture::bind(s, launch, source),
        Input::UsageSync => Ok(crate::usage_capture::sync(s)),
        Input::ReviewReadiness { key } => crate::readiness::report(s, &key),
        Input::ReconcilePlan { key } => crate::reconciliation::plan(s, key.as_deref()),
        Input::BeginStage { request } => crate::stages::begin(s, request, at),
        Input::EndStage { key } => crate::stages::end(s, &key, at),
        Input::Status => Ok(json!(s)),
        Input::Next => Ok(next(s, at)),
        Input::Init { .. } => bail!("run already initialized"),
        _ => bail!("incorrect support command"),
    }
}

fn transition_for(transitions: &[Transition], target: &str) -> Result<String> {
    let matches: Vec<_> = transitions.iter().filter(|t| t.to == target).collect();
    ensure!(
        matches.len() == 1,
        "missing or ambiguous transition to target status"
    );
    required_text(&matches[0].id, "transition id")?;
    Ok(matches[0].id.clone())
}
fn expected_status<'a>(s: &'a State, t: &Task) -> &'a str {
    if t.final_verified {
        &s.statuses.done
    } else if t.pr_open {
        &s.statuses.review
    } else {
        &s.statuses.progress
    }
}

fn task_action(s: &State, t: &Task) -> &'static str {
    if (t.spec.jira_status != s.statuses.progress || t.sync != Sync::Confirmed)
        && !t.pr_open
        && !t.final_verified
    {
        "synchronize-jira"
    } else if t.criteria.is_empty() {
        "prepare-brief"
    } else if t.slot.is_none() {
        "reserve-and-bind-slot"
    } else if loops::planned(s, t).is_err() {
        "prepare-loop-plan"
    } else if loops::needs_checkpoint(s, t) {
        "checkpoint-implementation"
    } else if t.final_verified && t.sync == Sync::Confirmed && t.spec.jira_status == s.statuses.done
    {
        "complete"
    } else if t.final_verified {
        "synchronize-jira-done"
    } else if t.main_verified {
        if crate::stages::current(t).is_some() {
            "end-stage"
        } else if reviewed(s, t).is_err() {
            "record-full-evidence-and-review"
        } else if loops::human_review(s, t).is_err() {
            "await-human-review"
        } else {
            "final-acceptance-verification"
        }
    } else if t.pr_open && t.spec.jira_status != s.statuses.review {
        "synchronize-jira-review"
    } else {
        work_action(s, t)
    }
}
fn work_action(s: &State, t: &Task) -> &'static str {
    if t.review
        .as_ref()
        .is_some_and(|r| r.output.verdict == Verdict::Pass)
        && acceptance(s, t).is_ok()
    {
        delivery_action(s, t)
    } else if t.reviews >= 3 && crate::authority::current_pair(&t.authorities).is_none() {
        "record-needs-human"
    } else if t
        .review
        .as_ref()
        .is_some_and(|r| r.output.verdict == Verdict::ChangesRequired)
    {
        if loops::implementation_budget(s, t).is_err() {
            "validate-existing-work-or-hold"
        } else {
            "disposition-and-repair"
        }
    } else if t
        .review
        .as_ref()
        .is_some_and(|r| r.output.verdict == Verdict::Blocked)
    {
        "resolve-evidence-gaps-or-hold"
    } else if acceptance(s, t).is_ok() {
        "independent-review"
    } else if loops::implementation_budget(s, t).is_err() {
        "validate-existing-work-or-hold"
    } else {
        "implement-or-validate"
    }
}
fn delivery_action(s: &State, t: &Task) -> &'static str {
    if t.pr.is_none() {
        if t.snapshot
            .as_ref()
            .is_some_and(|snapshot| on_main(&s.repo, &snapshot.head).is_ok())
        {
            if crate::stages::current(t).is_some() {
                "end-stage"
            } else if reviewed(s, t).is_err() {
                "record-full-evidence-and-review"
            } else if loops::human_review(s, t).is_err() {
                "await-human-review"
            } else {
                "final-acceptance-verification"
            }
        } else {
            "create-pr"
        }
    } else if t.draft {
        "mark-ready"
    } else if loops::human_review(s, t).is_err() {
        "await-human-review"
    } else {
        "poll-checks-and-merge"
    }
}
