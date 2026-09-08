use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::JiraPrepare { .. } => handle_jira_prepare(s, input, at),
        input @ Input::Dispatch { .. } => handle_dispatch(s, input, at),
        input @ Input::JiraObserve { .. } => handle_jira_observe(s, input, at),
        input @ Input::JiraRetry { .. } => handle_jira_retry(s, input, at),
        input @ Input::SyncFailed { .. } => handle_sync_failed(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_jira_prepare(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::JiraPrepare {
        key,
        issue,
        current,
        resolved,
        target,
        transitions,
    } = input
    else {
        bail!("incorrect command routed to handle_jira_prepare");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(
        issue == key.as_ref() || t.spec.subtasks.contains(&issue),
        "issue is outside this task"
    );
    if issue == key.as_ref() {
        ensure!(
            target == expected_status(s, &t),
            "target does not match actual delivery stage"
        );
        if target == s.statuses.done {
            reviewed(s, &t)?;
            loops::human_review(s, &t)?;
            on_main(
                &s.repo,
                t.merged_commit
                    .as_ref()
                    .context("verified merge commit required")?,
            )?;
        }
    } else {
        let sub = t
            .subtasks
            .get(&issue)
            .context("record individual subtask ownership and criteria first")?;
        ensure!(
            !sub.was_terminal || current == target,
            "pre-existing terminal subtask cannot be reopened"
        );
        ensure!(
            [&s.statuses.progress, &s.statuses.review, &s.statuses.done].contains(&&target),
            "invalid subtask target"
        );
        if target == s.statuses.done {
            ensure!(t.main_verified, "subtask delivery is not verified on main");
            acceptance(s, &t)?;
            ensure!(
                sub.criteria.iter().all(|id| t
                    .evidence
                    .get(id)
                    .is_some_and(|e| e.input.status == EvidenceStatus::Passed)),
                "subtask acceptance missing"
            );
            reviewed(s, &t)?;
            loops::human_review(s, &t)?;
        } else {
            ensure!(sub.owned, "subtask is not owned by this run");
        }
    }
    ensure!(!t.was_terminal, "cannot reopen pre-existing terminal task");
    if current == s.statuses.done {
        ensure!(
            t.started_by_run,
            "only this run's premature automatic Done may be corrected"
        );
    }
    if current == target {
        ensure!(
            resolved == (target == s.statuses.done),
            "Jira resolution is inconsistent"
        );
        if issue == key.as_ref() {
            t.spec.jira_status = current;
            t.spec.resolved = resolved;
            t.sync = Sync::Confirmed;
        } else {
            let sub = t.subtasks.get_mut(&issue).unwrap();
            sub.status = current;
            sub.resolved = resolved;
            sub.sync = Sync::Confirmed;
        }
        save(s, &key, t);
        return Ok(json!({"action":"already-correct"}));
    }
    let transition = transition_for(&transitions, &target)?;
    let op = Operation {
        id: s.id(),
        key: key.to_string(),
        kind: "jira".into(),
        status: OpStatus::Prepared,
        created: at,
        snapshot: t.snapshot.clone(),
        attempts: 0,
        details: json!({"issue":issue,"before":current,"target":target,"transition":transition}),
        observation: None,
        process: None,
    };
    if issue == key.as_ref() {
        t.sync = Sync::Pending;
    } else {
        t.subtasks.get_mut(&issue).unwrap().sync = Sync::Pending;
    }
    save(s, &key, t);
    s.operations.push(op.clone());
    Ok(json!(op))
}

fn handle_dispatch(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Dispatch { operation } = input else {
        bail!("incorrect command routed to handle_dispatch");
    };

    ensure!(
        !s.launches.iter().any(|l| l.ended.is_none()),
        "agent still active"
    );
    let prepared = s
        .operations
        .iter()
        .find(|o| o.id == operation)
        .context("unknown operation")?;
    if prepared.kind == "jira" && prepared.details["target"] == s.statuses.done {
        let t = &s.tasks[&prepared.key];
        reviewed(s, t)?;
        loops::human_review(s, t)?;
        ensure!(t.main_verified, "delivery is not verified on main");
        on_main(&s.repo, t.merged_commit.as_ref().context("merge commit")?)?;
        ensure!(
            prepared.snapshot == t.snapshot,
            "Done operation snapshot drift"
        );
    }
    let op = s
        .operations
        .iter_mut()
        .find(|o| o.id == operation)
        .context("unknown operation")?;
    ensure!(
        op.status == OpStatus::Prepared,
        "operation already dispatched or terminal; reconcile instead of replaying"
    );
    op.status = OpStatus::Unknown;
    op.attempts += 1;
    if op.kind == "jira" {
        let t = s.tasks.get_mut(&op.key).unwrap();
        if op.details["issue"] == op.key {
            t.sync = Sync::Unknown;
        } else {
            t.subtasks
                .get_mut(op.details["issue"].as_str().unwrap())
                .context("subtask")?
                .sync = Sync::Unknown;
        }
    }
    Ok(json!({"operation":op.id,"kind":op.kind,"details":op.details,"status":"unknown"}))
}

fn handle_jira_observe(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::JiraObserve {
        operation,
        current,
        resolved,
        evidence,
    } = input
    else {
        bail!("incorrect command routed to handle_jira_observe");
    };

    required_text(&evidence, "fresh Jira observation source")?;
    let idx = s
        .operations
        .iter()
        .position(|o| o.id == operation)
        .context("unknown operation")?;
    let op = s.operations[idx].clone();
    ensure!(
        op.kind == "jira" && matches!(op.status, OpStatus::Unknown | OpStatus::Failed),
        "only unresolved operations accept observations"
    );
    ensure!(
        !s.operations.iter().any(|later| later.id > op.id
            && later.kind == "jira"
            && later.details["issue"] == op.details["issue"]),
        "newer Jira operation supersedes this observation"
    );
    let target = op.details["target"].as_str().context("target")?;
    let correct = current == target && resolved == (target == s.statuses.done);
    let status = if correct {
        OpStatus::Confirmed
    } else {
        OpStatus::Failed
    };
    s.operations[idx].status = status;
    s.operations[idx].observation =
        Some(json!({"current":current,"resolved":resolved,"evidence":evidence,"at":at}));
    let mut t = task(s, &op.key)?;
    let sync = if correct {
        Sync::Confirmed
    } else {
        Sync::Failed
    };
    if op.details["issue"] == op.key {
        t.spec.jira_status = current;
        t.spec.resolved = resolved;
        t.sync = sync;
    } else {
        let sub = t
            .subtasks
            .get_mut(op.details["issue"].as_str().unwrap())
            .context("subtask")?;
        sub.status = current;
        sub.resolved = resolved;
        sub.sync = sync;
    }
    save(s, &op.key, t);
    Ok(json!({"confirmed":correct,"retry_available":!correct&&op.attempts<2}))
}

fn handle_jira_retry(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::JiraRetry {
        operation,
        current,
        resolved,
        transitions,
    } = input
    else {
        bail!("incorrect command routed to handle_jira_retry");
    };

    idle(s)?;
    let idx = s
        .operations
        .iter()
        .position(|o| o.id == operation)
        .context("unknown operation")?;
    let op = s.operations[idx].clone();
    ensure!(
        op.kind == "jira" && op.status == OpStatus::Failed && op.attempts < 2,
        "Jira retry unavailable"
    );
    let t = active(s, &op.key)?;
    let target = op.details["target"].as_str().unwrap();
    ensure!(
        op.details["issue"] != op.key || target == expected_status(s, &t),
        "delivery stage changed"
    );
    ensure!(
        current == op.details["before"] && !resolved,
        "remote status changed; do not replay"
    );
    let transition = transition_for(&transitions, target)?;
    s.operations[idx].details["transition"] = json!(transition);
    s.operations[idx].status = OpStatus::Prepared;
    let t = s.tasks.get_mut(&op.key).unwrap();
    if op.details["issue"] == op.key {
        t.sync = Sync::Pending;
    } else {
        t.subtasks
            .get_mut(op.details["issue"].as_str().unwrap())
            .context("subtask")?
            .sync = Sync::Pending;
    }
    Ok(json!(s.operations[idx]))
}

fn handle_sync_failed(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::SyncFailed { operation, reason } = input else {
        bail!("incorrect command routed to handle_sync_failed");
    };

    required_text(&reason, "synchronization failure evidence")?;
    let op = s
        .operations
        .iter()
        .find(|o| o.id == operation)
        .context("unknown operation")?;
    ensure!(
        op.kind == "jira" && op.status == OpStatus::Failed,
        "uncertain operations must be observed before continuing"
    );
    let key = op.key.clone();
    let mut t = active(s, &key)?;
    t.sync = Sync::Failed;
    t.next_action = reason;
    t.delivery = if t.final_verified {
        Delivery::Merged
    } else {
        Delivery::NeedsHuman
    };
    save(s, &key, t);
    s.active = None;
    Ok(next(s, at))
}
