mod common;

use common::*;
use domain::{
    ids::FindingId,
    review::{self, Disposition, DispositionDecision, ReviewError, Severity, Verdict},
    risk::Tier,
};
use std::{collections::BTreeMap, str::FromStr};

#[test]
fn review_requires_same_snapshot() {
    let mut value = review(Verdict::Pass);
    value.snapshot.head = sha('f');
    assert_eq!(
        review::validate(&value, &snapshot(), None),
        Err(ReviewError::StaleSnapshot)
    );
}

#[test]
fn reviewer_session_is_separate() {
    let value = review(Verdict::Pass);
    assert_eq!(
        review::validate(&value, &snapshot(), Some("reviewer")),
        Err(ReviewError::ReusedSession)
    );
}

#[test]
fn pass_rejects_evidence_gaps() {
    assert_eq!(
        review::verdict(&[], &["missing proof".into()], Tier::Lite),
        Verdict::Blocked
    );
}

#[test]
fn merge_requires_dispositions() {
    let mut value = review(Verdict::Pass);
    value.findings.push(finding(Severity::Warning));
    assert_eq!(
        review::merge_ready(&value),
        Err(ReviewError::MissingDisposition)
    );
    value.dispositions = BTreeMap::from([(
        FindingId::from_str("F1").unwrap_or_else(|error| panic!("fixture: {error}")),
        Disposition {
            decision: DispositionDecision::Accepted,
            evidence: "tracked correction".into(),
        },
    )]);
    assert!(review::merge_ready(&value).is_ok());
}

#[test]
fn warning_pattern_blocks_by_tier() {
    let mut first = finding(Severity::Warning);
    first.id = FindingId::from_str("F1").unwrap_or_else(|error| panic!("fixture: {error}"));
    let mut second = first.clone();
    second.id = FindingId::from_str("F2").unwrap_or_else(|error| panic!("fixture: {error}"));
    assert_eq!(
        review::verdict(&[first, second], &[], Tier::Full),
        Verdict::ChangesRequired
    );
}
