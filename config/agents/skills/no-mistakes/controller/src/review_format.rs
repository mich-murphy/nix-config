//! Supply and validate the runner's exact review contract before settlement.
use crate::model::{ReviewOutput, Snapshot, Verdict};
use crate::store::required_text;
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn schema() -> Value {
    let string = json!({"type":"string","minLength":1});
    let mut fields = serde_json::Map::new();
    for name in [
        "id",
        "impact",
        "category",
        "location",
        "trigger",
        "consequence",
        "correction",
    ] {
        fields.insert(name.into(), string.clone());
    }
    json!({"type":"object","additionalProperties":false,
    "required":["snapshot","verdict","findings","evidence_gaps"],
    "properties":{
        "snapshot":{"type":"object","additionalProperties":false,"required":["base","head","requirements"],
            "properties":{"base":string,"head":string,"requirements":string}},
        "verdict":{"type":"string","enum":["PASS","CHANGES_REQUIRED","BLOCKED"]},
        "findings":{"type":"array","items":{"type":"object","additionalProperties":false,
            "required":["id","impact","category","location","trigger","consequence","correction"],"properties":fields}},
        "evidence_gaps":{"type":"array","items":{"type":"string","minLength":1}}
    }})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    snapshot: Snapshot,
    review: ReviewOutput,
}

pub fn validate(value: Value) -> Result<Value> {
    let request: Request = serde_json::from_value(value)?;
    ensure!(
        request.review.snapshot == request.snapshot,
        "review snapshot mismatch"
    );
    check(&request.review)?;
    Ok(json!({"valid":true,"snapshot":request.snapshot,"verdict":request.review.verdict}))
}

pub fn check(review: &ReviewOutput) -> Result<()> {
    for value in [
        review.snapshot.base.as_ref(),
        review.snapshot.head.as_ref(),
        review.snapshot.requirements.as_str(),
    ] {
        required_text(value, "snapshot identity")?;
    }
    let mut ids = std::collections::BTreeSet::new();
    for finding in &review.findings {
        ensure!(ids.insert(&finding.id), "duplicate finding id");
        for value in [
            &finding.id,
            &finding.impact,
            &finding.category,
            &finding.location,
            &finding.trigger,
            &finding.consequence,
            &finding.correction,
        ] {
            required_text(value, "finding field")?;
        }
    }
    for gap in &review.evidence_gaps {
        required_text(gap, "evidence gap")?;
    }
    ensure!(
        review.verdict != Verdict::Pass
            || (review.findings.is_empty() && review.evidence_gaps.is_empty()),
        "PASS cannot include findings or evidence gaps"
    );
    Ok(())
}
