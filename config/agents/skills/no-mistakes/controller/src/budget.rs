//! Explicit user authority for one implementation and review pair.
use crate::{
    authority::{self, Authority, Grant},
    model::*,
    store::*,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdjustmentMode {
    AdditionalCycle,
    DocumentationCycle,
}

pub fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    let Input::AuthorizeBudgetAdjustment {
        key,
        mode,
        source,
        artifact: receipt,
        scope,
        paths,
    } = input
    else {
        anyhow::bail!("incorrect budget command");
    };
    crate::engine::idle(s)?;
    ensure!(s.active.is_none(), "hold before recording pair authority");
    required_text(&scope, "bounded adjustment scope and unchanged criteria")?;
    required_text(&source, "actual current-user authorization source")?;
    let t = s.tasks.get(key.as_ref()).context("unknown task")?;
    ensure!(
        t.started_by_run
            && !t.was_terminal
            && !t.final_verified
            && t.delivery == Delivery::NeedsHuman,
        "pair authority requires unfinished run-owned held work"
    );
    ensure!(
        t.reviews >= 3 || t.followup.is_some(),
        "pair authority requires exhausted or follow-up work"
    );
    let scoped = matches!(mode, AdjustmentMode::DocumentationCycle);
    ensure!(
        scoped || paths.is_empty(),
        "source pair does not take paths"
    );
    let grant_paths = scoped.then_some(paths);
    authority::validate_paths(grant_paths.as_deref())?;
    if let Some(paths) = &grant_paths {
        let checkout = worktree(s, t)?;
        let revision = t.snapshot.as_ref().context("snapshot")?.head.as_ref();
        for path in paths {
            let entry = git(&checkout, &["ls-tree", revision, "--", path])?;
            ensure!(
                entry.starts_with("100644 blob "),
                "documentation path must exist as regular Markdown"
            );
        }
    }
    let digest = artifact(&receipt)?;
    let requirements = crate::stages::revision(t)?.parse()?;
    let existing = t
        .authorities
        .iter()
        .position(|entry| entry.digest == digest);
    let authority = if let Some(index) = existing {
        let mut entry = t.authorities[index].clone();
        ensure!(
            authority::receipt_only(&entry)
                && entry.artifact == receipt
                && entry.requirements == requirements,
            "authority digest already registered"
        );
        authority::validate_receipt(&entry)?;
        entry.grant = Grant::Pair { paths: grant_paths };
        entry.used = None;
        entry
    } else {
        let entry = Authority {
            id: s.serial + 1,
            source,
            artifact: receipt,
            digest,
            requirements,
            recorded: at,
            grant: Grant::Pair { paths: grant_paths },
            used: None,
        };
        authority::register(&t.authorities, &entry, true)?;
        entry
    };
    let task = s.tasks.get_mut(key.as_ref()).context("unknown task")?;
    if let Some(index) = existing {
        task.authorities[index] = authority.clone();
    } else {
        let id = s.id();
        ensure!(id == authority.id, "authority identity changed");
        s.tasks
            .get_mut(key.as_ref())
            .context("unknown task")?
            .authorities
            .push(authority.clone());
    }
    Ok(json!({"authorization":authority,"acceptance_unchanged":true}))
}
