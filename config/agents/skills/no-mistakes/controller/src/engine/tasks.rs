use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::Brief { .. } => handle_brief(s, input, at),
        input @ Input::ReserveSlot { .. } => handle_reserve_slot(s, input, at),
        input @ Input::Bind { .. } => handle_bind(s, input, at),
        input @ Input::Snapshot { .. } => handle_snapshot(s, input, at),
        input @ Input::Evidence { .. } => handle_evidence(s, input, at),
        input @ Input::SubtaskRecord { .. } => handle_subtask_record(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_brief(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Brief {
        key,
        criteria,
        tier,
        reason,
    } = input
    else {
        bail!("incorrect command routed to handle_brief");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(
        t.sync == Sync::Confirmed
            && (t.spec.jira_status == s.statuses.progress
                || (t.pr_open && t.spec.jira_status == s.statuses.review)),
        "confirm Jira's actual stage before detailed work"
    );
    required_text(&reason, "risk classification reason")?;
    ensure!(!criteria.is_empty(), "acceptance criteria required");
    let mut ids = BTreeSet::new();
    for c in &criteria {
        required_text(&c.id, "criterion id")?;
        required_text(&c.text, "criterion text")?;
        ensure!(ids.insert(c.id.clone()), "duplicate criterion");
    }
    let revision = hash(&serde_json::to_vec(&criteria)?);
    crate::stages::guard_brief(&t, &revision)?;
    crate::followup::requirements(&t, &revision)?;
    if t.requirements.as_ref() != Some(&revision) {
        t.evidence.clear();
        t.review = None;
        t.final_verified = false;
        t.loop_state.human_review = None;
    }
    t.requirements = Some(revision.clone());
    t.criteria = criteria;
    t.tier = Some(tier);
    t.tier_reason = reason;
    if let Some(snap) = &mut t.snapshot {
        snap.requirements = revision.clone();
    }
    save(s, &key, t);
    Ok(json!({"requirements":revision}))
}

fn handle_reserve_slot(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::ReserveSlot { slot } = input else {
        bail!("incorrect command routed to handle_reserve_slot");
    };

    idle(s)?;
    ensure!(s.active.is_some(), "claim a task first");
    let reservations = s.repo.join(".local/epic-control/slots");
    fs::create_dir_all(&reservations)?;
    ensure!(
        reservations.canonicalize()? == reservations,
        "reservation path is redirected"
    );
    let path = reservations.join(slot.as_ref());
    if path.exists() {
        ensure!(
            fs::read_to_string(&path)? == s.run_id,
            "slot belongs to another run"
        );
    } else {
        ensure!(
            !s.repo.join(".worktrees").join(slot.as_ref()).exists(),
            "existing slot of unknown ownership is occupied"
        );
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("slot allocation collision")?;
        f.write_all(s.run_id.as_bytes())?;
        f.sync_all()?;
    }
    s.slots.insert(slot.to_string(), s.run_id.clone());
    Ok(
        json!({"slot":slot,"next":"Create the reserved slot with the repository helper, then create a task branch from fresh origin/main and bind it."}),
    )
}

fn handle_bind(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Bind {
        key,
        slot,
        historical_slot_reuse,
    } = input
    else {
        bail!("incorrect command routed to handle_bind");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(
        t.slot.is_none(),
        "task already bound; resume its existing slot"
    );
    ensure!(
        unfinished_slot_owner(s, key.as_ref(), slot.as_ref())?.is_none(),
        "slot contains unfinished work"
    );
    t.slot = Some(slot.to_string());
    let path = worktree(s, &t)?;
    clean(&path)?;
    let branch = git(&path, &["branch", "--show-current"])?;
    ensure!(
        branch.starts_with("agent-")
            && branch.contains(&format!("/{slot}/"))
            && branch.contains(key.as_ref()),
        "task branch must identify agent, slot and Jira key"
    );
    let head = sha(&path, "HEAD")?;
    let base = sha(&path, "origin/main")?;
    ensure!(
        head == base,
        "new task branch must start at fresh origin/main; existing work must resume its original run"
    );
    if let Some(request) = historical_slot_reuse {
        crate::followup::authorize_historical_slot_reuse(
            s,
            &mut t,
            crate::followup::HistoricalSlotBinding {
                key: &key,
                slot: &slot,
                base: &base,
                request,
                recorded: at,
            },
        )?;
        s.version = s.version.max(10);
    } else {
        crate::followup::binding(s, &t, &slot)?;
        if let Some(followup) = &t.followup {
            ensure!(
                base == followup.base,
                "origin/main changed; use an explicit current-user historical-slot reuse authorization or prepare a new task-scoped recovery decision"
            );
        }
    }
    t.branch = Some(branch);
    t.snapshot = Some(Snapshot {
        base: base.parse()?,
        head: head.parse()?,
        requirements: t.requirements.clone().context("brief required")?,
    });
    save(s, &key, t);
    Ok(next(s, at))
}

fn handle_snapshot(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Snapshot { key } = input else {
        bail!("incorrect command routed to handle_snapshot");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let snap = snapshot(s, &t)?;
    crate::documentation::validate_scope(s, &t, &snap)?;
    crate::superseded::scope(s, &t, &snap)?;
    if t.snapshot.as_ref() != Some(&snap) {
        t.review = None;
        t.evidence.clear();
        t.final_verified = false;
        t.loop_state.human_review = None;
        t.checks.clear();
        t.checks_head = None;
    }
    t.snapshot = Some(snap.clone());
    save(s, &key, t);
    Ok(json!(snap))
}

fn handle_evidence(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Evidence { key, evidence } = input else {
        bail!("incorrect command routed to handle_evidence");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let snap = fresh(s, &t)?;
    let criterion = crate::stages::criteria(&t)
        .iter()
        .find(|c| c.id == evidence.criterion)
        .context("unknown criterion")?;
    required_text(&evidence.description, "evidence description")?;
    ensure!(
        !evidence.implementation.is_empty(),
        "implementation references required"
    );
    if criterion.human_only {
        ensure!(
            evidence.kind == EvidenceKind::Human,
            "human criterion cannot be satisfied by automated evidence"
        );
    }
    if evidence.kind == EvidenceKind::Human {
        required_text(
            evidence.human_source.as_deref().unwrap_or(""),
            "current user's supplied acceptance source",
        )?;
    }
    let digest = artifact(&evidence.artifact)?;
    t.evidence.insert(
        evidence.criterion.clone(),
        Evidence {
            input: evidence,
            digest: digest.parse()?,
            snapshot: snap,
        },
    );
    // Changed evidence must be assessed by the reviewer, even if code did not change.
    t.review = None;
    t.final_verified = false;
    t.loop_state.human_review = None;
    save(s, &key, t);
    Ok(json!({"recorded":true}))
}

fn handle_subtask_record(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::SubtaskRecord {
        key,
        issue,
        current,
        resolved,
        criteria,
        ownership_evidence,
    } = input
    else {
        bail!("incorrect command routed to handle_subtask_record");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(
        t.spec
            .subtasks
            .iter()
            .any(|subtask| subtask.as_str() == &*issue),
        "subtask is outside frozen task membership"
    );
    ensure!(
        !criteria.is_empty()
            && criteria
                .iter()
                .all(|id| t.criteria.iter().any(|c| &c.id == id)),
        "subtask requires its own mapped acceptance criteria"
    );
    required_text(&ownership_evidence, "subtask ownership evidence")?;
    ensure!(
        !t.subtasks.contains_key::<str>(issue.as_ref()),
        "subtask already recorded; use transition observations to update it"
    );
    let terminal = resolved || current == s.statuses.done;
    t.subtasks.insert(
        issue.to_string(),
        SubtaskState {
            status: current,
            resolved,
            was_terminal: terminal,
            owned: !terminal,
            criteria,
            source: ownership_evidence,
            sync: Sync::Pending,
        },
    );
    save(s, &key, t);
    Ok(json!({"recorded":true}))
}
