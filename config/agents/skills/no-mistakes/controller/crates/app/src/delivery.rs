use crate::delivery_support::{
    get_task, next_delivery, validate_authority_file, validate_delivery_grant, validate_history,
    validate_open,
};
use crate::{AgentError, App, Output, Rejection};
use domain::{
    acceptance,
    authority::{self, Authority, Grant},
    command::{JiraRead, PublishStep},
    delivery::{self, Delivery, DeliveryKind, Outcome},
    event::{Event, OperationStatus},
    ids::{CriterionId, TaskId, UseId},
    ports::MergeMethod,
};
use std::path::PathBuf;

pub(super) struct PublishInput {
    pub step: PublishStep,
    pub title: Option<String>,
    pub body: Option<PathBuf>,
    pub method: Option<MergeMethod>,
}

impl App<'_> {
    pub(super) fn grant(
        &mut self,
        task: TaskId,
        authority: Authority,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("grant")?;
        let value = get_task(&state, &task)
            .map_err(|message| self.error("grant", Some(&task), Rejection::Invalid(message)))?;
        let idle = !state
            .launches
            .iter()
            .any(|launch| launch.task == task && launch.session.is_none())
            && !state.operations.iter().any(|operation| {
                operation.task == task
                    && matches!(
                        operation.status,
                        OperationStatus::Running | OperationStatus::Unknown
                    )
            });
        if matches!(authority.grant, Grant::Pair { .. }) && value.hold.is_none() {
            return Err(self.error(
                "grant",
                Some(&task),
                Rejection::Authority("pair grant requires a held task".into()),
            ));
        }
        authority::register(&value.authorities, &authority, idle).map_err(|error| {
            self.error(
                "grant",
                Some(&task),
                Rejection::Authority(format!("grant rejected: {error:?}")),
            )
        })?;
        validate_authority_file(&authority)
            .map_err(|message| self.error("grant", Some(&task), Rejection::Authority(message)))?;
        validate_grant_paths(self, "grant", &task, &authority)?;
        if authority.requirements != value.spec.requirements {
            return Err(self.error(
                "grant",
                Some(&task),
                Rejection::Authority("grant criteria changed".into()),
            ));
        }
        self.commit(
            "grant",
            Some(&task),
            vec![Event::AuthorityRegistered {
                task: task.clone(),
                authority: Box::new(authority),
            }],
            check,
        )
    }

    pub(super) fn open_delivery(
        &mut self,
        task: TaskId,
        kind: DeliveryKind,
        mut authority: Authority,
        jira: JiraRead,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("open-delivery")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Invalid(message))
        })?;
        validate_open(value, &jira).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Conflict(message))
        })?;
        delivery::can_open(&value.deliveries).map_err(|error| {
            self.error(
                "open-delivery",
                Some(&task),
                Rejection::Conflict(format!("delivery history: {error:?}")),
            )
        })?;
        let repurposed = repurposed_authority(value, &authority);
        prepare_authority(self, value, &task, &mut authority, repurposed)?;
        validate_open_authority(self, value, &task, &kind, &authority)?;
        validate_history(self, value).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Conflict(message))
        })?;
        let delivery = new_delivery(self, value, &task, kind, &authority)?;
        let events = open_events(&task, authority, delivery, repurposed.is_none());
        self.commit("open-delivery", Some(&task), events, check)
    }

    pub(super) fn narrow(
        &mut self,
        task: TaskId,
        criteria: Vec<CriterionId>,
        authority: Authority,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("narrow-acceptance")?;
        let value = get_task(&state, &task).map_err(|message| {
            self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Invalid(message),
            )
        })?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        delivery::can_narrow(delivery, &criteria).map_err(|error| {
            self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Conflict(format!("cannot narrow: {error:?}")),
            )
        })?;
        if overlaps_full_criteria(value, &criteria) {
            return Err(self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Invalid("narrowed criterion IDs must be distinct".into()),
            ));
        }
        validate_narrowing_authority(self, value, &task, &criteria, &authority)?;
        let events = vec![
            Event::AuthorityRegistered {
                task: task.clone(),
                authority: Box::new(authority.clone()),
            },
            Event::GrantUsed {
                task: task.clone(),
                authority: authority.id,
                by: UseId(delivery.id.0),
            },
            Event::AcceptanceNarrowed {
                task: task.clone(),
                delivery: delivery.id,
                criteria,
                authority: authority.id,
            },
        ];
        self.commit("narrow-acceptance", Some(&task), events, check)
    }
}

fn validate_grant_paths(
    app: &App<'_>,
    command: &str,
    task: &TaskId,
    authority: &Authority,
) -> Result<(), AgentError> {
    let Some(paths) = authority::paths(&authority.grant) else {
        return Ok(());
    };
    let repo = app
        .state(command)?
        .config
        .ok_or_else(|| {
            app.error(
                command,
                Some(task),
                Rejection::Internal("config missing".into()),
            )
        })?
        .repo;
    let valid = paths.iter().all(|path| {
        let file = repo.join(path);
        file.is_file() && !file.is_symlink()
    });
    if valid {
        Ok(())
    } else {
        Err(app.error(
            command,
            Some(task),
            Rejection::Authority("path scope must name existing regular Markdown files".into()),
        ))
    }
}

fn repurposed_authority(
    task: &domain::task::Task,
    authority: &Authority,
) -> Option<domain::ids::AuthorityId> {
    task.authorities
        .iter()
        .find(|entry| entry.digest == authority.digest)
        .filter(|entry| authority::unused_pair(entry))
        .map(|entry| entry.id)
}

fn validate_open_authority(
    app: &App<'_>,
    task: &domain::task::Task,
    id: &TaskId,
    kind: &DeliveryKind,
    authority: &Authority,
) -> Result<(), AgentError> {
    validate_authority_file(authority)
        .map_err(|message| app.error("open-delivery", Some(id), Rejection::Authority(message)))?;
    validate_delivery_grant(authority, kind)
        .map_err(|message| app.error("open-delivery", Some(id), Rejection::Authority(message)))?;
    validate_grant_paths(app, "open-delivery", id, authority)?;
    authority::validate_use(authority, &task.spec.requirements).map_err(|error| {
        app.error(
            "open-delivery",
            Some(id),
            Rejection::Authority(format!("authority use rejected: {error:?}")),
        )
    })
}

fn new_delivery(
    app: &App<'_>,
    task: &domain::task::Task,
    id: &TaskId,
    kind: DeliveryKind,
    authority: &Authority,
) -> Result<Delivery, AgentError> {
    let base = app
        .services
        .vcs
        .head("origin/main")
        .map_err(|error| app.error("open-delivery", Some(id), Rejection::External(error.0)))?;
    let criteria = match &authority.grant {
        Grant::Delivery { criteria, .. } => criteria.clone(),
        _ => Vec::new(),
    };
    Ok(Delivery {
        id: next_delivery(task),
        kind,
        authority: Some(authority.id),
        criteria,
        paths: authority::paths(&authority.grant).map(<[String]>::to_vec),
        base,
        work: None,
        proof: acceptance::Proof::default(),
        review: None,
        outcome: Outcome::Open,
        launches: Vec::new(),
        operations: Vec::new(),
    })
}

fn open_events(
    task: &TaskId,
    authority: Authority,
    delivery: Delivery,
    register: bool,
) -> Vec<Event> {
    let mut events = Vec::new();
    if register {
        events.push(Event::AuthorityRegistered {
            task: task.clone(),
            authority: Box::new(authority.clone()),
        });
    }
    let delivery_id = delivery.id;
    events.extend([
        Event::DeliveryOpened {
            task: task.clone(),
            delivery: Box::new(delivery),
        },
        Event::GrantUsed {
            task: task.clone(),
            authority: authority.id,
            by: UseId(delivery_id.0),
        },
        Event::Resumed { task: task.clone() },
    ]);
    events
}

fn validate_narrowing_authority(
    app: &App<'_>,
    task: &domain::task::Task,
    id: &TaskId,
    criteria: &[CriterionId],
    authority: &Authority,
) -> Result<(), AgentError> {
    authority::register(&task.authorities, authority, true).map_err(|error| {
        app.error(
            "narrow-acceptance",
            Some(id),
            Rejection::Authority(format!("authority rejected: {error:?}")),
        )
    })?;
    validate_authority_file(authority).map_err(|message| {
        app.error("narrow-acceptance", Some(id), Rejection::Authority(message))
    })?;
    let matches = authority.requirements == task.spec.requirements
        && matches!(&authority.grant, Grant::Narrowing { criteria: granted, .. } if granted == criteria);
    if matches {
        Ok(())
    } else {
        Err(app.error(
            "narrow-acceptance",
            Some(id),
            Rejection::Authority("narrowing grant does not match criteria".into()),
        ))
    }
}

fn overlaps_full_criteria(task: &domain::task::Task, criteria: &[CriterionId]) -> bool {
    criteria.iter().any(|id| {
        task.spec
            .criteria
            .iter()
            .any(|criterion| criterion.id == *id)
    })
}

fn prepare_authority(
    app: &App<'_>,
    task: &domain::task::Task,
    id: &TaskId,
    authority: &mut Authority,
    repurposed: Option<domain::ids::AuthorityId>,
) -> Result<(), AgentError> {
    if let Some(existing) = repurposed {
        authority.id = existing;
        return Ok(());
    }
    authority::register(&task.authorities, authority, true).map_err(|error| {
        app.error(
            "open-delivery",
            Some(id),
            Rejection::Authority(format!("authority rejected: {error:?}")),
        )
    })
}
