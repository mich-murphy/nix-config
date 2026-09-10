//! The run's quality schema as Prefactor declares it: one named schema on
//! the agent schema version, derived from `domain::judge::QualityPayload`,
//! with a Liquid template that renders the rollup as one line.

use serde_json::{Value, json};

/// The quality schema name; `record_quality` keys payloads by it.
pub(super) const NAME: &str = "delivery-quality";

const TEMPLATE: &str = "{{rollup.verdict}} {{rollup.mean_overall}}/100 across \
{{rollup.judged}} of {{rollup.tasks}} task(s): {{rollup.pass}} pass, \
{{rollup.degraded}} degraded, {{rollup.fail}} fail; friction \
{{rollup.mean_friction}}/100 (100 = none), {{rollup.friction_events}} event(s), \
{{rollup.avoidable_friction_events}} avoidable";

/// The `quality_schemas` array for `agent_instance/register`. Prefactor
/// takes the schema as written, `$defs` included, but not the draft
/// `$schema` declaration, so that key is dropped as it is for spans.
pub(super) fn schemas() -> Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(domain::judge::QualityPayload))
        .unwrap_or_else(|_| json!({"type": "object", "additionalProperties": true}));
    if let Some(object) = schema.as_object_mut() {
        object.remove("$schema");
    }
    json!([{
        "name": NAME,
        "title": "Delivery quality",
        "description": "LLM-as-judge verdicts per task with a run rollup",
        "schema": schema,
        "template": TEMPLATE,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_named_and_stripped_of_the_draft_declaration() {
        let value = schemas();
        assert_eq!(value[0]["name"], NAME);
        assert!(value[0]["schema"].get("$schema").is_none());
        assert!(value[0]["schema"]["properties"]["rollup"].is_object());
        assert!(
            value[0]["template"]
                .as_str()
                .is_some_and(|t| t.contains("rollup.verdict"))
        );
    }
}
