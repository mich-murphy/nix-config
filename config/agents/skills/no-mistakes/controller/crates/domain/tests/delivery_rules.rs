mod common;

use common::*;
use domain::{
    delivery::{self, DeliveryError, DeliveryKind, Outcome},
    event::Event,
    state::{State, apply},
};

#[test]
fn open_requires_closed_prior() {
    assert_eq!(
        delivery::can_open(&[delivery()]),
        Err(DeliveryError::PriorOpen)
    );
    let mut prior = delivery();
    prior.outcome = Outcome::Merged {
        commit: sha('c'),
        at: 1,
    };
    assert!(delivery::can_open(&[prior]).is_ok());
}

#[test]
fn verified_requires_full_delivery() {
    let mut value = delivery();
    value.review = Some(review(domain::review::Verdict::Pass));
    value.outcome = Outcome::Merged {
        commit: sha('c'),
        at: 1,
    };
    assert!(delivery::accepts(&value, &[criterion_id()]));
    assert!(!delivery::accepts(
        &value,
        &[
            criterion_id(),
            "AC2"
                .parse()
                .unwrap_or_else(|error| panic!("fixture: {error}"))
        ]
    ));
}

#[test]
fn narrowing_requires_open_code_delivery() {
    let mut value = delivery();
    value.kind = DeliveryKind::Verification { of: sha('a') };
    assert_eq!(
        delivery::can_narrow(&value, &[criterion_id()]),
        Err(DeliveryError::NotCode)
    );
    value.kind = DeliveryKind::Code { pr: None };
    value.outcome = Outcome::Merged {
        commit: sha('c'),
        at: 1,
    };
    assert_eq!(
        delivery::can_narrow(&value, &[criterion_id()]),
        Err(DeliveryError::AlreadyMerged)
    );
}

#[test]
fn narrowing_invalidates_proof() {
    let mut task = task();
    task.deliveries[0]
        .proof
        .entries
        .insert(criterion_id(), proof_entry());
    task.deliveries[0].review = Some(review(domain::review::Verdict::Pass));
    let mut state = State::empty();
    state.tasks.insert(task.id.clone(), task);
    apply(
        &mut state,
        &Event::AcceptanceNarrowed {
            task: task_id(),
            delivery: domain::ids::DeliveryId(1),
            criteria: vec![
                "STAGE1"
                    .parse()
                    .unwrap_or_else(|error| panic!("fixture: {error}")),
            ],
            authority: domain::ids::AuthorityId(1),
        },
    );
    let current = &state.tasks[&task_id()].deliveries[0];
    assert!(current.proof.entries.is_empty());
    assert!(current.review.is_none());
}

#[test]
fn narrowed_merge_keeps_acceptance_open() {
    let mut value = delivery();
    value.criteria = vec![
        "STAGE1"
            .parse()
            .unwrap_or_else(|error| panic!("fixture: {error}")),
    ];
    value.review = Some(review(domain::review::Verdict::Pass));
    value.outcome = Outcome::Merged {
        commit: sha('c'),
        at: 1,
    };
    assert!(!delivery::accepts(&value, &[criterion_id()]));
}

#[test]
fn verification_rejects_implementer() {
    let mut value = delivery();
    value.kind = DeliveryKind::Verification { of: sha('a') };
    assert!(!delivery::admits(
        &value,
        domain::command::AgentRole::Implementer
    ));
    assert!(delivery::admits(
        &value,
        domain::command::AgentRole::Reviewer
    ));
}

#[test]
fn closed_delivery_is_immutable() {
    let mut value = delivery();
    value.outcome = Outcome::Merged {
        commit: sha('c'),
        at: 1,
    };
    assert!(delivery::closed(&value));
    assert_eq!(delivery::can_open(&[value]), Ok(()));
}
