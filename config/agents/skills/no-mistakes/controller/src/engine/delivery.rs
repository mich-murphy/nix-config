use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::GhPrepare { .. } => handle_gh_prepare(s, input, at),
        input @ Input::FinalVerify { .. } => handle_final_verify(s, input, at),
        input @ Input::Complete { .. } => handle_complete(s, input, at),
        input @ Input::CleanupCheck { .. } => handle_cleanup_check(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_gh_prepare(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::GhPrepare {
        key,
        action,
        title,
        body,
        method,
    } = input
    else {
        bail!("incorrect command routed to handle_gh_prepare");
    };

    idle(s)?;
    let t = active(s, &key)?;
    fresh(s, &t)?;
    match action {
        GhAction::CreatePr => {
            ensure!(t.pr.is_none(), "PR already exists");
            let snap = t.snapshot.as_ref().context("snapshot")?;
            let path = worktree(s, &t)?;
            ensure!(
                !git(
                    &path,
                    &[
                        "diff",
                        "--name-only",
                        &format!("{}...{}", snap.base, snap.head)
                    ]
                )?
                .is_empty(),
                "already-satisfied work must not create an empty PR"
            );
            required_text(title.as_deref().unwrap_or(""), "PR title")?;
            artifact(body.as_ref().context("PR body file required")?)?;
        }
        GhAction::Ready => {
            ensure!(t.pr_open && t.draft, "no draft PR");
        }
        GhAction::Merge => {
            reviewed(s, &t)?;
            loops::human_review(s, &t)?;
            ensure!(t.pr_open && !t.draft, "PR must be open and ready");
            ensure!(
                t.checks_head.as_deref() == t.snapshot.as_ref().map(|x| x.head.as_ref()),
                "checks have not been observed for this head"
            );
            ensure!(
                !t.checks.is_empty()
                    && t.checks
                        .iter()
                        .all(|v| matches!(v["bucket"].as_str(), Some("pass" | "skipping"))),
                "required checks have not passed"
            );
            ensure!(
                matches!(method.as_deref(), Some("merge" | "squash" | "rebase")),
                "explicit repository merge method required"
            );
        }
    }
    let digest = body.as_ref().map(|p| artifact(p)).transpose()?;
    let op = Operation {
        id: s.id(),
        key: key.to_string(),
        kind: "github".into(),
        status: OpStatus::Prepared,
        created: at,
        snapshot: t.snapshot.clone(),
        attempts: 0,
        details: json!({"action":action,"title":title,"body":body,"body_digest":digest,"method":method,"branch":t.branch,"pr":t.pr}),
        observation: None,
        process: None,
    };
    s.operations.push(op.clone());
    Ok(json!(op))
}

fn handle_final_verify(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::FinalVerify {
        key,
        main_commit,
        evidence,
    } = input
    else {
        bail!("incorrect command routed to handle_final_verify");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    crate::stages::full_acceptance(&t)?;
    reviewed(s, &t)?;
    loops::human_review(s, &t)?;
    crate::followup::verify_history(s, &t)?;
    artifact(&evidence)?;
    on_main(&s.repo, &main_commit)?;
    if let Some(commit) = &t.merged_commit {
        ensure!(
            sha(&s.repo, commit)? == sha(&s.repo, &main_commit)?,
            "wrong merge commit"
        );
    } else {
        ensure!(
            t.pr.is_none() && t.snapshot.as_ref().unwrap().head == sha(&s.repo, &main_commit)?,
            "already-satisfied proof must identify the reviewed main head"
        );
        t.merged_commit = Some(main_commit.clone());
    }
    t.main_verified = true;
    t.final_verified = true;
    t.sync = Sync::Pending;
    t.next_action = format!(
        "Final acceptance recorded at {}; synchronize Jira Done",
        evidence.display()
    );
    save(s, &key, t);
    Ok(next(s, at))
}

fn handle_complete(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::Complete { key } = input else {
        bail!("incorrect command routed to handle_complete");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    ensure!(
        t.final_verified
            && t.main_verified
            && t.sync == Sync::Confirmed
            && t.spec.jira_status == s.statuses.done
            && t.spec.resolved,
        "delivery or Jira synchronization remains incomplete"
    );
    ensure!(
        t.spec.subtasks.iter().all(|key| t
            .subtasks
            .get(key)
            .is_some_and(|sub| sub.status == s.statuses.done
                && sub.resolved
                && sub.sync == Sync::Confirmed)),
        "subtask acceptance or Jira synchronization remains incomplete"
    );
    crate::stages::full_acceptance(&t)?;
    reviewed(s, &t)?;
    loops::human_review(s, &t)?;
    on_main(
        &s.repo,
        t.merged_commit.as_ref().context("missing main commit")?,
    )?;
    t.delivery = if t.pr.is_some() {
        Delivery::Merged
    } else {
        Delivery::AlreadySatisfied
    };
    t.next_action = "Run cleanup-check, then repository cleanup helper".into();
    save(s, &key, t);
    s.active = None;
    Ok(next(s, at))
}

fn handle_cleanup_check(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::CleanupCheck { key } = input else {
        bail!("incorrect command routed to handle_cleanup_check");
    };

    idle(s)?;
    let t = task(s, &key)?;
    ensure!(
        s.active.as_deref() != Some(&key)
            && matches!(t.delivery, Delivery::Merged | Delivery::AlreadySatisfied)
            && t.final_verified
            && t.sync == Sync::Confirmed,
        "cannot clean unfinished work"
    );
    on_main(
        &s.repo,
        t.merged_commit
            .as_ref()
            .context("missing delivery commit")?,
    )?;
    let path = worktree(s, &t)?;
    clean(&path)?;
    let head = sha(&path, "HEAD")?;
    let reset_slot = git(&path, &["branch", "--show-current"])? == *t.slot.as_ref().unwrap()
        && head == sha(&path, "origin/main")?;
    ensure!(
        Some(head.as_ref()) == t.snapshot.as_ref().map(|x| x.head.as_ref()) || reset_slot,
        "unrecorded commits must be preserved"
    );
    Ok(
        json!({"allowed":true,"slot":t.slot,"path":path,"method":"repository helper only; inspect the selected slot's assigned port and preview deletion"}),
    )
}
