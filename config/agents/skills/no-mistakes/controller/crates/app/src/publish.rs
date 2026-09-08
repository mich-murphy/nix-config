use crate::delivery::PublishInput;
use crate::delivery_support::{
    append_acceptance_hold, current_snapshot, execute_publish, get_task, next_operation,
    publish_action, publish_ready,
};
use crate::{AgentError, App, Output, Rejection, ResultData};
use domain::{
    event::{Event, Observation, Operation, OperationStatus},
    ids::TaskId,
};

impl App<'_> {
    pub(super) fn publish(
        &mut self,
        task: TaskId,
        input: PublishInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("publish")?;
        let value = get_task(&state, &task)
            .map_err(|message| self.error("publish", Some(&task), Rejection::Invalid(message)))?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "publish",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        let snapshot = current_snapshot(value).ok_or_else(|| {
            self.error(
                "publish",
                Some(&task),
                Rejection::Evidence("publish requires a snapshot".into()),
            )
        })?;
        publish_ready(&state, value, delivery, &input)
            .map_err(|message| self.error("publish", Some(&task), Rejection::Evidence(message)))?;
        let operation = publish_operation(self, &state, &task, delivery, snapshot, &input)?;
        self.execute_publish_flow(value, delivery, snapshot, &input, operation, check)
    }

    fn execute_publish_flow(
        &mut self,
        task: &domain::task::Task,
        delivery: &domain::delivery::Delivery,
        snapshot: &domain::acceptance::Snapshot,
        input: &PublishInput,
        operation: Operation,
        check: bool,
    ) -> Result<Output, AgentError> {
        if check {
            return self.commit(
                "publish",
                Some(&task.id),
                vec![Event::OperationStarted { operation }],
                true,
            );
        }
        let id = operation.id;
        let mut records = self.write(
            "publish",
            Some(&task.id),
            vec![Event::OperationStarted { operation }],
            false,
        )?;
        let observed = execute_publish(self, delivery, snapshot, input).map_err(|message| {
            self.error("publish", Some(&task.id), Rejection::External(message))
        })?;
        let events = publish_events(self, task, delivery, id, observed)?;
        records.extend(self.write("publish", Some(&task.id), events, false)?);
        Ok(Output {
            events: records,
            result: ResultData::Applied,
        })
    }
}

fn publish_operation(
    app: &App<'_>,
    state: &domain::state::State,
    task: &TaskId,
    delivery: &domain::delivery::Delivery,
    snapshot: &domain::acceptance::Snapshot,
    input: &PublishInput,
) -> Result<Operation, AgentError> {
    let action = publish_action(delivery, snapshot, input)
        .map_err(|message| app.error("publish", Some(task), Rejection::Invalid(message)))?;
    Ok(Operation {
        id: next_operation(state),
        task: task.clone(),
        delivery: delivery.id,
        action,
        status: OperationStatus::Running,
        process: None,
        timeout_seconds: 300,
    })
}

fn publish_events(
    app: &App<'_>,
    task: &domain::task::Task,
    delivery: &domain::delivery::Delivery,
    operation: domain::ids::OperationId,
    observed: domain::delivery::PullRequest,
) -> Result<Vec<Event>, AgentError> {
    let mut events = vec![
        Event::OperationSettled {
            operation,
            status: OperationStatus::Confirmed,
            observation: Observation::PullRequest(observed.clone()),
        },
        Event::PrObserved {
            task: task.id.clone(),
            delivery: delivery.id,
            pr: observed.clone(),
        },
    ];
    if let Some(event) =
        crate::checks::merged_event(app, &task.id, delivery.id, &observed, "publish")?
    {
        events.push(event);
        append_acceptance_hold(app, task, delivery, &mut events);
    }
    Ok(events)
}
