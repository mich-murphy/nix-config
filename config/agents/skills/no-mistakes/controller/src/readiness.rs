//! Cheap pre-launch acceptance check. Explicit gap reviews remain possible.
use crate::{engine, model::*, store::*};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub fn report(s: &State, key: &str) -> Result<Value> {
    let t = s.tasks.get(key).context("unknown task")?;
    let problem = engine::acceptance(s, t).err().map(|e| e.to_string());
    Ok(
        json!({"key":key,"snapshot":t.snapshot,"stage":crate::stages::current(t),
        "ready":problem.is_none(),"blocker":problem,"launches_consumed":0,
        "guidance":"Product/stage criteria must be evidenced before normal review. The current independent review and merge are controller gates, never criteria that require their own settlement."}),
    )
}
pub fn before_launch(s: &State, t: &Task, role: &Role, gaps: bool, reason: &str) -> Result<()> {
    crate::stages::validate(t)?;
    crate::followup::before_launch(s, t, role)?;
    ensure!(
        !gaps || *role == Role::Reviewer,
        "gap-review option belongs only to reviewers"
    );
    if *role != Role::Reviewer {
        return Ok(());
    }
    if gaps {
        required_text(reason, "specific evidence-gap investigation question")?;
    } else {
        engine::acceptance(s, t).context("review is not ready; record missing proof before consuming a launch, or explicitly request a gap investigation")?;
    }
    Ok(())
}

/// Keep the original native result even if a later freshness/settlement guard rejects it.
/// This receipt is evidence, not an accepted review and never grants another launch.
pub fn preserve(root: &std::path::Path, s: &State, input: &Input) -> Result<()> {
    let Input::AgentFinish {
        launch,
        session,
        outcome,
        review: Some(review),
        ..
    } = input
    else {
        return Ok(());
    };
    let l = s
        .launches
        .iter()
        .find(|l| l.id == *launch)
        .context("unknown launch")?;
    ensure!(
        l.role == Role::Reviewer && l.ended.is_none(),
        "receipt requires a live reviewer claim"
    );
    ensure!(
        l.snapshot.as_ref() == Some(&review.snapshot),
        "receipt differs from launched snapshot"
    );
    crate::review_format::check(review)?;
    let data = serde_json::to_vec_pretty(
        &json!({"launch":launch,"session":session,"outcome":outcome,"review":review}),
    )?;
    let path = root.join(format!("native-review-{launch}-{}.json", hash(&data)));
    if path.exists() {
        ensure!(std::fs::read(&path)? == data, "review receipt changed");
        return Ok(());
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    use std::io::Write;
    file.write_all(&data)?;
    file.sync_all()?;
    Ok(())
}
