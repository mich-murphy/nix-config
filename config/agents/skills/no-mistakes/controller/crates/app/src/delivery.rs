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
        authority: Authority,
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
        authority::register(&value.authorities, &authority, true).map_err(|error| {
            self.error(
                "open-delivery",
                Some(&task),
                Rejection::Authority(format!("authority rejected: {error:?}")),
            )
        })?;
        validate_authority_file(&authority).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Authority(message))
        })?;
        validate_delivery_grant(&authority, &kind).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Authority(message))
        })?;
        validate_history(self, value).map_err(|message| {
            self.error("open-delivery", Some(&task), Rejection::Conflict(message))
        })?;
        let base = self.services.vcs.head("origin/main").map_err(|error| {
            self.error("open-delivery", Some(&task), Rejection::External(error.0))
        })?;
        let criteria = match &authority.grant {
            Grant::Delivery { criteria, .. } => criteria.clone(),
            _ => Vec::new(),
        };
        let paths = authority::paths(&authority.grant).map(<[String]>::to_vec);
        let delivery = Delivery {
            id: next_delivery(value),
            kind,
            authority: Some(authority.id),
            criteria,
            paths,
            base,
            work: None,
            proof: acceptance::Proof::default(),
            review: None,
            outcome: Outcome::Open,
            launches: Vec::new(),
            operations: Vec::new(),
        };
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
            Event::DeliveryOpened {
                task: task.clone(),
                delivery: Box::new(delivery),
            },
            Event::Resumed { task: task.clone() },
        ];
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
        authority::register(&value.authorities, &authority, true).map_err(|error| {
            self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Authority(format!("authority rejected: {error:?}")),
            )
        })?;
        validate_authority_file(&authority).map_err(|message| {
            self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Authority(message),
            )
        })?;
        if !matches!(&authority.grant, Grant::Narrowing { criteria: granted, .. } if granted == &criteria)
        {
            return Err(self.error(
                "narrow-acceptance",
                Some(&task),
                Rejection::Authority("narrowing grant does not match criteria".into()),
            ));
        }
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
