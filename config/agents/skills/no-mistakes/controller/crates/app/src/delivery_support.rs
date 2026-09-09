use crate::App;
use crate::delivery::PublishInput;
use crate::task_support::digest_file;
use crate::{ConflictReason, EvidenceError, Rejection};
use domain::{
    acceptance::{self, Snapshot},
    command::{JiraRead, PublishStep, ReviewMode},
    delivery::{CheckState, Delivery, DeliveryKind, Outcome, PrState},
    event::{Event, GitHubAction},
    ids::{IssueKey, PrNumber},
    review::{self, Verdict},
    sync::Sync,
    task::{HoldReason, Phase, Task},
};
use std::collections::BTreeMap;

pub(super) fn validate_open(task: &Task, jira: &JiraRead) -> Result<(), ConflictReason> {
    if !matches!(
        task.hold.as_ref().map(|hold| &hold.reason),
        Some(HoldReason::NeedsHuman { .. })
    ) {
        return Err(ConflictReason::OpenRequiresNeedsHumanHold);
    }
    let terminal = task.spec.was_terminal
        || matches!(
            task.phase,
            Phase::Verified { .. } | Phase::Completed { .. } | Phase::Excluded(_)
        );
    if terminal {
        return Err(ConflictReason::TerminalTaskCannotOpen);
    }
    if !jira.member || jira.resolved || !jira.ownership_clear {
        return Err(ConflictReason::JiraOwnershipUnresolved);
    }
    if jira.requirements != task.spec.requirements {
        return Err(ConflictReason::CriteriaChanged);
    }
    Ok(())
}

pub(super) fn validate_history(app: &App<'_>, task: &Task) -> Result<(), Rejection> {
    for delivery in &task.deliveries {
        match &delivery.outcome {
            Outcome::Merged { commit, .. }
                if !app
                    .services
                    .vcs
                    .on_main(commit)
                    .map_err(|error| Rejection::External(error.0))? =>
            {
                return Err(Rejection::Conflict(ConflictReason::HistoricalMergeLeftMain));
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
) -> Result<(), Rejection> {
    let observed = app
        .services
        .github
        .observe(replacement.pr)
        .map_err(|error| Rejection::External(error.0))?;
    let unchanged = observed.state == PrState::Merged
        && observed.head == replacement.head
        && observed.merge.as_ref() == Some(&replacement.merge);
    if unchanged {
        Ok(())
    } else {
        Err(Rejection::Conflict(
            ConflictReason::ReplacementIdentityChanged,
        ))
    }
}

/// The snapshot currently in force for a task's current delivery: the one
/// its `PlannedWork` is tracking while open, or the one its settled review
/// last covered once closed. Both sides read the same delivery, so they
/// agree by construction rather than by a phase-keyed branch.
pub(super) fn current_snapshot(task: &Task) -> Option<&Snapshot> {
    let delivery = task.current_delivery()?;
    delivery
        .work
        .as_ref()
        .and_then(|work| work.snapshot.as_ref())
        .or_else(|| review_snapshot(delivery))
}

pub(super) fn review_snapshot(delivery: &Delivery) -> Option<&Snapshot> {
    delivery.review.as_ref().map(|review| &review.snapshot)
}

pub(super) fn publish_ready(
    state: &domain::state::State,
    delivery: &Delivery,
    input: &PublishInput,
) -> Result<(), EvidenceError> {
    if !matches!(delivery.outcome, Outcome::Open)
        || !matches!(delivery.kind, DeliveryKind::Code { .. })
    {
        return Err(other("publish needs an open code delivery"));
    }
    if input.step != PublishStep::Merge {
        return Ok(());
    }
    merge_ready(state, delivery)
}

fn merge_ready(state: &domain::state::State, delivery: &Delivery) -> Result<(), EvidenceError> {
    let review = delivery
        .review
        .as_ref()
        .ok_or_else(|| other("merge needs independent review"))?;
    if review.verdict != Verdict::Pass || review::merge_ready(review).is_err() {
        return Err(other("merge needs PASS and dispositions"));
    }
    let human_mode = matches!(
        state.config.as_ref().map(|config| config.review_mode),
        Some(ReviewMode::HumanReview)
    );
    let current_human = delivery
        .human_review
        .as_ref()
        .is_some_and(|receipt| receipt.snapshot == review.snapshot);
    if human_mode && !current_human {
        return Err(other("human review receipt is stale or missing"));
    }
    if required_checks_failed(delivery) {
        return Err(other("required checks are not green"));
    }
    Ok(())
}

fn other(message: &str) -> EvidenceError {
    EvidenceError::Other(message.into())
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

pub(super) fn append_acceptance_hold(
    app: &App<'_>,
    task: &Task,
    delivery: &Delivery,
    events: &mut Vec<Event>,
) {
    let full = task
        .spec
        .criteria
        .iter()
        .map(|criterion| criterion.id.clone())
        .collect::<Vec<_>>();
    if delivery.criteria != full || !domain::delivery::acceptance_ready(delivery, &full) {
        events.push(Event::Held {
            task: task.id.clone(),
            reason: HoldReason::NeedsHuman {
                diagnosis: "merged delivery does not establish full acceptance".into(),
                remaining: full,
            },
            at: app.services.clock.now(),
        });
    }
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
) -> Result<(), ConflictReason> {
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
        _ => Err(ConflictReason::ClosedDeliveryChanged),
    }
}

pub(super) fn verify_proof(
    task: &Task,
    delivery: &Delivery,
    snapshot: &Snapshot,
) -> Result<(), EvidenceError> {
    let mut artifacts = BTreeMap::new();
    for entry in delivery.proof.entries.values() {
        artifacts.insert(entry.artifact.clone(), digest_file(&entry.artifact)?);
    }
    let baselines = &task.baselines;
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
    acceptance::complete(&criteria, &delivery.proof, snapshot, &artifacts, baselines)?;
    Ok(())
}

pub(super) fn current_sync<'a>(task: &'a Task, issue: &IssueKey) -> Option<&'a Sync> {
    if IssueKey::from(task.id.clone()) == *issue {
        Some(&task.sync)
    } else {
        task.subtasks.get(issue).map(|subtask| &subtask.sync)
    }
}
