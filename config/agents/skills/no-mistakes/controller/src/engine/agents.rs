use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::AgentBegin { .. } => handle_agent_begin(s, input, at),
        input @ Input::AgentFinish { .. } => handle_agent_finish(s, input, at),
        input @ Input::Disposition { .. } => handle_disposition(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_agent_begin(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::AgentBegin {
        key,
        role,
        model,
        effort,
        reason,
        ci_repair,
        allow_evidence_gaps,
    } = input
    else {
        bail!("incorrect command routed to handle_agent_begin");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(t.tier.is_some(), "classify risk first");
    crate::readiness::before_launch(s, &t, &role, allow_evidence_gaps, &reason)?;
    let context = loops::before_agent(s, &t, &role)?;
    let documentation = launch_policy(s, &t, &role, &model, &effort, ci_repair)?;
    let snap = if role == Role::Implementer {
        None
    } else {
        Some(fresh(s, &t)?)
    };
    match role {
        Role::Implementer => {
            prepare_implementation(&mut t, &model, &reason, ci_repair, documentation)?
        }
        Role::Reviewer => prepare_review(s, &mut t, &model, &reason, &snap, documentation)?,
        Role::Escalation => prepare_escalation(s, &mut t, &model, &reason)?,
    }
    crate::stages::guard_launch(&t, &role)?;
    let session = match role {
        Role::Implementer => t.implementer_session.clone(),
        Role::Reviewer => t.reviewer_session.clone(),
        Role::Escalation => None,
    };
    let launch = Launch {
        usage_source: None,
        context: context.clone(),
        id: s.id(),
        key: key.to_string(),
        role,
        model,
        effort,
        snapshot: snap,
        session,
        started: at,
        ended: None,
        outcome: None,
        reason,
        usage: None,
        process: None,
        managed: false,
    };
    record_authorized_launch(&mut t, &launch, documentation)?;
    s.launches.push(launch.clone());
    save(s, &key, t);
    let mut receipt = json!(launch);
    receipt["context"] = context;
    Ok(receipt)
}

fn handle_agent_finish(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::AgentFinish {
        launch,
        session,
        outcome,
        review,
        usage,
    } = input
    else {
        bail!("incorrect command routed to handle_agent_finish");
    };

    required_text(&session, "actual session id")?;
    ensure!(
        ["completed", "failed", "cancelled"].contains(&outcome.as_str()),
        "invalid agent outcome"
    );
    let idx = s
        .launches
        .iter()
        .position(|l| l.id == launch)
        .context("unknown launch")?;
    let l = s.launches[idx].clone();
    ensure!(l.ended.is_none(), "launch is already terminal");
    ensure!(
        !l.managed || l.process.is_some(),
        "managed launch has uncertain startup; establish process identity before releasing it"
    );
    if let Some(process) = &l.process {
        ensure!(
            process.terminal,
            "controller-owned process group is not verified stopped; use recover-agent"
        );
    }
    let mut t = active(s, &l.key)?;
    if let Some(expected) = &l.session {
        ensure!(expected == &session, "resume the existing role session");
    }
    match l.role {
        Role::Implementer => {
            ensure!(
                t.reviewer_session.as_ref() != Some(&session),
                "implementer and reviewer sessions must differ"
            );
            ensure!(
                review.is_none(),
                "implementer cannot submit independent review"
            );
            t.implementer_session = Some(session.clone());
        }
        Role::Reviewer => {
            ensure!(
                t.implementer_session.as_ref() != Some(&session),
                "review must use a separate session"
            );
            t.reviewer_session = Some(session.clone());
            if outcome == "completed" {
                let output = review.context("review output required")?;
                ensure!(
                    Some(&output.snapshot) == l.snapshot.as_ref()
                        && output.snapshot == fresh(s, &t)?,
                    "review snapshot drift"
                );
                crate::review_format::check(&output)?;
                if output.verdict == Verdict::Pass {
                    ensure!(
                        output.findings.is_empty() && output.evidence_gaps.is_empty(),
                        "PASS cannot include unresolved findings or gaps"
                    );
                    acceptance(s, &t)?;
                    ensure!(
                        t.findings.keys().all(|id| t.dispositions.contains_key(id)),
                        "disposition prior findings before PASS"
                    );
                }
                for f in &output.findings {
                    t.dispositions.remove(&f.id);
                    t.findings.insert(f.id.clone(), f.clone());
                }
                t.review = Some(Review { launch, output });
            } else {
                ensure!(
                    review.is_none(),
                    "failed/cancelled launch cannot supply a verdict"
                );
            }
        }
        Role::Escalation => {
            ensure!(
                review.is_none(),
                "Astra consultation cannot bypass the Sol review budget"
            );
        }
    }
    s.launches[idx].session = Some(session);
    s.launches[idx].ended = Some(at);
    s.launches[idx].outcome = Some(outcome);
    let (usage, warning) = crate::telemetry::settlement_usage(usage);
    s.launches[idx].usage = usage;
    save(s, &l.key, t);
    let captured_warning = crate::usage_capture::settle(s, launch);
    let mut result = next(s, at);
    if let Some(warning) = captured_warning.as_deref().or(warning) {
        result["usage_warning"] = json!(warning);
    }
    Ok(result)
}

fn handle_disposition(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Disposition {
        key,
        finding,
        decision,
        evidence,
    } = input
    else {
        bail!("incorrect command routed to handle_disposition");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(t.findings.contains_key(&finding), "unknown finding");
    ensure!(
        ["accepted", "declined", "corrected"].contains(&decision.as_str()),
        "invalid disposition"
    );
    required_text(&evidence, "disposition evidence")?;
    t.dispositions
        .insert(finding, format!("{decision}: {evidence}"));
    save(s, &key, t);
    Ok(json!({"recorded":true}))
}

fn prepare_implementation(
    t: &mut Task,
    model: &str,
    reason: &str,
    ci_repair: bool,
    documentation: bool,
) -> Result<()> {
    ensure!(
        model == "gpt-5.6-sol" || (t.tier == Some(Tier::Routine) && model == "gpt-5.6-terra"),
        "implementation model violates policy"
    );
    if let Some(old) = &t.implementer_model {
        if old != model {
            ensure!(
                old == "gpt-5.6-terra" && model == "gpt-5.6-sol",
                "unsupported model transfer"
            );
            required_text(reason, "Terra diagnosis and transfer evidence")?;
        }
    } else if t.tier == Some(Tier::Routine) && model == "gpt-5.6-sol" {
        required_text(reason, "Terra unavailability evidence")?;
    }
    if t.reviews > 0 && !documentation {
        ensure!(
            t.reviews < 3
                || crate::authority::current_pair(&t.authorities)
                    .is_some_and(|entry| crate::authority::pair_available(entry, false)),
            "review budget exhausted"
        );
        if t.last_repair_review != Some(t.reviews) {
            ensure!(
                t.repairs < 2
                    || crate::authority::current_pair(&t.authorities)
                        .is_some_and(|entry| crate::authority::pair_available(entry, true)),
                "repair budget exhausted"
            );
            t.repairs += 1;
            t.last_repair_review = Some(t.reviews);
        }
    }
    if ci_repair {
        ensure!(
            t.ci_repairs < 1 && t.reviews < 3,
            "CI repair budget exhausted"
        );
        required_text(reason, "task-caused CI diagnosis")?;
        t.ci_repairs += 1;
    }
    t.implementer_model = Some(model.to_owned());
    t.review = None;
    t.final_verified = false;
    t.loop_state.human_review = None;

    Ok(())
}
fn prepare_review(
    s: &State,
    t: &mut Task,
    model: &str,
    reason: &str,
    snap: &Option<Snapshot>,
    documentation: bool,
) -> Result<()> {
    if model == "gpt-6-astra" {
        ensure!(
            t.escalations < 1
                && s.launches.iter().any(|l| l.key == t.spec.key
                    && l.role == Role::Reviewer
                    && l.model == "gpt-5.6-sol"
                    && l.ended.is_some()),
            "Astra review requires an unresolved prior Sol review and remaining escalation budget"
        );
        required_text(
            reason,
            "consequential unresolved question and required evidence",
        )?;
        t.escalations += 1;
    } else {
        ensure!(model == "gpt-5.6-sol", "independent reviewer must be Sol");
    }
    ensure!(
        documentation
            || t.reviews < 3
            || crate::authority::current_pair(&t.authorities)
                .is_some_and(|entry| crate::authority::pair_available(entry, false)),
        "review budget exhausted"
    );
    // Missing acceptance can be reported BLOCKED; it can never result in PASS.
    if s.launches
        .iter()
        .any(|l| l.key == t.spec.key && l.role == Role::Reviewer && &l.snapshot == snap)
    {
        required_text(
            reason,
            "new evidence or specific unresolved review question",
        )?;
    }
    if !documentation {
        t.reviews += 1;
    }
    t.review = None;

    Ok(())
}

fn validate_pair(t: &Task, role: &Role) -> Result<()> {
    let Some(pair) = crate::authority::current_pair(&t.authorities) else {
        return Ok(());
    };
    if *role == Role::Escalation {
        return Ok(());
    }
    crate::authority::validate_receipt(pair)?;
    let revision = crate::stages::revision(t)?;
    ensure!(
        pair.requirements.as_ref() == revision
            || t.requirements.as_deref() == Some(pair.requirements.as_ref()),
        "authority does not cover current criteria"
    );
    ensure!(
        crate::authority::pair_available(pair, *role == Role::Implementer),
        "pair grant is spent"
    );
    Ok(())
}

fn record_authorized_launch(t: &mut Task, launch: &Launch, documentation: bool) -> Result<()> {
    if documentation {
        crate::documentation::record_launch(t, launch)
    } else {
        record_extra_launch(t, launch)
    }
}

fn record_extra_launch(t: &mut Task, launch: &Launch) -> Result<()> {
    if let Some(pair) = crate::authority::current_pair_mut(&mut t.authorities) {
        crate::authority::record_pair(pair, launch.role == Role::Implementer, launch.id)?;
    }
    Ok(())
}

fn launch_policy(
    s: &State,
    t: &Task,
    role: &Role,
    model: &str,
    effort: &str,
    ci_repair: bool,
) -> Result<bool> {
    let documentation = crate::documentation::before_launch(s, t, role)?;
    if !documentation {
        validate_pair(t, role)?;
    }
    ensure!(
        !documentation || !ci_repair,
        "documentation exception cannot authorize CI repair"
    );
    let expected_effort =
        if t.tier == Some(Tier::HighRisk) || *role == Role::Escalation || model == "gpt-6-astra" {
            "high"
        } else {
            "medium"
        };
    ensure!(
        effort == expected_effort,
        "reasoning effort violates policy"
    );
    Ok(documentation)
}

fn prepare_escalation(s: &State, t: &mut Task, model: &str, reason: &str) -> Result<()> {
    ensure!(model == "gpt-6-astra", "escalation must use Astra");
    ensure!(t.escalations < 1, "Astra budget exhausted");
    ensure!(
        s.launches
            .iter()
            .any(|l| l.key == t.spec.key && l.model == "gpt-5.6-sol" && l.ended.is_some()),
        "Sol must investigate before Astra"
    );
    required_text(
        reason,
        "consequential unresolved question and required evidence",
    )?;
    t.escalations += 1;
    Ok(())
}
