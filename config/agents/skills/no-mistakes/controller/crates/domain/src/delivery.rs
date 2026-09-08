use crate::{
    Instant,
    acceptance::Proof,
    ids::{AuthorityId, CriterionId, DeliveryId, LaunchId, OperationId, PrNumber, Sha},
    review::{Review, Verdict},
    task::PlannedWork,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryKind {
    Code { pr: Option<PullRequest> },
    Verification { of: Sha },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub outcome: Outcome,
    pub launches: Vec<LaunchId>,
    pub operations: Vec<OperationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Open,
    Merged { commit: Sha, at: Instant },
    Replaced { by: Replacement, at: Instant },
    Accepted { at: Instant },
    Abandoned { reason: String, at: Instant },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub pr: PrNumber,
    pub head: Sha,
    pub merge: Sha,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: PrNumber,
    pub state: PrState,
    pub draft: bool,
    pub head: Sha,
    pub merge: Option<Sha>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub state: CheckState,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckState {
    Pending,
    Pass,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        .any(|path| !allowed.iter().any(|pattern| path_matches(pattern, path)));
    if outside {
        Err(DeliveryError::OutsidePaths)
    } else {
        Ok(())
    }
}

fn path_matches(pattern: &str, path: &str) -> bool {
    pattern == path
        || pattern
            .strip_suffix("/**")
            .is_some_and(|prefix| path.starts_with(&format!("{prefix}/")))
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
}
