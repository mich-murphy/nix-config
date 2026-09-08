//! Atomic measurement/evidence submission. All ordinary guards still execute.
use crate::{engine, ids::TaskId, model::*, store::*};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionResult {
    pub evidence: EvidenceInput,
    pub observed: String,
    pub satisfied: bool,
}

pub fn record(
    s: &mut State,
    key: TaskId,
    snapshot: Snapshot,
    results: Vec<CriterionResult>,
    at: u64,
) -> Result<Value> {
    engine::idle(s)?;
    let t = s
        .tasks
        .get(key.as_ref())
        .ok_or_else(|| anyhow::anyhow!("unknown task"))?;
    ensure!(fresh(s, t)? == snapshot, "batch snapshot drift");
    ensure!(!results.is_empty(), "empty evidence batch");
    let mut seen = std::collections::BTreeSet::new();
    let mut staged = s.clone();
    for result in results {
        let criterion = result.evidence.criterion.clone();
        ensure!(seen.insert(criterion.clone()), "duplicate batch criterion");
        engine::apply(
            &mut staged,
            Input::Measure {
                key: key.clone(),
                criterion,
                observed: result.observed,
                satisfied: result.satisfied,
                artifact: result.evidence.artifact.clone(),
            },
            at,
        )?;
        engine::apply(
            &mut staged,
            Input::Evidence {
                key: key.clone(),
                evidence: result.evidence,
            },
            at,
        )?;
    }
    *s = staged;
    Ok(json!({"key":key,"snapshot":snapshot,"recorded":seen,"review_invalidated":true}))
}
