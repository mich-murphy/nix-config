use crate::{
    Instant,
    acceptance::{Proof, Snapshot},
    command::AgentRole,
    ids::{AuthorityId, CriterionId, DeliveryId, Digest, LaunchId, OperationId, PrNumber, Sha},
    review::{Review, Verdict},
    task::PlannedWork,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryKind {
    Code { pr: Option<PullRequest> },
    Verification { of: Sha },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub id: DeliveryId,
    pub kind: DeliveryKind,
    pub authority: Option<AuthorityId>,
    pub criteria: Vec<CriterionId>,
    pub paths: Option<Vec<String>>,
    pub base: Sha,
    pub work: Option<PlannedWork>,
    pub proof: Proof,
    pub review: Option<Review>,
    /// The last settled review this delivery's snapshot (or new proof)
    /// superseded, kept so a repair review can concentrate on what
    /// changed since it. `default` so older projections still load.
    #[serde(default)]
    pub prior_review: Option<Review>,
    /// A human decision for the exact snapshot, in human-review mode.
    /// Cleared whenever this delivery's snapshot changes, so a stale
    /// receipt can never gate a merge it was never taken against.
    pub human_review: Option<HumanReceipt>,
    /// The `poll-checks --wait` deadline recorded per PR head, so a hold
    /// and resume never restart the wait.
    pub check_deadlines: BTreeMap<Sha, Instant>,
    pub outcome: Outcome,
    pub launches: Vec<LaunchId>,
    pub operations: Vec<OperationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HumanReceipt {
    pub snapshot: Snapshot,
    pub receipt: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Open,
    Merged { commit: Sha, at: Instant },
    Replaced { by: Replacement, at: Instant },
    Accepted { at: Instant },
    Abandoned { reason: String, at: Instant },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub pr: PrNumber,
    pub head: Sha,
    pub merge: Sha,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PullRequest {
    pub number: PrNumber,
    pub state: PrState,
    pub draft: bool,
    pub head: Sha,
    pub merge: Option<Sha>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub name: String,
    pub state: CheckState,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CheckState {
    Pending,
    Pass,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryError {
    PriorOpen,
    NotCode,
    AlreadyMerged,
    DuplicateCriteria,
    OutsidePaths,
    Incomplete,
}

#[must_use]
pub fn closed(delivery: &Delivery) -> bool {
    !matches!(delivery.outcome, Outcome::Open)
}

#[must_use]
pub fn accepts(delivery: &Delivery, full: &[CriterionId]) -> bool {
    acceptance_ready(delivery, full)
        && matches!(
            delivery.outcome,
            Outcome::Merged { .. } | Outcome::Accepted { .. }
        )
}

#[must_use]
pub fn acceptance_ready(delivery: &Delivery, full: &[CriterionId]) -> bool {
    delivery.criteria == full
        && delivery
            .review
            .as_ref()
            .is_some_and(|review| review.verdict == Verdict::Pass)
}

#[must_use]
pub fn admits(delivery: &Delivery, role: AgentRole) -> bool {
    role != AgentRole::Implementer || !matches!(delivery.kind, DeliveryKind::Verification { .. })
}

pub fn can_open(history: &[Delivery]) -> Result<(), DeliveryError> {
    if history.last().is_some_and(|delivery| !closed(delivery)) {
        Err(DeliveryError::PriorOpen)
    } else {
        Ok(())
    }
}

pub fn can_narrow(delivery: &Delivery, criteria: &[CriterionId]) -> Result<(), DeliveryError> {
    if !matches!(delivery.kind, DeliveryKind::Code { .. }) {
        return Err(DeliveryError::NotCode);
    }
    if !matches!(delivery.outcome, Outcome::Open) {
        return Err(DeliveryError::AlreadyMerged);
    }
    let mut unique = criteria.to_vec();
    unique.sort();
    unique.dedup();
    if unique.len() != criteria.len() || criteria.is_empty() {
        return Err(DeliveryError::DuplicateCriteria);
    }
    Ok(())
}

pub fn paths_allow(delivery: &Delivery, commits: &[Vec<String>]) -> Result<(), DeliveryError> {
    let Some(allowed) = &delivery.paths else {
        return Ok(());
    };
    let outside = commits
        .iter()
        .flatten()
        .any(|path| !allowed.iter().any(|pattern| glob(pattern, path)));
    if outside {
        Err(DeliveryError::OutsidePaths)
    } else {
        Ok(())
    }
}

/// The one glob matcher for both delivery path scopes and sensitive-path
/// risk signals: an exact path, an anchored `prefix/**` (every path under
/// `prefix/`), or an anchored `**/suffix` (`suffix` itself, or anything
/// ending `/suffix`). Anchored so `auth/**` cannot match `authx/file` and
/// `**/schema.json` cannot match `myschema.json`.
#[must_use]
pub fn glob(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'));
    }
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return path
            .strip_suffix(suffix)
            .is_some_and(|rest| rest.is_empty() || rest.ends_with('/'));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn paths_bound_every_commit() -> Result<(), crate::ids::InvalidId> {
        let delivery = Delivery {
            id: DeliveryId(1),
            kind: DeliveryKind::Code { pr: None },
            authority: None,
            criteria: Vec::new(),
            paths: Some(vec!["docs/**".into()]),
            base: Sha::from_str(&"a".repeat(40))?,
            work: None,
            proof: Proof::default(),
            review: None,
            human_review: None,
            check_deadlines: BTreeMap::new(),
            prior_review: None,
            outcome: Outcome::Open,
            launches: Vec::new(),
            operations: Vec::new(),
        };
        let commits = vec![vec!["src/lib.rs".into()], vec!["docs/guide.md".into()]];
        assert_eq!(
            paths_allow(&delivery, &commits),
            Err(DeliveryError::OutsidePaths)
        );
        Ok(())
    }

    #[test]
    fn glob_is_anchored() {
        assert!(glob("auth/**", "auth/session.rs"));
        assert!(!glob("auth/**", "authx/file.rs"));
        assert!(glob("**/schema.json", "crates/domain/schema.json"));
        assert!(!glob("**/schema.json", "myschema.json"));
        assert!(glob("Cargo.toml", "Cargo.toml"));
    }
}
