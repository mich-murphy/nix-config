#![allow(dead_code)]

use domain::{
    acceptance::{Measurement, Proof, ProofEntry, ProofKind, ProofStatus, Snapshot},
    authority::{Authority, AuthorityUse, Grant},
    budget::Budgets,
    delivery::{Delivery, DeliveryKind, Outcome},
    ids::{AuthorityId, CriterionId, Digest, LaunchId, Sha, TaskId},
    review::{Category, Finding, Review, Severity, Verdict},
    risk::{Tier, TierState},
    sync::Sync,
    task::{Criterion, Phase, Task, TaskSpec},
};
use std::{collections::BTreeMap, fmt::Debug, path::PathBuf, str::FromStr};

fn parse<T>(value: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    match value.parse() {
        Ok(parsed) => parsed,
        Err(error) => panic!("invalid fixture: {error:?}"),
    }
}

pub fn task_id() -> TaskId {
    parse("GAIN-1")
}

pub fn criterion_id() -> CriterionId {
    parse("AC1")
}

pub fn digest(seed: char) -> Digest {
    parse(&seed.to_string().repeat(64))
}

pub fn sha(seed: char) -> Sha {
    parse(&seed.to_string().repeat(40))
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        base: sha('a'),
        head: sha('b'),
        requirements: digest('c'),
    }
}

pub fn criterion() -> Criterion {
    Criterion {
        id: criterion_id(),
        text: "observable result".into(),
        human_only: false,
    }
}

pub fn proof_entry() -> ProofEntry {
    ProofEntry {
        criterion: criterion_id(),
        status: ProofStatus::Passed,
        kind: ProofKind::Command,
        artifact: PathBuf::from("/proof"),
        digest: digest('d'),
        snapshot: snapshot(),
        measurement: Some(Measurement {
            baseline: digest('e'),
            observed: "changed from baseline".into(),
            satisfied: true,
        }),
        implementation: vec!["src/lib.rs:1".into()],
        description: "real boundary".into(),
    }
}

pub fn finding(severity: Severity) -> Finding {
    Finding {
        id: parse("F1"),
        severity,
        category: Category::Correctness,
        location: "src/lib.rs:1".into(),
        trigger: "input".into(),
        consequence: "wrong result".into(),
        correction: "correct result".into(),
    }
}

pub fn review(verdict: Verdict) -> Review {
    Review {
        launch: LaunchId(1),
        session: "reviewer".into(),
        snapshot: snapshot(),
        findings: Vec::new(),
        evidence_gaps: Vec::new(),
        reviewer_opinion: verdict,
        verdict,
        dispositions: BTreeMap::new(),
    }
}

pub fn authority() -> Authority {
    Authority {
        id: AuthorityId(1),
        source: "current user".into(),
        artifact: PathBuf::from("/receipt"),
        digest: digest('f'),
        requirements: digest('c'),
        recorded: 1,
        grant: Grant::Pair { paths: None },
        used: AuthorityUse::default(),
    }
}

pub fn delivery() -> Delivery {
    Delivery {
        id: domain::ids::DeliveryId(1),
        kind: DeliveryKind::Code { pr: None },
        authority: None,
        criteria: vec![criterion_id()],
        paths: None,
        base: sha('a'),
        work: None,
        proof: Proof::default(),
        review: None,
        human_review: None,
        check_deadlines: BTreeMap::new(),
        outcome: Outcome::Open,
        launches: Vec::new(),
        operations: Vec::new(),
    }
}

pub fn task() -> Task {
    Task {
        id: task_id(),
        spec: TaskSpec {
            parent: parse("GAIN-0"),
            criteria: vec![criterion()],
            dependencies: Vec::new(),
            priority: 1,
            due: None,
            rank: "a".into(),
            not_before: None,
            member: true,
            ownership_clear: true,
            ownership_evidence: "fresh reads".into(),
            was_terminal: false,
            requirements: digest('c'),
        },
        phase: Phase::Claimed,
        hold: None,
        sync: Sync::Pending,
        budgets: Budgets::default(),
        deliveries: vec![delivery()],
        authorities: Vec::new(),
        tier: TierState {
            current: Tier::Lite,
            provisional: true,
            history: Vec::new(),
        },
        subtasks: BTreeMap::new(),
        baselines: BTreeMap::new(),
    }
}
