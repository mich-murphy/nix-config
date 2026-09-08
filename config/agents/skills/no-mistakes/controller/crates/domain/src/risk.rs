use crate::{Instant, ids::ModelId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Trivial,
    Lite,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierState {
    pub current: Tier,
    pub provisional: bool,
    pub history: Vec<TierChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierChange {
    pub to: Tier,
    pub reason: String,
    pub at: Instant,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signals {
    pub lines: u32,
    pub files: u32,
    pub sensitive_paths: u32,
    pub manifest_or_lockfile: bool,
    pub human_only_criteria: u32,
    pub dependencies: u32,
}

pub fn classify(signals: &Signals) -> Tier {
    let forced = signals.sensitive_paths > 0
        || signals.manifest_or_lockfile
        || signals.human_only_criteria > 0
        || signals.dependencies > 2
        || signals.lines > 100
        || signals.files > 20;
    if forced {
        Tier::Full
    } else if signals.lines <= 10 && signals.files <= 5 {
        Tier::Trivial
    } else {
        Tier::Lite
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub model: ModelId,
    pub effort: Effort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub rank: u32,
    #[serde(default)]
    pub fallback_for: Vec<ModelId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub harness: HarnessConfig,
    pub models: BTreeMap<ModelId, Model>,
    pub coordinator: Assignment,
    pub tier: TierProfiles,
    pub escalation: Escalation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessConfig {
    pub kind: HarnessKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierProfiles {
    pub trivial: Roles,
    pub lite: Roles,
    pub full: Roles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escalation {
    pub reviewer: Assignment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roles {
    pub implementer: Assignment,
    pub reviewer: Assignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessKind {
    Codex,
    Pi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    UnknownModel,
    WeakReviewer,
    WeakEscalation,
}

pub fn validate(profile: &Profile) -> Result<(), ProfileError> {
    let roles = [
        &profile.tier.trivial,
        &profile.tier.lite,
        &profile.tier.full,
    ];
    for role in roles {
        let implementer = rank(profile, &role.implementer.model)?;
        let reviewer = rank(profile, &role.reviewer.model)?;
        if reviewer < implementer {
            return Err(ProfileError::WeakReviewer);
        }
    }
    rank(profile, &profile.coordinator.model)?;
    let escalation = rank(profile, &profile.escalation.reviewer.model)?;
    let full = rank(profile, &profile.tier.full.reviewer.model)?;
    if escalation <= full {
        return Err(ProfileError::WeakEscalation);
    }
    Ok(())
}

fn rank(profile: &Profile, model: &ModelId) -> Result<u32, ProfileError> {
    profile
        .models
        .get(model)
        .map(|entry| entry.rank)
        .ok_or(ProfileError::UnknownModel)
}

pub fn raise(current: Tier, proposed: Tier) -> Tier {
    current.max(proposed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_never_lowers() {
        assert_eq!(raise(Tier::Full, Tier::Trivial), Tier::Full);
        assert_eq!(raise(Tier::Trivial, Tier::Lite), Tier::Lite);
    }

    #[test]
    fn sensitive_change_is_full() {
        assert_eq!(
            classify(&Signals {
                sensitive_paths: 1,
                ..Signals::default()
            }),
            Tier::Full
        );
    }
}
