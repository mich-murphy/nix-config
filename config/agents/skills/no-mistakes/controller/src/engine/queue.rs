use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::Discover { .. } => handle_discover(s, input, at),
        input @ Input::Refresh { .. } => handle_refresh(s, input, at),
        input @ Input::Claim { .. } => handle_claim(s, input, at),
        input @ Input::Hold { .. } => handle_hold(s, input, at),
        input @ Input::Resume { .. } => handle_resume(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_discover(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Discover {
        tasks,
        planning_order,
    } = input
    else {
        bail!("incorrect command routed to handle_discover");
    };

    idle(s)?;
    ensure!(!s.frozen, "membership already frozen");
    ensure!(!tasks.is_empty(), "empty epic batch");
    let keys: BTreeSet<_> = tasks.iter().map(|t| t.key.clone()).collect();
    ensure!(keys.len() == tasks.len(), "duplicate task key");
    let mut subs = BTreeSet::new();
    for t in tasks {
        spec_valid(&t)?;
        ensure!(
            !keys.contains(&t.parent),
            "subtasks must be nested under their delivery task, not separately scheduled"
        );
        for sub in &t.subtasks {
            ensure!(
                !keys.contains(sub) && subs.insert(sub.clone()),
                "duplicate delivery/subtask membership"
            );
        }
        s.tasks
            .insert(t.key.to_string(), Task::new(t, &s.statuses.done));
    }
    ensure!(
        planning_order.is_empty()
            || (planning_order.iter().cloned().collect::<BTreeSet<_>>() == keys
                && planning_order.len() == keys.len()),
        "explicit planning order must list every delivery task once"
    );
    s.order = planning_order;
    s.frozen = true;
    s.refreshed = at;
    s.refresh_generation += 1;
    Ok(json!({"frozen":keys,"next":next(s,at)}))
}

fn handle_refresh(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Refresh { tasks } = input else {
        bail!("incorrect command routed to handle_refresh");
    };

    idle(s)?;
    ensure!(s.frozen, "discover first");
    let provided: BTreeSet<_> = tasks.iter().map(|t| t.key.clone()).collect();
    ensure!(
        provided.len() == tasks.len() && s.tasks.keys().all(|key| provided.contains(key.as_str())),
        "refresh must include every frozen task exactly once, including removed tasks"
    );
    for mut spec in tasks {
        spec_valid(&spec)?;
        if let Some(t) = s.tasks.get_mut(&spec.key) {
            ensure!(
                spec.parent == t.spec.parent || !spec.member,
                "parent changed but task still reported as an epic member"
            );
            if spec.subtasks != t.spec.subtasks {
                spec.blocker = Some(
                    "Subtask membership changed after discovery; reconcile frozen scope".into(),
                );
                spec.subtasks = t.spec.subtasks.clone();
            }
            if !spec.member && t.delivery == Delivery::Queued {
                t.delivery = Delivery::Excluded;
            }
            if !t.started_by_run && (spec.resolved || spec.jira_status == s.statuses.done) {
                t.was_terminal = true;
            }
            t.spec = spec;
        } else if !s.followups.contains(&spec.key) {
            s.followups.push(spec.key);
        }
    }
    s.refreshed = at;
    s.refresh_generation += 1;
    Ok(next(s, at))
}

fn handle_claim(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Claim { key } = input else {
        bail!("incorrect command routed to handle_claim");
    };

    idle(s)?;
    ensure!(
        s.active.is_none(),
        "finish or explicitly defer the current task first"
    );
    refreshed(s, at)?;
    loops::capacity(s)?;
    ensure!(
        candidates(s, at).first().map(String::as_str) == Some(key.as_ref()),
        "task is not first in the eligible queue"
    );
    let mut t = task(s, &key)?;
    blocked(s, &t, at)?;
    reserve_issue(s, &key)?;
    t.started_by_run = true;
    t.delivery = Delivery::Active;
    t.next_action = "Transition Jira to In Progress before detailed work".into();
    t.sync = Sync::Pending;
    s.active = Some(key.to_string());
    s.selection_generation = s.refresh_generation;
    save(s, &key, t);
    Ok(next(s, at))
}

fn handle_hold(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Hold {
        key,
        outcome,
        reason,
    } = input
    else {
        bail!("incorrect command routed to handle_hold");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    required_text(&reason, "blocker and exact resumption action")?;
    ensure!(
        matches!(outcome, Delivery::Deferred | Delivery::NeedsHuman),
        "hold must be deferred or needs-human"
    );
    if outcome == Delivery::Deferred {
        let head = &t.snapshot.as_ref().context("snapshot required")?.head;
        ensure!(
            t.deadlines.get(head.as_ref()).is_some_and(|d| at >= *d),
            "CI deadline has not expired; do not overlap another task"
        );
    }
    t.delivery = outcome;
    t.next_action = reason;
    save(s, &key, t);
    s.active = None;
    Ok(next(s, at))
}

fn handle_resume(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Resume { key, final_revisit } = input else {
        bail!("incorrect command routed to handle_resume");
    };

    idle(s)?;
    ensure!(s.active.is_none(), "another task is active");
    refreshed(s, at)?;
    let mut t = task(s, &key)?;
    ensure!(
        matches!(t.delivery, Delivery::Deferred | Delivery::NeedsHuman) || t.sync == Sync::Failed,
        "task is not resumable"
    );
    blocked(s, &t, at)?;
    reserve_issue(s, &key)?;
    if final_revisit {
        ensure!(!t.revisit_used, "final revisit already used");
        t.revisit_used = true;
    }
    t.delivery = Delivery::Active;
    save(s, &key, t);
    s.active = Some(key.to_string());
    s.selection_generation = s.refresh_generation;
    Ok(next(s, at))
}
