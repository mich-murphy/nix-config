mod common;

use common::*;
use domain::{
    authority::{self, Grant},
    budget::{self, BudgetKind, Budgets},
    delivery::DeliveryKind,
    ids::LaunchId,
    risk::Tier,
};

#[test]
fn cancelled_review_counts() {
    let mut budgets = Budgets::default();
    let limit = budget::Limits::for_tier(Tier::Trivial, None).reviews;
    budget::spend(&mut budgets, BudgetKind::Review, true);
    assert_eq!(
        budget::remaining(BudgetKind::Review, &budgets, Tier::Trivial, &[], None),
        limit - 1
    );
}

#[test]
fn continued_turns_use_one_repair() {
    let mut budgets = Budgets::default();
    budget::spend(&mut budgets, BudgetKind::Repair, true);
    budget::spend(&mut budgets, BudgetKind::Implementation, true);
    assert_eq!(budgets.repairs, 1);
    assert_eq!(budgets.implementation_turns, 1);
}

#[test]
fn review_budget_spans_deliveries() {
    let budgets = Budgets {
        reviews: 2,
        ..Budgets::default()
    };
    assert_eq!(
        budget::remaining(BudgetKind::Review, &budgets, Tier::Lite, &[], None),
        1
    );
}

#[test]
fn pair_grants_one_of_each() {
    let mut grant = authority();
    assert!(authority::spend_pair(&mut grant, true, LaunchId(1)).is_ok());
    assert!(authority::spend_pair(&mut grant, false, LaunchId(2)).is_ok());
    assert!(!authority::pair_available(&grant, true));
    assert!(!authority::pair_available(&grant, false));
}

#[test]
fn failed_pair_is_spent() {
    let mut grant = authority();
    assert!(authority::spend_pair(&mut grant, true, LaunchId(1)).is_ok());
    assert!(!authority::pair_available(&grant, true));
}

#[test]
fn verification_pair_spends_review_only() {
    let mut grant = authority();
    grant.grant = Grant::Delivery {
        kind: DeliveryKind::Verification { of: sha('a') },
        criteria: vec![criterion_id()],
        paths: None,
    };
    assert!(authority::spend_pair(&mut grant, false, LaunchId(1)).is_ok());
    assert!(grant.used.implementation.is_none());
    assert_eq!(grant.used.review, Some(LaunchId(1)));
}
