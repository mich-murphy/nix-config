mod common;

use adapters::sqlite::Store;
use app::App;
use common::golden::{narrow_record_review_and_merge, plan_and_implement, task_baseline};
use common::literals::{criterion_id, digest, write_text};
use common::script::{
    Step, execute, final_verify, merged_commit, record_proof, record_proof_with, review, run_steps,
    step, unchecked,
};
use common::*;
use domain::{
    acceptance::Snapshot,
    authority::{AuthorityReceipt, Grant},
    command::{Command, JiraRead, NextAction},
    delivery::DeliveryKind,
    task::{Hold, HoldReason, Phase},
};

/// Builds the `open-delivery` step that resumes full AC1 acceptance
/// against the narrowed merge, once the coordinator has a fresh Jira read
/// and an authority receipt for it.
fn open_verification_delivery(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<Step, Box<dyn std::error::Error>> {
    let commit = merged_commit(app, task)?;
    let requirements = task_state(app, task)?.spec.requirements;
    let receipt = directory.join("open-delivery-receipt.txt");
    let receipt_digest =
        write_artifact(&receipt, "resume full acceptance after the narrowed merge")?;
    Ok(step(
        |action| matches!(action, NextAction::OpenDelivery { .. }),
        execute(Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: commit.clone() },
            receipt: Some(AuthorityReceipt {
                source: "current user".into(),
                artifact: receipt,
                digest: receipt_digest,
                requirements: requirements.clone(),
                grant: Grant::Delivery {
                    kind: DeliveryKind::Verification { of: commit },
                    criteria: vec![criterion_id("AC1")],
                    paths: None,
                },
            }),
            jira: JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        }),
    ))
}

/// Asserts the task is held on `NeedsHuman`, the hold `narrow-acceptance`
/// leaves behind once its delivery has merged with acceptance still open.
fn assert_held_for_human(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    let held = matches!(
        state.hold,
        Some(Hold {
            reason: HoldReason::NeedsHuman { .. },
            ..
        })
    );
    if held {
        Ok(())
    } else {
        Err(format!("expected a NeedsHuman hold, found {:?}", state.hold).into())
    }
}

/// Builds the rest of the script once the narrowed merge is held on
/// `NeedsHuman`: `open-delivery` against it, then measured proof, a PASS
/// review and `final-verify` against the same snapshot (the merge commit
/// as both base and head, since a `Verification` delivery reopens no new
/// commits).
fn verification_script(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<Vec<Step>, Box<dyn std::error::Error>> {
    let open_delivery = open_verification_delivery(app, task, directory)?;
    let commit = merged_commit(app, task)?;
    let requirements = task_state(app, task)?.spec.requirements;
    let snapshot = Snapshot {
        base: commit.clone(),
        head: commit,
        requirements,
    };
    let ac1_baseline = task_baseline(app, task, "AC1")?;
    Ok(vec![
        open_delivery,
        step(
            |action| matches!(action, NextAction::RecordProof { .. }),
            record_proof_with(
                "AC1",
                ac1_baseline,
                snapshot.clone(),
                directory.join("proof-final-ac1.txt"),
            ),
        ),
        step(
            |action| matches!(action, NextAction::Review { .. }),
            review(write_text(directory, "review-final.md", "bounded task")),
        ),
        step(
            |action| matches!(action, NextAction::FinalVerify { .. }),
            final_verify(directory.join("final-verify.txt")),
        ),
    ])
}

/// `plan_and_implement`, then one recorded AC1 proof (superseded, not
/// reused, once `narrow-acceptance` clears it — same flow as
/// `code_delivery_reaches_cleanup`), a narrowed STAGE1 merge, and the
/// `NeedsHuman` hold that merge leaves behind: the shared starting point
/// `verification_delivery_reaches_verified`, `open_delivery_repurposes_unused_pair`
/// and `verification_routes_to_acceptance` all need before they diverge
/// on what happens next.
fn narrowed_and_held(
    app: &mut App<'_>,
    fake: &Fake,
    directory: &std::path::Path,
) -> Result<domain::ids::TaskId, Box<dyn std::error::Error>> {
    let task = plan_and_implement(app, directory)?;
    run_steps(
        app,
        fake,
        &task,
        vec![unchecked(record_proof(
            "AC1",
            digest('b'),
            directory.join("proof-ac1.txt"),
        ))],
    )?;
    narrow_record_review_and_merge(app, fake, &task, directory, None)?;
    assert_held_for_human(app, &task)?;
    Ok(task)
}

#[test]
fn verification_delivery_reaches_verified() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = narrowed_and_held(&mut app, &fake, directory.path())?;

    let script = verification_script(&mut app, &task, directory.path())?;
    run_steps(&mut app, &fake, &task, script)?;

    let state = task_state(&mut app, &task)?;
    assert!(matches!(state.phase, Phase::Verified { .. }));
    Ok(())
}

/// Grants an unused `Pair` for the held task, then repurposes it (same
/// digest and artifact) as the grant for a `Verification` delivery
/// reopened against `commit`. Returns the authority id the pair was
/// granted under, for the caller to confirm it, not a new one, now
/// carries the delivery grant.
fn grant_then_repurpose_pair(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
    commit: domain::ids::Sha,
) -> Result<domain::ids::AuthorityId, Box<dyn std::error::Error>> {
    let requirements = task_state(app, task)?.spec.requirements;
    let artifact = directory.join("pair-receipt.txt");
    let digest = write_artifact(&artifact, "one more pair for cleanup")?;
    app.execute(
        Command::Grant {
            task: task.clone(),
            receipt: AuthorityReceipt {
                source: "current user".into(),
                artifact: artifact.clone(),
                digest: digest.clone(),
                requirements: requirements.clone(),
                grant: Grant::Pair { paths: None },
            },
        },
        false,
    )?;
    let pair_id = pair_authority_id(app, task, &digest)?;
    app.execute(
        Command::OpenDelivery {
            task: task.clone(),
            kind: DeliveryKind::Verification { of: commit.clone() },
            receipt: Some(AuthorityReceipt {
                source: "current user".into(),
                artifact,
                digest,
                requirements: requirements.clone(),
                grant: Grant::Delivery {
                    kind: DeliveryKind::Verification { of: commit },
                    criteria: vec![criterion_id("AC1")],
                    paths: None,
                },
            }),
            jira: JiraRead {
                member: true,
                resolved: false,
                ownership_clear: true,
                requirements,
            },
        },
        false,
    )?;
    Ok(pair_id)
}

fn pair_authority_id(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    digest: &domain::ids::Digest,
) -> Result<domain::ids::AuthorityId, Box<dyn std::error::Error>> {
    task_state(app, task)?
        .authorities
        .iter()
        .find(|entry| entry.digest == *digest)
        .map(|entry| entry.id)
        .ok_or_else(|| "pair authority missing".into())
}

/// A receipt naming an unused `Pair` grant repurposes it as `open-delivery`'s
/// grant (design Section 5, batch B item 4): no second authority is
/// registered under the same digest, and the existing one's `grant`
/// changes from `Pair` to `Delivery` in place, recorded by the explicit
/// `AuthorityRepurposed` event rather than a hidden mutation inside
/// `apply`.
#[test]
fn open_delivery_repurposes_unused_pair() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = narrowed_and_held(&mut app, &fake, directory.path())?;

    let commit = merged_commit(&mut app, &task)?;
    let pair_id = grant_then_repurpose_pair(&mut app, &task, directory.path(), commit)?;

    assert_authority_count_and_grant(&mut app, &task, pair_id, 2)?;
    Ok(())
}

/// Asserts the task has exactly `count` authorities (proving repurposing
/// registered no second one) and that `pair_id` now carries a `Delivery`
/// grant.
fn assert_authority_count_and_grant(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    pair_id: domain::ids::AuthorityId,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    assert_eq!(
        state.authorities.len(),
        count,
        "repurposing must not register a second authority"
    );
    let repurposed = state
        .authorities
        .iter()
        .find(|entry| entry.id == pair_id)
        .ok_or("repurposed authority missing")?;
    assert!(matches!(repurposed.grant, Grant::Delivery { .. }));
    Ok(())
}

/// Once a held task opens a `Verification` delivery against its narrowed
/// merge commit, `next` is final acceptance throughout: it asks for the
/// missing proof, then the review, and once both are satisfied it yields
/// `FinalVerify` naming the delivery's own `of` commit as both base and
/// head — never `Publish`, which a `Verification` delivery has no PR to
/// support.
/// `verification_script`'s steps up to (but not performing) `final-verify`
/// itself: `open-delivery`, the reopened delivery's own AC1 proof, and its
/// PASS review, leaving `final-verify` for the caller to assert `next`
/// against instead of executing.
fn record_proof_and_review_steps(
    app: &mut App<'_>,
    task: &domain::ids::TaskId,
    directory: &std::path::Path,
) -> Result<Vec<Step>, Box<dyn std::error::Error>> {
    let mut script = verification_script(app, task, directory)?;
    script.truncate(3);
    Ok(script)
}

#[test]
fn verification_routes_to_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let task = narrowed_and_held(&mut app, &fake, directory.path())?;

    let commit = merged_commit(&mut app, &task)?;
    let script = record_proof_and_review_steps(&mut app, &task, directory.path())?;
    run_steps(&mut app, &fake, &task, script)?;

    let action = expect_next(&mut app, |action| {
        matches!(action, NextAction::FinalVerify { .. })
    })?;
    assert_eq!(
        action,
        NextAction::FinalVerify {
            task: task.clone(),
            commit,
        }
    );
    Ok(())
}
