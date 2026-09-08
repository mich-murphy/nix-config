use crate::{
    acceptance::Snapshot,
    ids::{FindingId, LaunchId},
    risk::Tier,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub launch: LaunchId,
    pub session: String,
    pub snapshot: Snapshot,
    pub findings: Vec<Finding>,
    pub evidence_gaps: Vec<String>,
    pub reviewer_opinion: Verdict,
    pub verdict: Verdict,
    pub dispositions: BTreeMap<FindingId, Disposition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: FindingId,
    pub severity: Severity,
    pub category: Category,
    pub location: String,
    pub trigger: String,
    pub consequence: String,
    pub correction: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Suggestion,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Correctness,
    Security,
    Reliability,
    Maintainability,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    ChangesRequired,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disposition {
    pub decision: DispositionDecision,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DispositionDecision {
    Accepted,
    Declined,
    Corrected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewError {
    DuplicateFinding,
    StaleSnapshot,
    ReusedSession,
    MissingDisposition,
}

#[must_use]
pub fn verdict(findings: &[Finding], gaps: &[String], tier: Tier) -> Verdict {
    if !gaps.is_empty() {
        return Verdict::Blocked;
    }
    if findings
        .iter()
        .any(|finding| finding.severity == Severity::Critical)
    {
        return Verdict::ChangesRequired;
    }
    warning_verdict(findings, tier)
}

pub fn validate(
    review: &Review,
    snapshot: &Snapshot,
    implementer_session: Option<&str>,
) -> Result<(), ReviewError> {
    let unique: BTreeSet<_> = review.findings.iter().map(|finding| &finding.id).collect();
    if unique.len() != review.findings.len() {
        return Err(ReviewError::DuplicateFinding);
    }
    if &review.snapshot != snapshot {
        return Err(ReviewError::StaleSnapshot);
    }
    if implementer_session == Some(review.session.as_str()) {
        return Err(ReviewError::ReusedSession);
    }
    Ok(())
}

pub fn merge_ready(review: &Review) -> Result<(), ReviewError> {
    if review
        .findings
        .iter()
        .any(|finding| !review.dispositions.contains_key(&finding.id))
    {
        return Err(ReviewError::MissingDisposition);
    }
    Ok(())
}

fn warning_verdict(findings: &[Finding], tier: Tier) -> Verdict {
    let warnings: Vec<_> = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Warning)
        .collect();
    let (category_limit, total_limit) = match tier {
        Tier::Full => (2, 4),
        Tier::Trivial | Tier::Lite => (3, 5),
    };
    let mut categories = BTreeMap::new();
    for finding in &warnings {
        *categories.entry(&finding.category).or_insert(0_usize) += 1;
    }
    if warnings.len() >= total_limit || categories.values().any(|count| *count >= category_limit) {
        Verdict::ChangesRequired
    } else {
        Verdict::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn finding(severity: Severity) -> Result<Finding, crate::ids::InvalidId> {
        Ok(Finding {
            id: FindingId::from_str("F1")?,
            severity,
            category: Category::Correctness,
            location: "src/lib.rs:1".into(),
            trigger: "input".into(),
            consequence: "failure".into(),
            correction: "fix".into(),
        })
    }

    #[test]
    fn critical_finding_blocks() -> Result<(), crate::ids::InvalidId> {
        assert_eq!(
            verdict(&[finding(Severity::Critical)?], &[], Tier::Lite),
            Verdict::ChangesRequired
        );
        Ok(())
    }

    #[test]
    fn verdict_ignores_reviewer_claim() -> Result<(), crate::ids::InvalidId> {
        let findings = [finding(Severity::Critical)?];
        let computed = verdict(&findings, &[], Tier::Lite);
        assert_ne!(computed, Verdict::Pass);
        Ok(())
    }
}
