use adapters::{Process, ProcessOutput, ProcessRequest, sqlite::Store};
use app::{App, ResultData, Services, initialize};
use domain::{
    command::{Command, DiscoveredTask, JiraConfig, ReviewMode, RiskConfig, RunConfig, StatusMap},
    ids::{Digest, JiraStatus, ModelId, Sha, TaskId},
    ports::{
        Capabilities, Clock, GitHub, Harness, Isolation, LaunchRequest, LaunchResult, MergeMethod,
        PortError, StructuredOutput, Vcs,
    },
    risk::{
        Assignment, Effort, Escalation, HarnessConfig, HarnessKind, Model, Profile, Roles,
        TierProfiles,
    },
    task::TaskSpec,
};
use std::{collections::BTreeMap, path::Path, str::FromStr};

struct Fake;

impl Clock for Fake {
    fn now(&self) -> u64 {
        10
    }
}

impl Process for Fake {
    fn run(&self, _request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        Ok(ProcessOutput {
            code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

impl Vcs for Fake {
    fn head(&self, _revision: &str) -> Result<Sha, PortError> {
        sha('a')
    }
    fn on_main(&self, _commit: &Sha) -> Result<bool, PortError> {
        Ok(true)
    }
    fn changed_paths(&self, _base: &Sha, _head: &Sha) -> Result<Vec<String>, PortError> {
        Ok(Vec::new())
    }
    fn changed_lines(&self, _base: &Sha, _head: &Sha) -> Result<u32, PortError> {
        Ok(0)
    }
    fn commit_paths(&self, _base: &Sha, _head: &Sha) -> Result<Vec<Vec<String>>, PortError> {
        Ok(Vec::new())
    }
    fn ignored(&self, _path: &Path) -> Result<bool, PortError> {
        Ok(true)
    }
    fn reserve(&self, _task: &TaskId) -> Result<bool, PortError> {
        Ok(true)
    }
    fn bind_slot(
        &self,
        _task: &TaskId,
        _slot: &domain::ids::SlotId,
        _branch: &str,
    ) -> Result<(), PortError> {
        Ok(())
    }
    fn clean_slot(&self, _slot: &domain::ids::SlotId, _delete: bool) -> Result<(), PortError> {
        Ok(())
    }
}

impl GitHub for Fake {
    fn create(
        &self,
        _title: &str,
        _body: &Path,
        _head: &Sha,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        Err(PortError("unused".into()))
    }
    fn ready(
        &self,
        _number: domain::ids::PrNumber,
        _head: &Sha,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        Err(PortError("unused".into()))
    }
    fn merge(
        &self,
        _number: domain::ids::PrNumber,
        _head: &Sha,
        _method: MergeMethod,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        Err(PortError("unused".into()))
    }
    fn observe(
        &self,
        _number: domain::ids::PrNumber,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        Err(PortError("unused".into()))
    }
}

impl Harness for Fake {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Pi
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::SelfValidated,
            isolation: Isolation::ToolRestriction,
        }
    }
    fn run(&self, _request: &LaunchRequest) -> Result<LaunchResult, PortError> {
        Err(PortError("unused".into()))
    }
}

fn services(fake: &Fake) -> Services<'_> {
    Services {
        vcs: fake,
        github: fake,
        harness: fake,
        process: fake,
        clock: fake,
    }
}

fn initialized() -> Result<(tempfile::TempDir, Fake), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let fake = Fake;
    initialize(
        directory.path(),
        config(directory.path())?,
        profile()?,
        services(&fake),
    )?;
    Ok((directory, fake))
}

fn config(repo: &Path) -> Result<RunConfig, domain::ids::InvalidId> {
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

fn profile() -> Result<Profile, domain::ids::InvalidId> {
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

fn task() -> Result<DiscoveredTask, domain::ids::InvalidId> {
    Ok(DiscoveredTask {
        id: TaskId::from_str("GAIN-2")?,
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

fn sha(seed: char) -> Result<Sha, PortError> {
    Sha::from_str(&seed.to_string().repeat(40)).map_err(|error| PortError(error.to_string()))
}

#[test]
fn check_flag_writes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let preview = app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        true,
    )?;
    assert!(!preview.events.is_empty());
    let state = app.execute(Command::Status, false)?;
    match state.result {
        ResultData::State { state } => assert!(!state.frozen),
        _ => return Err("status returned wrong result".into()),
    }
    Ok(())
}

#[test]
fn command_returns_events() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    let output = app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    assert_eq!(output.events.len(), 1);
    Ok(())
}

#[test]
fn next_fills_command_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let output = app.execute(Command::Next, false)?;
    match output.result {
        ResultData::Next { next: Some(next) } => {
            assert!(next.command.contains("controller claim GAIN-2"));
            assert!(next.schema.required.contains(&"run".to_owned()));
            assert!(!next.schema.properties.is_empty());
        }
        _ => return Err("next returned no action".into()),
    }
    Ok(())
}

#[test]
fn refresh_accepts_delta() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, fake) = initialized()?;
    let mut app = App::new(Store::open(directory.path())?, services(&fake));
    app.execute(
        Command::Discover {
            tasks: vec![task()?],
            planning_order: None,
        },
        false,
    )?;
    let mut changed = task()?;
    changed.spec.priority = 2;
    let output = app.execute(
        Command::Refresh {
            changed: vec![changed],
            removed: Vec::new(),
        },
        false,
    )?;
    assert_eq!(output.events.len(), 1);
    Ok(())
}
