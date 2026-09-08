use crate::App;
use crate::delivery::PublishInput;
use domain::{
    acceptance::{self, Snapshot},
    authority::{self, Authority, Grant},
    command::{JiraRead, PublishStep, ReviewMode},
    delivery::{CheckState, Delivery, DeliveryKind, Outcome, PrState},
    event::GitHubAction,
    ids::{DeliveryId, IssueKey, OperationId, PrNumber, TaskId},
    review::{self, Verdict},
    sync::Sync,
    task::{HoldReason, Phase},
};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, fs, str::FromStr};

pub(super) fn get_task<'a>(
    state: &'a domain::state::State,
    task: &TaskId,
) -> Result<&'a domain::task::Task, String> {
    state.tasks.get(task).ok_or_else(|| "unknown task".into())
}

pub(super) fn next_delivery(task: &domain::task::Task) -> DeliveryId {
    DeliveryId(
        task.deliveries
            .iter()
            .map(|delivery| delivery.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

pub(super) fn next_operation(state: &domain::state::State) -> OperationId {
    OperationId(
        state
            .operations
            .iter()
            .map(|operation| operation.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

pub(super) fn validate_authority_file(authority: &Authority) -> Result<(), String> {
    let bytes = fs::read(&authority.artifact).map_err(|error| error.to_string())?;
    let digest = domain::ids::Digest::from_str(&format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| error.to_string())?;
    authority::validate_receipt(authority, &digest)
        .map_err(|error| format!("receipt rejected: {error:?}"))
}

pub(super) fn validate_open(task: &domain::task::Task, jira: &JiraRead) -> Result<(), String> {
    if !matches!(
        task.hold.as_ref().map(|hold| &hold.reason),
        Some(HoldReason::NeedsHuman { .. })
    ) {
        return Err("open-delivery requires a needs-human hold".into());
    }
    if task.spec.was_terminal || matches!(task.phase, Phase::Verified { .. } | Phase::Excluded(_)) {
        return Err("terminal tasks cannot open another delivery".into());
    }
    if !jira.member || jira.resolved || !jira.ownership_clear {
        return Err("fresh Jira read does not establish unresolved run ownership".into());
    }
    if jira.requirements != task.spec.requirements {
        return Err("criteria changed".into());
    }
    Ok(())
}

pub(super) fn validate_delivery_grant(
    authority: &Authority,
    kind: &DeliveryKind,
) -> Result<(), String> {
    match &authority.grant {
        Grant::Delivery {
            kind: granted,
            criteria,
            ..
        } if std::mem::discriminant(granted) == std::mem::discriminant(kind)
            && !criteria.is_empty() =>
        {
            Ok(())
        }
        _ => Err("authority is not a matching delivery grant".into()),
    }
}

pub(super) fn validate_history(app: &App<'_>, task: &domain::task::Task) -> Result<(), String> {
    for delivery in &task.deliveries {
        match &delivery.outcome {
            Outcome::Merged { commit, .. }
                if !app.services.vcs.on_main(commit).map_err(|error| error.0)? =>
            {
                return Err("historical merge left main".into());
            }
            Outcome::Replaced { by, .. } => validate_replacement(app, by)?,
            _ => {}
        }
    }
    Ok(())
}

fn validate_replacement(
    app: &App<'_>,
    replacement: &domain::delivery::Replacement,
) -> Result<(), String> {
    let observed = app
        .services
        .github
        .observe(replacement.pr)
        .map_err(|error| error.0)?;
    let unchanged = observed.state == PrState::Merged
        && observed.head == replacement.head
        && observed.merge.as_ref() == Some(&replacement.merge);
    if unchanged {
        Ok(())
    } else {
        Err("replacement identity changed".into())
    }
}

pub(super) fn current_snapshot(task: &domain::task::Task) -> Option<&Snapshot> {
    match &task.phase {
        Phase::Planned { work } | Phase::InFlight { work, .. } => work.snapshot.as_ref(),
        Phase::Merged { .. }
        | Phase::Queued
        | Phase::Blocked(_)
        | Phase::NeedsInput(_)
        | Phase::Claimed
        | Phase::Verified { .. }
        | Phase::Excluded(_) => task.deliveries.last().and_then(review_snapshot),
    }
}

pub(super) fn review_snapshot(delivery: &Delivery) -> Option<&Snapshot> {
    delivery.review.as_ref().map(|review| &review.snapshot)
}

pub(super) fn publish_ready(
    state: &domain::state::State,
    task: &domain::task::Task,
    delivery: &Delivery,
    input: &PublishInput,
) -> Result<(), String> {
    if !matches!(delivery.outcome, Outcome::Open)
        || !matches!(delivery.kind, DeliveryKind::Code { .. })
    {
        return Err("publish needs an open code delivery".into());
    }
    if input.step != PublishStep::Merge {
        return Ok(());
    }
    merge_ready(state, task, delivery)
}

fn merge_ready(
    state: &domain::state::State,
    task: &domain::task::Task,
    delivery: &Delivery,
) -> Result<(), String> {
    let review = delivery
        .review
        .as_ref()
        .ok_or("merge needs independent review")?;
    if review.verdict != Verdict::Pass || review::merge_ready(review).is_err() {
        return Err("merge needs PASS and dispositions".into());
    }
    let human_mode = matches!(
        state.config.as_ref().map(|config| config.review_mode),
        Some(ReviewMode::HumanReview)
    );
    let current_human = state
        .human_reviews
        .get(&task.id)
        .map(|(snapshot, _)| snapshot)
        == Some(&review.snapshot);
    if human_mode && !current_human {
        return Err("human review receipt is stale or missing".into());
    }
    if required_checks_failed(delivery) {
        return Err("required checks are not green".into());
    }
    Ok(())
}

fn required_checks_failed(delivery: &Delivery) -> bool {
    let DeliveryKind::Code { pr: Some(pr) } = &delivery.kind else {
        return true;
    };
    pr.head
        != delivery
            .review
            .as_ref()
            .map_or_else(|| pr.head.clone(), |review| review.snapshot.head.clone())
        || pr
            .checks
            .iter()
            .any(|check| check.required && check.state != CheckState::Pass)
}

pub(super) fn publish_action(
    delivery: &Delivery,
    snapshot: &Snapshot,
    input: &PublishInput,
) -> Result<GitHubAction, String> {
    match input.step {
        PublishStep::Create => Ok(GitHubAction::Create {
            title: input.title.clone().ok_or("create needs title")?,
            body: input.body.clone().ok_or("create needs body")?,
            head: snapshot.head.clone(),
        }),
        PublishStep::Ready => Ok(GitHubAction::Ready {
            pr: pr_number(delivery)?,
            head: snapshot.head.clone(),
        }),
        PublishStep::Merge => Ok(GitHubAction::Merge {
            pr: pr_number(delivery)?,
            head: snapshot.head.clone(),
            method: input.method.ok_or("merge needs method")?,
        }),
    }
}

pub(super) fn execute_publish(
    app: &App<'_>,
    delivery: &Delivery,
    snapshot: &Snapshot,
    input: &PublishInput,
) -> Result<domain::delivery::PullRequest, String> {
    match input.step {
        PublishStep::Create => app
            .services
            .github
            .create(
                input.title.as_deref().ok_or("create needs title")?,
                input.body.as_deref().ok_or("create needs body")?,
                &snapshot.head,
            )
            .map_err(|error| error.0),
        PublishStep::Ready => app
            .services
            .github
            .ready(pr_number(delivery)?, &snapshot.head)
            .map_err(|error| error.0),
        PublishStep::Merge => app
            .services
            .github
            .merge(
                pr_number(delivery)?,
                &snapshot.head,
                input.method.ok_or("merge needs method")?,
            )
            .map_err(|error| error.0),
    }
}

pub(super) fn pr_number(delivery: &Delivery) -> Result<PrNumber, String> {
    match &delivery.kind {
        DeliveryKind::Code { pr: Some(pr) } => Ok(pr.number),
        _ => Err("delivery has no PR".into()),
    }
}

pub(super) fn validate_closed(
    delivery: &Delivery,
    observed: &domain::delivery::PullRequest,
) -> Result<(), String> {
    match &delivery.outcome {
        Outcome::Merged { commit, .. }
            if observed.state == PrState::Merged && observed.merge.as_ref() == Some(commit) =>
        {
            Ok(())
        }
        Outcome::Replaced { by, .. }
            if observed.number == by.pr
                && observed.head == by.head
                && observed.merge.as_ref() == Some(&by.merge) =>
        {
            Ok(())
        }
        Outcome::Accepted { .. } | Outcome::Abandoned { .. } => Ok(()),
        _ => Err("closed delivery observation changed".into()),
    }
}

pub(super) fn verify_proof(
    task: &domain::task::Task,
    delivery: &Delivery,
    snapshot: &Snapshot,
) -> Result<(), String> {
    let mut artifacts = BTreeMap::new();
    for entry in delivery.proof.entries.values() {
        let bytes = fs::read(&entry.artifact).map_err(|error| error.to_string())?;
        let digest = domain::ids::Digest::from_str(&format!("{:x}", Sha256::digest(bytes)))
            .map_err(|error| error.to_string())?;
        artifacts.insert(entry.artifact.clone(), digest);
    }
    let baselines = delivery
        .work
        .as_ref()
        .and_then(|work| work.plan.as_ref())
        .map(|plan| plan.baselines.clone())
        .unwrap_or_default();
    let criteria = delivery
        .criteria
        .iter()
        .map(|id| {
            task.spec
                .criteria
                .iter()
                .find(|criterion| criterion.id == *id)
                .cloned()
                .unwrap_or_else(|| domain::task::Criterion {
                    id: id.clone(),
                    text: "authorized narrowed criterion".into(),
                    human_only: false,
                })
        })
        .collect::<Vec<_>>();
    acceptance::complete(&criteria, &delivery.proof, snapshot, &artifacts, &baselines)
        .map_err(|error| format!("acceptance failed: {error:?}"))
}

pub(super) fn current_sync<'a>(task: &'a domain::task::Task, issue: &IssueKey) -> Option<&'a Sync> {
    if IssueKey::from(task.id.clone()) == *issue {
        Some(&task.sync)
    } else {
        task.subtasks.get(issue).map(|subtask| &subtask.sync)
    }
}
