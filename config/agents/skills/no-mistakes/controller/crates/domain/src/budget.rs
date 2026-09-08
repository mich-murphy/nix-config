use crate::{
    authority::{Authority, Grant},
    risk::Tier,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BudgetKind {
    Implementation,
    Stalled,
    Review,
    Repair,
    CiRepair,
    Escalation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budgets {
    pub implementation_turns: u32,
    pub stalled_checkpoints: u32,
    pub reviews: u32,
    pub repairs: u32,
    pub ci_repairs: u32,
    pub escalations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub implementation: u32,
    pub stalled: u32,
    pub reviews: u32,
    pub repairs: u32,
    pub ci_repairs: u32,
    pub escalations: u32,
}

impl Limits {
    #[must_use]
    pub fn for_tier(tier: Tier) -> Self {
        match tier {
            Tier::Trivial => Self::new(4, 2, 1, 1, 1, 0),
            Tier::Lite => Self::new(8, 2, 3, 2, 1, 1),
            Tier::Full => Self::new(12, 2, 5, 4, 1, 1),
        }
    }

    const fn new(
        implementation: u32,
        stalled: u32,
        reviews: u32,
        repairs: u32,
        ci_repairs: u32,
        escalations: u32,
    ) -> Self {
        Self {
            implementation,
            stalled,
            reviews,
            repairs,
            ci_repairs,
            escalations,
        }
    }
}

#[must_use]
pub fn remaining(
    kind: BudgetKind,
    budgets: &Budgets,
    tier: Tier,
    authorities: &[Authority],
) -> u32 {
    let base = limit(Limits::for_tier(tier), kind);
    let extra = authorities
        .iter()
        .filter(|entry| grants_counted_pair(entry, kind))
        .count() as u32;
    base.saturating_add(extra)
        .saturating_sub(spent(budgets, kind))
}

pub fn spend(budgets: &mut Budgets, kind: BudgetKind, counted: bool) {
    if !counted {
        return;
    }
    match kind {
        BudgetKind::Implementation => budgets.implementation_turns += 1,
        BudgetKind::Stalled => budgets.stalled_checkpoints += 1,
        BudgetKind::Review => budgets.reviews += 1,
        BudgetKind::Repair => budgets.repairs += 1,
        BudgetKind::CiRepair => budgets.ci_repairs += 1,
        BudgetKind::Escalation => budgets.escalations += 1,
    }
}

const fn limit(limits: Limits, kind: BudgetKind) -> u32 {
    match kind {
        BudgetKind::Implementation => limits.implementation,
        BudgetKind::Stalled => limits.stalled,
        BudgetKind::Review => limits.reviews,
        BudgetKind::Repair => limits.repairs,
        BudgetKind::CiRepair => limits.ci_repairs,
        BudgetKind::Escalation => limits.escalations,
    }
}

const fn spent(budgets: &Budgets, kind: BudgetKind) -> u32 {
    match kind {
        BudgetKind::Implementation => budgets.implementation_turns,
        BudgetKind::Stalled => budgets.stalled_checkpoints,
        BudgetKind::Review => budgets.reviews,
        BudgetKind::Repair => budgets.repairs,
        BudgetKind::CiRepair => budgets.ci_repairs,
        BudgetKind::Escalation => budgets.escalations,
    }
}

fn grants_counted_pair(authority: &Authority, kind: BudgetKind) -> bool {
    let pair_kind = matches!(kind, BudgetKind::Implementation | BudgetKind::Review);
    let unscoped = match &authority.grant {
        Grant::Pair { paths } | Grant::Delivery { paths, .. } => paths.is_none(),
        Grant::Narrowing { .. } | Grant::SlotReuse { .. } => false,
    };
    pair_kind && unscoped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_cap_follows_tier() {
        let budgets = Budgets::default();
        assert_eq!(
            remaining(BudgetKind::Review, &budgets, Tier::Trivial, &[]),
            1
        );
        assert_eq!(remaining(BudgetKind::Review, &budgets, Tier::Lite, &[]), 3);
        assert_eq!(remaining(BudgetKind::Review, &budgets, Tier::Full, &[]), 5);
    }

    #[test]
    fn scoped_pair_is_uncounted() {
        let mut budgets = Budgets::default();
        spend(&mut budgets, BudgetKind::Review, false);
        assert_eq!(budgets.reviews, 0);
    }

    #[test]
    fn scoped_pair_keeps_exhaustion() {
        let budgets = Budgets {
            reviews: 1,
            ..Budgets::default()
        };
        assert_eq!(
            remaining(BudgetKind::Review, &budgets, Tier::Trivial, &[]),
            0
        );
    }
}
