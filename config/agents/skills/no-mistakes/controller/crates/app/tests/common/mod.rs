#![allow(dead_code)]

use adapters::Process;
use app::{App, ResultData, Services, initialize};
use domain::{
    acceptance::Snapshot,
    command::{
        Command, DiscoveredTask, JiraConfig, NextAction, ReviewMode, RiskConfig, RunConfig,
        StatusMap, Transition,
    },
    ids::{
        CriterionId, DeliveryId, Digest, IssueKey, JiraStatus, ModelId, Sha, SlotId, TaskId,
        TransitionId,
    },
    ports::PortError,
    risk::{
        Assignment, Effort, Escalation, HarnessConfig, HarnessKind, Model, Profile, Roles,
        TierProfiles,
    },
    task::{Criterion, Plan, Task, TaskSpec},
};
use std::{collections::BTreeMap, path::Path, str::FromStr};

mod fake;
pub mod golden;
pub mod literals;
mod process;
pub mod script;

pub use fake::Fake;

pub fn services_with_process<'a>(fake: &'a Fake, process: &'a dyn Process) -> Services<'a> {
    Services {
        vcs: fake,
        github: fake,
        harness: fake,
        process,
        clock: fake,
    }
}

pub fn services(fake: &Fake) -> Services<'_> {
    Services {
        vcs: fake,
        github: fake,
        harness: fake,
        process: fake,
        clock: fake,
    }
}

pub fn initialized() -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    initialized_with(Fake {
        reserve: true,
        slot: domain::ports::SlotState::Missing,
        observation: std::cell::RefCell::new(None),
        clock: std::cell::Cell::new(10),
        head: std::cell::Cell::new('a'),
        reviewer_output: std::cell::RefCell::new("done".into()),
        pending_checks: std::cell::Cell::new(false),
    })
}

pub fn initialized_with(
    fake: Fake,
) -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    initialize(
        directory.path(),
        config(directory.path())?,
        profile()?,
        services(&fake),
    )?;
    Ok((directory, fake))
}

pub fn config(repo: &Path) -> Result<RunConfig, domain::ids::InvalidId> {
    Ok(RunConfig {
        repo: repo.to_owned(),
        github_repo: "owner/repo".into(),
        epic: TaskId::from_str("GAIN-1")?,
        review_mode: ReviewMode::Autonomous,
        max_open_prs: 2,
        feedback_file: None,
        jira: JiraConfig {
            statuses: StatusMap {
                todo: JiraStatus::from_str("todo")?,
                progress: JiraStatus::from_str("progress")?,
                review: JiraStatus::from_str("review")?,
                done: JiraStatus::from_str("done")?,
            },
        },
        risk: RiskConfig {
            sensitive: vec!["auth/**".into()],
        },
        caps: BTreeMap::new(),
    })
}

pub fn profile() -> Result<Profile, domain::ids::InvalidId> {
    let terra = ModelId::from_str("openai/terra")?;
    let sol = ModelId::from_str("openai/sol")?;
    let astra = ModelId::from_str("openai/astra")?;
    let mut models = BTreeMap::new();
    models.insert(
        terra.clone(),
        Model {
            rank: 1,
            fallback_for: Vec::new(),
        },
    );
    models.insert(
        sol.clone(),
        Model {
            rank: 2,
            fallback_for: vec![terra.clone()],
        },
    );
    models.insert(
        astra.clone(),
        Model {
            rank: 3,
            fallback_for: Vec::new(),
        },
    );
    let implementer = Assignment {
        model: terra,
        effort: Effort::Medium,
    };
    let reviewer = Assignment {
        model: sol.clone(),
        effort: Effort::Medium,
    };
    Ok(Profile {
        harness: HarnessConfig {
            kind: HarnessKind::Pi,
        },
        models,
        coordinator: reviewer.clone(),
        tier: TierProfiles {
            trivial: Roles {
                implementer: implementer.clone(),
                reviewer: reviewer.clone(),
            },
            lite: Roles {
                implementer: reviewer.clone(),
                reviewer: reviewer.clone(),
            },
            full: Roles {
                implementer: reviewer.clone(),
                reviewer,
            },
        },
        escalation: Escalation {
            reviewer: Assignment {
                model: astra,
                effort: Effort::High,
            },
        },
    })
}

pub fn task() -> Result<DiscoveredTask, domain::ids::InvalidId> {
    task_named("GAIN-2")
}

pub fn task_named(id: &str) -> Result<DiscoveredTask, domain::ids::InvalidId> {
    Ok(DiscoveredTask {
        id: TaskId::from_str(id)?,
        spec: TaskSpec {
            parent: TaskId::from_str("GAIN-1")?,
            criteria: Vec::new(),
            dependencies: Vec::new(),
            priority: 1,
            due: None,
            rank: "a".into(),
            not_before: None,
            member: true,
            ownership_clear: true,
            ownership_evidence: "fresh reads".into(),
            was_terminal: false,
            jira_status: JiraStatus::from_str("todo")?,
            requirements: Digest::from_str(&"0".repeat(64))?,
        },
    })
}

pub fn sha(seed: char) -> Result<Sha, PortError> {
    Sha::from_str(&seed.to_string().repeat(40)).map_err(|error| PortError(error.to_string()))
}

/// Writes `contents` to `path` and returns the digest a proof or authority
/// receipt must be recorded against.
pub fn write_artifact(
    path: &std::path::Path,
    contents: &str,
) -> Result<Digest, Box<dyn std::error::Error>> {
    use sha2::{Digest as _, Sha256};
    std::fs::write(path, contents)?;
    Ok(Digest::from_str(&format!(
        "{:x}",
        Sha256::digest(std::fs::read(path)?)
    ))?)
}

/// Reads one task back through `Command::Status`, the public boundary
/// tests assert at, instead of a nested `match` on `ResultData` at every
/// call site.
pub fn task_state(app: &mut App<'_>, task: &TaskId) -> Result<Task, Box<dyn std::error::Error>> {
    let ResultData::State { state } = app.execute(Command::Status, false)?.result else {
        return Err("status returned wrong result".into());
    };
    state
        .tasks
        .get(task)
        .cloned()
        .ok_or_else(|| "task missing from status".into())
}

/// The current delivery's id and work snapshot, read back through
/// `Command::Status`. Panics-free stand-in for the 40-field literal a test
/// would otherwise need to track a delivery's state.
pub fn current_delivery(
    app: &mut App<'_>,
    task: &TaskId,
) -> Result<(DeliveryId, Snapshot), Box<dyn std::error::Error>> {
    let state = task_state(app, task)?;
    let delivery = state.deliveries.last().ok_or("delivery missing")?;
    let work = delivery.work.as_ref().ok_or("delivery work missing")?;
    Ok((
        delivery.id,
        work.snapshot.clone().ok_or("snapshot missing")?,
    ))
}

/// Asserts that `Command::Next` currently yields the action `predicate`
/// describes, failing with the actual action printed rather than a bare
/// mismatch. This is the contract design Section 11 promises: the model
/// never performs a step `next` did not just name.
pub fn expect_next(
    app: &mut App<'_>,
    predicate: impl Fn(&NextAction) -> bool,
) -> Result<NextAction, Box<dyn std::error::Error>> {
    let ResultData::Next { next } = app.execute(Command::Next, false)?.result else {
        return Err("next returned wrong result".into());
    };
    let envelope = next.ok_or("next returned no action")?;
    if !predicate(&envelope.action) {
        return Err(format!("unexpected next action: {:?}", envelope.action).into());
    }
    Ok(envelope.action)
}

pub fn prepare(app: &mut App<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let task = TaskId::from_str("GAIN-2")?;
    discover_claim(app, &task)?;
    sync_progress(app, &task)?;
    brief_bind_plan(app, &task)
}

pub fn discover_claim(
    app: &mut App<'_>,
    task_id: &TaskId,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    app.execute(
        Command::Claim {
            task: task_id.clone(),
        },
        false,
    )?;
    Ok(())
}

pub fn sync_progress(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: JiraStatus::from_str("todo")?,
            target: JiraStatus::from_str("progress")?,
            transitions: vec![Transition {
                id: TransitionId::from_str("31")?,
                to: JiraStatus::from_str("progress")?,
            }],
        },
        false,
    )?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            status: JiraStatus::from_str("progress")?,
            evidence: "fresh connector read".into(),
        },
        false,
    )?;
    Ok(())
}

/// A generic Jira status round trip: `set-status` then `observe-status`,
/// confirming `target` was reached via `transition`. `sync_progress` above
/// is the fixed `todo` -> `progress` case every `prepare` needs; this is
/// the same round trip for any other status (`review`, `done`, ...).
pub fn sync_status(
    app: &mut App<'_>,
    task: &TaskId,
    current: &str,
    target: &str,
    transition: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    app.execute(
        Command::SetStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            current: JiraStatus::from_str(current)?,
            target: JiraStatus::from_str(target)?,
            transitions: vec![Transition {
                id: TransitionId::from_str(transition)?,
                to: JiraStatus::from_str(target)?,
            }],
        },
        false,
    )?;
    app.execute(
        Command::ObserveStatus {
            task: task.clone(),
            issue: IssueKey::from(task.clone()),
            status: JiraStatus::from_str(target)?,
            evidence: "fresh connector read".into(),
        },
        false,
    )?;
    Ok(())
}

pub fn brief_bind_plan(app: &mut App<'_>, task: &TaskId) -> Result<(), Box<dyn std::error::Error>> {
    let criterion = CriterionId::from_str("AC1")?;
    app.execute(
        Command::Brief {
            task: task.clone(),
            criteria: vec![Criterion {
                id: criterion.clone(),
                text: "observable result".into(),
                human_only: false,
            }],
        },
        false,
    )?;
    app.execute(
        Command::BindSlot {
            task: task.clone(),
            slot: SlotId::from_str("worker1")?,
            branch: "agent/worker1/GAIN-2".into(),
            authority: None,
        },
        false,
    )?;
    app.execute(
        Command::Plan {
            task: task.clone(),
            plan: Plan {
                deliverable: "observable result".into(),
                components: vec!["src".into()],
                examples: Vec::new(),
                baselines: BTreeMap::from([(criterion, Digest::from_str(&"b".repeat(64))?)]),
                lesson_families: vec!["controller".into()],
            },
        },
        false,
    )?;
    Ok(())
}
