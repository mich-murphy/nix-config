use domain::{
    ids::ModelId,
    risk::{
        Assignment, Effort, Escalation, HarnessConfig, HarnessKind, Model, Profile, Roles,
        TierProfiles,
    },
};
use std::{collections::BTreeMap, str::FromStr};

pub fn profile() -> Result<Profile, domain::ids::InvalidId> {
    let terra = ModelId::from_str("openai/terra")?;
    let sol = ModelId::from_str("openai/sol")?;
    let astra = ModelId::from_str("openai/astra")?;
    let mut models = BTreeMap::new();
    models.insert(
        terra.clone(),
        Model {
            rank: 1,
            fallback_for: Vec::new(),
        },
    );
    models.insert(
        sol.clone(),
        Model {
            rank: 2,
            fallback_for: vec![terra.clone()],
        },
    );
    models.insert(
        astra.clone(),
        Model {
            rank: 3,
            fallback_for: Vec::new(),
        },
    );
    let implementer = Assignment {
        model: terra,
        effort: Effort::Medium,
    };
    let reviewer = Assignment {
        model: sol.clone(),
        effort: Effort::Medium,
    };
    Ok(Profile {
        harness: HarnessConfig {
            kind: HarnessKind::Pi,
        },
        models,
        coordinator: reviewer.clone(),
        tier: TierProfiles {
            trivial: Roles {
                implementer: implementer.clone(),
                reviewer: reviewer.clone(),
            },
            lite: Roles {
                implementer: reviewer.clone(),
                reviewer: reviewer.clone(),
            },
            full: Roles {
                implementer: reviewer.clone(),
                reviewer,
            },
        },
        escalation: Escalation {
            reviewer: Assignment {
                model: astra,
                effort: Effort::High,
            },
        },
    })
}
