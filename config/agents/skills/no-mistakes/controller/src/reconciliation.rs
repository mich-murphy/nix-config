//! Select remote reads without revisiting every settled PR on each task.
use crate::{model::*, store::*};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn completed(s: &State, t: &Task) -> bool {
    s.active.as_deref() != Some(&t.spec.key)
        && t.main_verified
        && t.final_verified
        && t.sync == Sync::Confirmed
        && t.spec.jira_status == s.statuses.done
        && t.spec.resolved
        && t.pr.is_some()
        && !t.pr_open
        && !t.draft
        && t.delivery == Delivery::Merged
        && t.spec.subtasks.iter().all(|key| {
            t.subtasks.get(key).is_some_and(|sub| {
                sub.status == s.statuses.done && sub.resolved && sub.sync == Sync::Confirmed
            })
        })
}
pub fn plan(s: &State, key: Option<&str>) -> Result<Value> {
    let mut inspect: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(key) = key.or(s.active.as_deref()) {
        let t = s.tasks.get(key).context("unknown selected task")?;
        inspect
            .entry(key.into())
            .or_default()
            .push("selected task".into());
        for dep in &t.spec.dependencies {
            if let Some(parent) = s.tasks.get(&dep.key) {
                let reusable = completed(s, parent)
                    && dep.verified
                    && dep
                        .main_commit
                        .as_ref()
                        .is_none_or(|sha| on_main(&s.repo, sha).is_ok());
                if !reusable {
                    inspect
                        .entry(dep.key.clone())
                        .or_default()
                        .push("unsettled or changed prerequisite".into());
                }
            }
        }
    }
    for op in &s.operations {
        if matches!(op.status, OpStatus::Prepared | OpStatus::Unknown) {
            inspect
                .entry(op.key.clone())
                .or_default()
                .push("pending or uncertain operation".into());
        }
    }
    for t in s.tasks.values() {
        if t.pr_open
            || t.sync == Sync::Failed
            || (key.is_none() && t.pr.is_some() && !completed(s, t))
        {
            inspect
                .entry(t.spec.key.clone())
                .or_default()
                .push("unfinished delivery or synchronization".into());
        }
    }
    let reused: Vec<_> = s
        .tasks
        .values()
        .filter(|t| completed(s, t) && !inspect.contains_key(&t.spec.key))
        .map(|t| json!({"key":t.spec.key,"pr":t.pr,"merge":t.merged_commit}))
        .collect();
    Ok(
        json!({"inspect":inspect,"reuse_completed":reused,"remote_calls":0,
        "guidance":"Refresh Jira membership/dependencies before selection. Reobserve explicit external changes and uncertain identities. These completion receipts do not cache live operational health or bypass exact-head merge checks."}),
    )
}
