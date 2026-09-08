mod common;

use common::*;
use domain::{
    acceptance::{self, AcceptanceError, Proof},
    event::Event,
    state::{State, apply},
};
use std::collections::BTreeMap;

fn complete(
    entry: domain::acceptance::ProofEntry,
) -> Result<(), domain::acceptance::AcceptanceError> {
    let mut proof = Proof::default();
    proof.entries.insert(entry.criterion.clone(), entry.clone());
    let artifacts = BTreeMap::from([(entry.artifact.clone(), entry.digest.clone())]);
    let baseline = entry
        .measurement
        .as_ref()
        .map(|value| value.baseline.clone())
        .unwrap_or_else(|| digest('f'));
    let baselines = BTreeMap::from([(entry.criterion.clone(), baseline)]);
    acceptance::complete(&[criterion()], &proof, &snapshot(), &artifacts, &baselines)
}

#[test]
fn pass_requires_all_criteria() {
    let result = acceptance::complete(
        &[criterion()],
        &Proof::default(),
        &snapshot(),
        &BTreeMap::new(),
        &BTreeMap::new(),
    );
    assert_eq!(result, Err(AcceptanceError::Missing));
}

#[test]
fn changed_artifact_fails_acceptance() {
    let entry = proof_entry();
    let mut proof = Proof::default();
    proof.entries.insert(entry.criterion.clone(), entry.clone());
    let artifacts = BTreeMap::from([(entry.artifact.clone(), digest('f'))]);
    let baselines = BTreeMap::from([(entry.criterion.clone(), digest('e'))]);
    assert_eq!(
        acceptance::complete(&[criterion()], &proof, &snapshot(), &artifacts, &baselines),
        Err(AcceptanceError::ArtifactChanged)
    );
}

#[test]
fn changed_baseline_fails_acceptance() {
    let entry = proof_entry();
    let mut proof = Proof::default();
    proof.entries.insert(entry.criterion.clone(), entry.clone());
    let artifacts = BTreeMap::from([(entry.artifact.clone(), entry.digest.clone())]);
    let baselines = BTreeMap::from([(entry.criterion.clone(), digest('f'))]);
    assert_eq!(
        acceptance::complete(&[criterion()], &proof, &snapshot(), &artifacts, &baselines),
        Err(AcceptanceError::BaselineChanged)
    );
}

#[test]
fn proof_batch_is_atomic() {
    let bad = proof_entry();
    let before = Proof::default();
    assert!(acceptance::validate_batch(&[bad.clone(), bad], &[criterion()]).is_err());
    assert!(before.entries.is_empty());
}

#[test]
fn new_proof_invalidates_review() {
    let mut task = task();
    task.deliveries[0].review = Some(review(domain::review::Verdict::Pass));
    let mut state = State::empty();
    state.tasks.insert(task.id.clone(), task);
    apply(
        &mut state,
        &Event::ProofRecorded {
            task: task_id(),
            delivery: domain::ids::DeliveryId(1),
            entries: vec![proof_entry()],
        },
    );
    assert!(state.tasks[&task_id()].deliveries[0].review.is_none());
}

#[test]
fn proof_invalidated_clears_entries() {
    let mut task = task();
    task.deliveries[0]
        .proof
        .entries
        .insert(criterion_id(), proof_entry());
    let mut state = State::empty();
    state.tasks.insert(task.id.clone(), task);
    apply(
        &mut state,
        &Event::ProofInvalidated {
            task: task_id(),
            delivery: domain::ids::DeliveryId(1),
            cause: "head changed".into(),
        },
    );
    assert!(
        state.tasks[&task_id()].deliveries[0]
            .proof
            .entries
            .is_empty()
    );
}

#[test]
fn passing_entry_completes_acceptance() {
    assert!(complete(proof_entry()).is_ok());
}
